use crate::{
    clipboard::Clipboard,
    config::Config,
    modals::{ErrorModal, ErrorModalMessage, WaitingModal},
    screens::{
        HttpClientScreen, HttpClientScreenMessage, RequestsScreen, RequestsScreenMessage,
        RequestsScreenState,
    },
};
use crossterm::event::{Event, EventStream};
use proxy::{
    certs::{CertificateStore, generate_certificate, generate_key_pair},
    http::{Body, Request, Response},
    server::{Error as ServerError, Message as ProxyMessage, RequestId, Server as ProxyServer},
};
use ratatui::{DefaultTerminal, Frame};
use std::{
    collections::BTreeMap,
    net::SocketAddr,
    sync::{Arc, Mutex},
};
use tokio::{
    io::AsyncWriteExt,
    sync::{mpsc, oneshot},
};
use tokio_stream::StreamExt;

#[derive(Debug, Clone, PartialEq)]
pub enum Message {
    // Events
    RequestEntryUpdated(Box<RequestEntry>),
    // Global
    ShowRequestsScreen,
    ShowHttpClientScreen,
    StoreRequest(Box<(RequestId, Request)>),
    StoreResponse(Box<(RequestId, Response)>),
    StoreRequestsScreenState(Box<RequestsScreenState>),
    CopyToClipboard(String),
    Quit,
    // Modal-specific
    ShowErrorModal(String),
    CloseModal,
    ErrorModal(ErrorModalMessage),
    // Screen-specific
    RequestsScreen(RequestsScreenMessage),
    HttpClientScreen(HttpClientScreenMessage),
}

pub trait Screen {
    fn draw(&mut self, frame: &mut Frame) {
        let _ = frame;
    }

    fn handle_event(&self, event: Event) -> Option<Message> {
        let _ = event;
        None
    }

    fn update(&mut self, message: Message) -> Option<Message> {
        let _ = message;
        None
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct RequestEntry {
    pub request_id: RequestId,
    pub request: Request,
    pub response: Option<Response>,
}

pub type BoxedScreen = Box<dyn Screen>;
pub type RequestStore = Arc<Mutex<BTreeMap<RequestId, RequestEntry>>>;

pub struct App {
    abort_app_rx: Option<oneshot::Receiver<Result<(), ServerError>>>,
    abort_server_tx: Option<oneshot::Sender<()>>,
    message_rx: Option<mpsc::Receiver<ProxyMessage>>,
    config: Config,
    screen: BoxedScreen,
    modal: Option<BoxedScreen>,
    request_store: RequestStore,
    requests_screen_state: RequestsScreenState,
    clipboard: Clipboard,
    running: bool,
    exit_error: Option<anyhow::Error>,
}

impl App {
    const MESSAGE_CHANNEL_BUFFER_SIZE: usize = 128;

    pub fn new(config: Config) -> anyhow::Result<Self> {
        let modal = Box::new(WaitingModal::new(&config.server_host, config.server_port));
        let request_store = RequestStore::new(Mutex::new(BTreeMap::new()));
        let clipboard = Clipboard::new()?;

        let requests_screen_state = RequestsScreenState::default();
        let screen = Box::new(RequestsScreen::new(
            request_store.clone(),
            requests_screen_state.clone(),
        ));

        Ok(Self {
            abort_app_rx: None,
            abort_server_tx: None,
            message_rx: None,
            config,
            screen,
            modal: Some(modal),
            request_store,
            requests_screen_state,
            clipboard,
            running: true,
            exit_error: None,
        })
    }

    pub async fn run(&mut self, terminal: &mut DefaultTerminal) -> anyhow::Result<()> {
        let (abort_app_tx, abort_app_rx) = oneshot::channel();
        let (abort_server_tx, abort_server_rx) = oneshot::channel();
        let (message_tx, message_rx) = mpsc::channel(Self::MESSAGE_CHANNEL_BUFFER_SIZE);
        self.abort_app_rx = Some(abort_app_rx);
        self.abort_server_tx = Some(abort_server_tx);
        self.message_rx = Some(message_rx);

        let server_addr: SocketAddr =
            format!("{}:{}", self.config.server_host, self.config.server_port).parse()?;
        let certificate_store = self.load_certificate_store().await?;
        let server = ProxyServer::new(server_addr, certificate_store, message_tx);
        let server_handle = tokio::spawn(async move {
            let result = server.run(abort_server_rx).await;
            let _ = abort_app_tx.send(result);
        });

        let mut event_stream = EventStream::new();
        while self.running {
            terminal.draw(|frame: &mut Frame| self.draw(frame))?;

            let mut current_message = self.handle_event(&mut event_stream).await;
            while let Some(message) = current_message {
                current_message = self.update(message);
            }
        }

        server_handle.await?;

        self.exit_error.take().map_or(Ok(()), Err)
    }

    async fn load_certificate_store(&self) -> anyhow::Result<CertificateStore> {
        tokio::fs::create_dir_all(&self.config.certs_dir)
            .await
            .map_err(|e| {
                anyhow::anyhow!(
                    "Failed to create certificates directory '{}': {}",
                    self.config.certs_dir.display(),
                    e
                )
            })?;

        if !self.config.certs_dir.join("ca.crt").exists()
            || !self.config.certs_dir.join("ca.key").exists()
            || !self.config.certs_dir.join("ca.pem").exists()
        {
            let key_pair = generate_key_pair()
                .map_err(|e| anyhow::anyhow!("Failed to generate key pair: {}", e))?;
            let cert = generate_certificate(&key_pair)
                .map_err(|e| anyhow::anyhow!("Failed to generate certificate: {}", e))?;

            let certs_path = self.config.certs_dir.clone();
            let mut cert_file_der = tokio::fs::File::create(certs_path.join("ca.crt")).await?;
            cert_file_der.write_all(cert.der()).await?;
            let mut cert_file_pem = tokio::fs::File::create(certs_path.join("ca.pem")).await?;
            cert_file_pem.write_all(cert.pem().as_bytes()).await?;
            let mut keypair_file = tokio::fs::File::create(certs_path.join("ca.key")).await?;
            keypair_file.write_all(key_pair.serialized_der()).await?;
        }

        CertificateStore::from_files(
            &self.config.certs_dir.join("ca.crt"),
            &self.config.certs_dir.join("ca.key"),
        )
        .await
        .map_err(|e| anyhow::anyhow!("Failed to load certificate and key: {}", e))
    }

    fn draw(&mut self, frame: &mut Frame) {
        self.screen.draw(frame);
        if let Some(modal) = self.modal.as_mut() {
            modal.draw(frame);
        }
    }

    async fn handle_event(&mut self, event_stream: &mut EventStream) -> Option<Message> {
        let message_rx = match self.message_rx.as_mut() {
            Some(rx) => rx,
            None => return Some(Message::Quit),
        };
        let mut abort_app_rx = match self.abort_app_rx.as_mut() {
            Some(rx) => rx,
            None => return Some(Message::Quit),
        };

        tokio::select! {
            Some(event) = event_stream.next() => {
                let event = event.ok()?;
                self.handle_event_stream(event)
            }
            Some(message) = message_rx.recv() => {
                self.handle_proxy_message(message).await
            }
            result = &mut abort_app_rx => {
                self.handle_abort_app(result).await
            }
        }
    }

    fn handle_event_stream(&self, event: Event) -> Option<Message> {
        if let Some(modal) = self.modal.as_ref() {
            modal.handle_event(event)
        } else {
            self.screen.handle_event(event)
        }
    }

    async fn handle_proxy_message(&mut self, message: ProxyMessage) -> Option<Message> {
        match message {
            ProxyMessage::RequestSent((id, request)) => {
                return Some(Message::StoreRequest(Box::new((id, request))));
            }
            ProxyMessage::ResponseReceived((id, response)) => {
                return Some(Message::StoreResponse(Box::new((id, response))));
            }
            ProxyMessage::ErrorOccurred((request_id, error)) => {
                let text = if let Some(request_id) = request_id {
                    format!("Error occurred for request {}: {:#?}", request_id, error)
                } else {
                    format!("Error occurred: {:#?}", error)
                };
                return Some(Message::ShowErrorModal(text));
            }
            _ => {}
        }

        None
    }

    async fn handle_abort_app(
        &mut self,
        result: Result<Result<(), ServerError>, oneshot::error::RecvError>,
    ) -> Option<Message> {
        match result {
            Ok(Err(error)) => {
                self.exit_error = Some(anyhow::anyhow!(error.to_string()));
                return Some(Message::Quit);
            }
            Err(_) => {
                self.exit_error = Some(anyhow::anyhow!("Server was aborted unexpectedly"));
                return Some(Message::Quit);
            }
            _ => {}
        }

        None
    }

    fn update(&mut self, message: Message) -> Option<Message> {
        match message {
            Message::ShowRequestsScreen => self.show_requests_screen(),
            Message::ShowHttpClientScreen => self.show_http_client_screen(),
            Message::StoreRequest(message) => self.store_request(*message),
            Message::StoreResponse(message) => self.store_response(*message),
            Message::StoreRequestsScreenState(message) => {
                self.store_requests_screen_state(*message)
            }
            Message::CopyToClipboard(content) => self.copy_to_clipboard(content),
            Message::Quit => self.quit(),
            Message::ShowErrorModal(content) => self.show_error_modal(content),
            Message::CloseModal => self.close_modal(),
            message => self.update_modal_and_screen(message),
        }
    }

    fn make_requests_screen(&self) -> RequestsScreen {
        RequestsScreen::new(
            self.request_store.clone(),
            self.requests_screen_state.clone(),
        )
    }

    fn make_http_client_screen(&self, request_entry: RequestEntry) -> HttpClientScreen {
        HttpClientScreen::new(request_entry)
    }

    fn show_requests_screen(&mut self) -> Option<Message> {
        self.screen = Box::new(self.make_requests_screen());
        Some(RequestsScreenMessage::UpdateTableState.into())
    }

    fn show_http_client_screen(&mut self) -> Option<Message> {
        let selected = self.requests_screen_state.selected_request_entry?;

        let request_entry = match self.request_store.lock() {
            Ok(store) => store.values().nth(selected)?.clone(),
            Err(_) => return Some(Message::Quit),
        };

        let screen = self.make_http_client_screen(request_entry);
        self.screen = Box::new(screen);
        None
    }

    fn store_request(&mut self, message: (RequestId, Request)) -> Option<Message> {
        let (request_id, request) = message;
        match self.request_store.lock() {
            Ok(mut store) => {
                let formatted_body = formatter::format_code(&request.body.to_string());
                let body = Body::new(formatted_body.into_bytes());
                let request_entry = RequestEntry {
                    request_id,
                    request: Request { body, ..request },
                    response: None,
                };
                store.insert(request_id, request_entry.clone());

                Some(Message::RequestEntryUpdated(request_entry.into()))
            }
            Err(_) => Some(Message::Quit),
        }
    }

    fn store_response(&mut self, message: (RequestId, Response)) -> Option<Message> {
        let (request_id, response) = message;
        match self.request_store.lock() {
            Ok(mut store) => {
                if let Some(request_entry) = store.get_mut(&request_id) {
                    let formatted_body = formatter::format_code(&response.body.to_string());
                    let body = Body::new(formatted_body.into_bytes());
                    let response = Response { body, ..response };
                    request_entry.response = Some(response);

                    return Some(Message::RequestEntryUpdated(request_entry.clone().into()));
                }
                None
            }
            Err(_) => Some(Message::Quit),
        }
    }

    fn store_requests_screen_state(&mut self, state: RequestsScreenState) -> Option<Message> {
        self.requests_screen_state = state;
        None
    }

    fn copy_to_clipboard(&mut self, content: String) -> Option<Message> {
        match self.clipboard.set_text(content) {
            Ok(_) => None,
            Err(e) => Some(Message::ShowErrorModal(format!(
                "Failed to copy to clipboard: {}",
                e
            ))),
        }
    }

    fn quit(&mut self) -> Option<Message> {
        self.running = false;
        self.abort_server_tx.take().map(|tx| tx.send(()));
        None
    }

    fn show_error_modal(&mut self, content: String) -> Option<Message> {
        self.modal = Some(Box::new(ErrorModal::new(&content)));
        None
    }

    fn close_modal(&mut self) -> Option<Message> {
        self.modal = None;
        None
    }

    fn update_modal_and_screen(&mut self, message: Message) -> Option<Message> {
        if let Some(modal) = self.modal.as_mut() {
            let mut current_message = modal.update(message.clone());
            while let Some(message) = current_message {
                current_message = self.update(message);
            }
        }

        self.screen.update(message)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use insta::assert_snapshot;
    use proxy::http::{Body, HeaderMap, Method};
    use ratatui::{Terminal, backend::TestBackend, widgets::TableState};
    use rstest::{fixture, rstest};

    #[fixture]
    fn terminal() -> Terminal<TestBackend> {
        Terminal::new(TestBackend::new(80, 24)).unwrap()
    }

    #[fixture]
    fn config() -> Config {
        Config::new("localhost".to_string(), 8888).unwrap()
    }

    #[fixture]
    fn app(config: Config) -> App {
        App::new(config).unwrap()
    }

    #[rstest]
    fn show_waiting_message(mut terminal: Terminal<TestBackend>, mut app: App) {
        terminal.draw(|frame| app.draw(frame)).unwrap();

        assert_snapshot!(terminal.backend());
    }

    #[rstest]
    fn handle_store_request_message(mut app: App) {
        let request_id = RequestId::new(1);
        let request = Request {
            method: Method::GET,
            url: "http://example.com".into(),
            headers: HeaderMap::new(vec![]),
            body: Body::new(vec![]),
        };
        let store_message = Message::StoreRequest(Box::new((request_id, request.clone())));
        let expected_message = Message::RequestEntryUpdated(Box::new(RequestEntry {
            request_id,
            request,
            response: None,
        }));

        let message = app.update(store_message);

        assert_eq!(message, Some(expected_message));
    }

    #[rstest]
    fn handle_store_response_message(mut app: App) {
        let request_id = RequestId::new(1);
        let request = Request {
            method: Method::GET,
            url: "http://example.com".into(),
            headers: HeaderMap::new(vec![]),
            body: Body::new(vec![]),
        };
        app.store_request((request_id, request.clone()));
        let response = Response {
            status: 200.try_into().unwrap(),
            headers: HeaderMap::new(vec![]),
            body: Body::new(vec![]),
        };
        let store_message = Message::StoreResponse(Box::new((request_id, response.clone())));
        let expected_message = Message::RequestEntryUpdated(Box::new(RequestEntry {
            request_id,
            request,
            response: Some(response),
        }));

        let message = app.update(store_message);

        assert_eq!(message, Some(expected_message));
    }

    #[rstest]
    fn handle_quit_message(mut app: App) {
        let message = app.update(Message::Quit);

        assert_eq!(message, None);
        assert!(!app.running);
    }

    #[rstest]
    fn handle_non_app_message(mut app: App) {
        let non_app_message = Message::RequestsScreen(RequestsScreenMessage::SelectNextRow);

        let message = app.update(non_app_message);

        assert_eq!(
            message,
            Some(Message::StoreRequestsScreenState(
                RequestsScreenState {
                    selected_request_entry: Some(0),
                    table_state: TableState::default().with_selected(Some(0)),
                    ..Default::default()
                }
                .into()
            ))
        );
    }

    #[tokio::test]
    #[rstest]
    async fn handle_abort_app_with_proxy_error(mut app: App) {
        let error = Ok(Err(ServerError::AcceptConnection("TEST ERROR".into())));

        let message = app.handle_abort_app(error).await;

        assert_eq!(message, Some(Message::Quit));
        assert_eq!(
            app.exit_error.unwrap().to_string(),
            "Failed to accept connection: TEST ERROR".to_string()
        );
    }

    #[tokio::test]
    #[rstest]
    async fn handle_abort_app_with_recv_error(mut app: App) {
        let (tx, rx) = oneshot::channel::<Result<(), ServerError>>();
        drop(tx);
        let error = rx.await.unwrap_err();

        let message = app.handle_abort_app(Err(error)).await;

        assert_eq!(message, Some(Message::Quit));
        assert_eq!(
            app.exit_error.unwrap().to_string(),
            "Server was aborted unexpectedly".to_string()
        );
    }

    #[rstest]
    fn ignore_non_key_event(app: App) {
        let non_key_event = Event::Mouse(crossterm::event::MouseEvent {
            kind: crossterm::event::MouseEventKind::Down(crossterm::event::MouseButton::Left),
            column: 0,
            row: 0,
            modifiers: crossterm::event::KeyModifiers::NONE,
        });

        let message = app.screen.handle_event(non_key_event);

        assert_eq!(message, None);
    }
}
