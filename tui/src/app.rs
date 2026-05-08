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
use ratatui::{
    DefaultTerminal, Frame,
    layout::{Alignment, Constraint},
    text::Text,
    widgets::{Cell, Row},
};
use std::{
    collections::BTreeMap,
    net::SocketAddr,
    ops::Deref,
    sync::{Arc, Mutex},
};
use tokio::{
    io::AsyncWriteExt,
    sync::{mpsc, oneshot},
};
use tokio_stream::StreamExt;

#[derive(Debug, PartialEq)]
pub enum Message {
    ShowRequestsScreen,
    ShowHttpClientScreen(Box<RequestEntry>),
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

#[derive(Debug, Clone, Default, PartialEq)]
pub struct RequestEntryRow {
    pub request_id: String,
    pub method: String,
    pub url: String,
    pub body: String,
    pub status: String,
}

impl From<&RequestEntry> for RequestEntryRow {
    fn from(entry: &RequestEntry) -> Self {
        let status = if let Some(response) = &entry.response {
            response.status.as_u16().to_string()
        } else {
            "Pending".to_string()
        };
        let body = {
            let request_body_len = entry.request.body.as_bytes().len();
            let response_body_len = entry
                .response
                .as_ref()
                .map_or(0, |response| response.body.as_bytes().len());
            format!(
                "Sent: {} bytes | Received: {} bytes",
                request_body_len, response_body_len
            )
        };

        RequestEntryRow {
            request_id: entry.request_id.to_string(),
            method: entry.request.method.to_string(),
            url: entry.request.url.to_string(),
            body,
            status,
        }
    }
}

impl From<RequestEntryRow> for Row<'_> {
    fn from(row: RequestEntryRow) -> Self {
        Row::new(vec![
            Cell::from(Text::from(row.request_id).alignment(Alignment::Right)),
            Cell::from(row.method),
            Cell::from(row.url),
            Cell::from(row.body),
            Cell::from(row.status),
        ])
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct RequestsTableColumnWidths {
    request_id: u16,
    method: u16,
    url: u16,
    body: u16,
    status: u16,
}

impl RequestsTableColumnWidths {
    pub const TABLE_COLUMN_REQ_ID: &str = "ReqId";
    pub const TABLE_COLUMN_METHOD: &str = "Method";
    pub const TABLE_COLUMN_URL: &str = "URL";
    pub const TABLE_COLUMN_BODY: &str = "Body";
    pub const TABLE_COLUMN_STATUS: &str = "Status";

    pub fn update(&mut self, row: RequestEntryRow) {
        self.request_id = self.request_id.max(row.request_id.len() as u16);
        self.method = self.method.max(row.method.len() as u16);
        self.url = self.url.max(row.url.len() as u16);
        self.body = self.body.max(row.body.len() as u16);
        self.status = self.status.max(row.status.len() as u16);
    }

    pub fn to_table_widths(&self) -> [Constraint; 5] {
        [
            Constraint::Length(self.request_id),
            Constraint::Length(self.method),
            Constraint::Length(self.url),
            Constraint::Min(self.body),
            Constraint::Length(self.status),
        ]
    }
}

impl Default for RequestsTableColumnWidths {
    fn default() -> Self {
        let mut widths = Self {
            request_id: 0,
            method: 0,
            url: 0,
            body: 0,
            status: 0,
        };
        widths.update(RequestEntryRow {
            request_id: Self::TABLE_COLUMN_REQ_ID.to_string(),
            method: Self::TABLE_COLUMN_METHOD.to_string(),
            url: Self::TABLE_COLUMN_URL.to_string(),
            body: Self::TABLE_COLUMN_BODY.to_string(),
            status: Self::TABLE_COLUMN_STATUS.to_string(),
        });
        widths
    }
}

pub type RequestStore = Arc<Mutex<BTreeMap<RequestId, RequestEntry>>>;
pub type Clipboard = Arc<Mutex<crate::clipboard::Clipboard>>;
pub type SharedRequestsTableColumnWidths = Arc<Mutex<RequestsTableColumnWidths>>;

pub struct App {
    abort_app_rx: Option<oneshot::Receiver<Result<(), ServerError>>>,
    abort_server_tx: Option<oneshot::Sender<()>>,
    message_rx: Option<mpsc::Receiver<ProxyMessage>>,
    config: Config,
    screen: Box<dyn Screen>,
    request_store: RequestStore,
    clipboard: Clipboard,
    waiting_messages: bool,
    requests_table_column_widths: SharedRequestsTableColumnWidths,
    running: bool,
    exit_error: Option<anyhow::Error>,
}

impl App {
    const MESSAGE_CHANNEL_BUFFER_SIZE: usize = 128;

    pub fn new(config: Config) -> anyhow::Result<Self> {
        let screen = Box::new(WaitingScreen::new(
            config.server_host.clone(),
            config.server_port,
        ));
        let request_store = RequestStore::new(Mutex::new(BTreeMap::new()));
        let system_clipboard = crate::clipboard::Clipboard::new()?;
        let clipboard = Clipboard::new(Mutex::new(system_clipboard));
        let requests_table_column_widths =
            SharedRequestsTableColumnWidths::new(Mutex::new(RequestsTableColumnWidths::default()));

        Ok(Self {
            abort_app_rx: None,
            abort_server_tx: None,
            message_rx: None,
            config,
            screen,
            request_store,
            clipboard,
            waiting_messages: true,
            requests_table_column_widths,
            running: true,
            exit_error: None,
        })
    }

    fn make_requests_screen(&self) -> RequestsScreen {
        RequestsScreen::new(
            self.request_store.clone(),
            self.requests_table_column_widths.clone(),
        )
    }

    fn make_http_client_screen(&self, request_entry: RequestEntry) -> HttpClientScreen {
        HttpClientScreen::new(self.clipboard.clone(), request_entry)
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
            Message::ShowRequestsScreen => self.show_requests_screen(),
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

    fn show_requests_screen(&mut self) -> Option<Message> {
        self.screen = Box::new(self.make_requests_screen());
        self.screen.update(Message::RequestsScreen(
            RequestsScreenMessage::UpdateTableState,
        ))
    }

    fn show_http_client_screen(&mut self, request_entry: RequestEntry) -> Option<Message> {
        let screen = self.make_http_client_screen(request_entry);
        self.screen = Box::new(screen);
        None
    }

    fn store_request(&mut self, request_id: RequestId, request: Request) -> Option<Message> {
        if self.waiting_messages {
            self.screen = Box::new(self.make_requests_screen());
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

                if let Ok(mut widths) = self.requests_table_column_widths.lock() {
                    widths.update((&request_entry).into());
                }

                Some(RequestsScreenMessage::UpdateTableState.into())
            }
            Err(_) => Some(Message::Quit),
        }
    }

    fn store_response(&mut self, request_id: RequestId, response: Response) -> Option<Message> {
        match self.request_store.lock() {
            Ok(mut store) => {
                if let Some(request_entry) = store.get_mut(&request_id) {
                    request_entry.response = Some(response);

                    if let Ok(mut widths) = self.requests_table_column_widths.lock() {
                        widths.update(request_entry.deref().into());
                    }

                    return Some(RequestsScreenMessage::UpdateTableState.into());
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
        let expected_message = Message::RequestsScreen(RequestsScreenMessage::UpdateTableState);

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
            status: 200.try_into().unwrap(),
            headers: HeaderMap::new(vec![]),
            body: Body::new(vec![]),
        };
        let store_message = Message::StoreResponse(Box::new((request_id, response.clone())));
        let expected_message = Message::RequestsScreen(RequestsScreenMessage::UpdateTableState);

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
