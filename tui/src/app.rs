use crate::{
    config::Config,
    http_client_screen::{HttpClientScreen, Message as HttpClientScreenMessage},
    requests_screen::{Message as RequestsScreenMessage, RequestsScreen},
    waiting_screen::WaitingScreen,
};
use crossterm::event::{Event, EventStream};
use proxy::{
    certs::{CertificateStore, generate_certificate, generate_key_pair},
    http::{Request, Response},
    server::{Error as ServerError, Message as ProxyMessage, RequestId, Server as ProxyServer},
};
use ratatui::{DefaultTerminal, Frame};
use std::{
    collections::BTreeMap,
    io::Write,
    net::SocketAddr,
    ops::Deref,
    sync::{Arc, Mutex},
};
use tokio::sync::{mpsc, oneshot};
use tokio_stream::StreamExt;

#[derive(Debug, PartialEq)]
pub enum Message {
    ShowHttpClientScreen(Box<Option<RequestEntry>>),
    StoreRequest(Box<(RequestId, Request)>),
    StoreResponse(Box<(RequestId, Response)>),
    Quit,
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

pub type RequestStore = Arc<Mutex<BTreeMap<RequestId, RequestEntry>>>;

pub struct App {
    abort_app_rx: Option<oneshot::Receiver<Result<(), ServerError>>>,
    abort_server_tx: Option<oneshot::Sender<()>>,
    message_rx: Option<mpsc::Receiver<ProxyMessage>>,
    config: Config,
    screen: Box<dyn Screen>,
    running: bool,
    exit_error: Option<anyhow::Error>,
    waiting_messages: bool,
    request_store: RequestStore,
}

impl App {
    const MESSAGE_CHANNEL_BUFFER_SIZE: usize = 128;

    pub fn new(config: Config) -> Self {
        let screen = Box::new(WaitingScreen::new(
            config.server_host.clone(),
            config.server_port,
        ));
        Self {
            abort_app_rx: None,
            abort_server_tx: None,
            message_rx: None,
            config,
            screen,
            running: true,
            exit_error: None,
            waiting_messages: true,
            request_store: RequestStore::new(Mutex::new(BTreeMap::new())),
        }
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
        let certificate_store = self.load_certificate_store()?;
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

    fn load_certificate_store(&self) -> anyhow::Result<CertificateStore> {
        std::fs::create_dir_all(&self.config.certs_dir).map_err(|e| {
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
            let mut cert_file_der = std::fs::File::create(certs_path.join("ca.crt"))?;
            cert_file_der.write_all(cert.der())?;
            let mut cert_file_pem = std::fs::File::create(certs_path.join("ca.pem"))?;
            cert_file_pem.write_all(cert.pem().as_bytes())?;
            let mut keypair_file = std::fs::File::create(certs_path.join("ca.key"))?;
            keypair_file.write_all(key_pair.serialized_der())?;
        }

        CertificateStore::from_files(
            &self.config.certs_dir.join("ca.crt"),
            &self.config.certs_dir.join("ca.key"),
        )
        .map_err(|e| anyhow::anyhow!("Failed to load certificate and key: {}", e))
    }

    fn draw(&mut self, frame: &mut Frame) {
        self.screen.draw(frame);
    }

    async fn handle_event(&mut self, event_stream: &mut EventStream) -> Option<Message> {
        let message_rx = match &mut self.message_rx {
            Some(rx) => rx,
            None => return Some(Message::Quit),
        };
        let mut abort_app_rx = match &mut self.abort_app_rx {
            Some(rx) => rx,
            None => return Some(Message::Quit),
        };

        tokio::select! {
            Some(event) = event_stream.next() => {
                let event = event.ok()?;
                self.screen.handle_event(event)
            }
            Some(message) = message_rx.recv() => {
                self.handle_proxy_message(message).await
            }
            result = &mut abort_app_rx => {
                self.handle_abort_app(result).await
            }
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
                // TODO: We should probably display the error in the UI
                // instead of just quitting the app
                self.exit_error = Some(anyhow::anyhow!(
                    "Error occurred for request {:?}: {:?}",
                    request_id,
                    error
                ));
                return Some(Message::Quit);
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
            Message::ShowHttpClientScreen(request_entry) => {
                self.show_http_client_screen(*request_entry)
            }
            Message::StoreRequest(message) => {
                let (request_id, request) = *message;
                self.store_request(request_id, request)
            }
            Message::StoreResponse(message) => {
                let (request_id, response) = *message;
                self.store_response(request_id, response)
            }
            Message::Quit => self.quit(),
            message => self.screen.update(message),
        }
    }

    fn show_http_client_screen(&mut self, request_entry: Option<RequestEntry>) -> Option<Message> {
        self.screen = Box::new(HttpClientScreen::new(request_entry));
        None
    }

    fn store_request(&mut self, request_id: RequestId, request: Request) -> Option<Message> {
        if self.waiting_messages {
            self.screen = Box::new(RequestsScreen::new(self.request_store.clone()));
            self.waiting_messages = false;
        }

        match self.request_store.lock() {
            Ok(mut store) => {
                let request_entry = RequestEntry {
                    request_id,
                    request,
                    response: None,
                };
                store.insert(request_id, request_entry.clone());

                Some(
                    RequestsScreenMessage::UpdateTableState(Box::new((&request_entry).into()))
                        .into(),
                )
            }
            Err(_) => Some(Message::Quit),
        }
    }

    fn store_response(&mut self, request_id: RequestId, response: Response) -> Option<Message> {
        match self.request_store.lock() {
            Ok(mut store) => {
                if let Some(request_entry) = store.get_mut(&request_id) {
                    request_entry.response = Some(response);

                    return Some(
                        RequestsScreenMessage::UpdateTableState(Box::new(
                            request_entry.deref().into(),
                        ))
                        .into(),
                    );
                }
                None
            }
            Err(_) => Some(Message::Quit),
        }
    }

    fn quit(&mut self) -> Option<Message> {
        self.running = false;
        self.abort_server_tx.take().map(|tx| tx.send(()));
        None
    }
}

pub fn is_quit_key_event(event: &Event) -> Option<Message> {
    if let Event::Key(key_event) = event
        && key_event.code == crossterm::event::KeyCode::Char('q')
    {
        return Some(Message::Quit);
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use insta::assert_snapshot;
    use proxy::http::{Body, HeaderMap, Method};
    use ratatui::{Terminal, backend::TestBackend};
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
        App::new(config)
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
        let expected_message =
            Message::RequestsScreen(RequestsScreenMessage::UpdateTableState(Box::new(
                (&RequestEntry {
                    request_id,
                    request: request.clone(),
                    response: None,
                })
                    .into(),
            )));

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
        app.store_request(request_id, request.clone());
        let response = Response {
            status: 200,
            headers: HeaderMap::new(vec![]),
            body: Body::new(vec![]),
        };
        let store_message = Message::StoreResponse(Box::new((request_id, response.clone())));
        let expected_message =
            Message::RequestsScreen(RequestsScreenMessage::UpdateTableState(Box::new(
                (&RequestEntry {
                    request_id,
                    request,
                    response: Some(response),
                })
                    .into(),
            )));

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

        assert_eq!(message, None);
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
    fn quit_on_q_key(app: App) {
        let quit_event = Event::Key(crossterm::event::KeyEvent::new(
            crossterm::event::KeyCode::Char('q'),
            crossterm::event::KeyModifiers::NONE,
        ));

        let message = app.screen.handle_event(quit_event);

        assert_eq!(message, Some(Message::Quit));
    }

    #[rstest]
    fn ignore_non_q_key(app: App) {
        let non_quit_event = Event::Key(crossterm::event::KeyEvent::new(
            crossterm::event::KeyCode::Char('a'),
            crossterm::event::KeyModifiers::NONE,
        ));

        let message = app.screen.handle_event(non_quit_event);

        assert_eq!(message, None);
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
