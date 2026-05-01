use crate::{
    requests_screen::{Message as RequestsScreenMessage, RequestsScreen},
    waiting_screen::WaitingScreen,
};
use crossterm::event::{Event, EventStream};
use ratatui::{DefaultTerminal, Frame};
use std::{
    collections::BTreeMap,
    net::SocketAddr,
    ops::Deref,
    sync::{Arc, Mutex},
};
use tokio::sync::{mpsc, oneshot};
use tokio_stream::StreamExt;

pub enum Message {
    StoreRequest(Box<(proxy::RequestId, proxy::Request)>),
    StoreResponse(Box<(proxy::RequestId, proxy::Response)>),
    Quit,
    RequestsScreen(RequestsScreenMessage),
}

pub trait Screen {
    fn draw(&mut self, frame: &mut Frame);
    fn handle_event(&mut self, event: Event) -> Option<Message>;
    fn update(&mut self, message: Message) -> Option<Message>;
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct RequestEntry {
    pub request_id: proxy::RequestId,
    pub request: proxy::Request,
    pub response: Option<proxy::Response>,
}

pub type RequestStore = Arc<Mutex<BTreeMap<proxy::RequestId, RequestEntry>>>;

pub struct App {
    abort_app_rx: Option<oneshot::Receiver<Result<(), proxy::Error>>>,
    abort_server_tx: Option<oneshot::Sender<()>>,
    message_rx: Option<mpsc::Receiver<proxy::Message>>,
    server_host: String,
    server_port: u16,
    screen: Box<dyn Screen>,
    running: bool,
    exit_error: Option<anyhow::Error>,
    waiting_messages: bool,
    request_store: RequestStore,
}

impl App {
    const MESSAGE_CHANNEL_BUFFER_SIZE: usize = 128;

    pub fn new(server_host: String, server_port: u16) -> Self {
        let screen = Box::new(WaitingScreen::new(server_host.clone(), server_port));
        Self {
            abort_app_rx: None,
            abort_server_tx: None,
            message_rx: None,
            server_host,
            server_port,
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
            format!("{}:{}", self.server_host, self.server_port).parse()?;
        let server = proxy::Server::new(server_addr, message_tx);
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

        if let Some(error) = self.exit_error.take() {
            Err(error)
        } else {
            Ok(())
        }
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

    async fn handle_proxy_message(&mut self, message: proxy::Message) -> Option<Message> {
        match message {
            proxy::Message::RequestSent((id, request)) => {
                return Some(Message::StoreRequest(Box::new((id, request))));
            }
            proxy::Message::ResponseReceived((id, response)) => {
                return Some(Message::StoreResponse(Box::new((id, response))));
            }
            _ => {}
        }

        None
    }

    async fn handle_abort_app(
        &mut self,
        result: Result<Result<(), proxy::Error>, oneshot::error::RecvError>,
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

    fn make_requests_screen(&mut self) -> Box<dyn Screen> {
        Box::new(RequestsScreen::new(self.request_store.clone()))
    }

    fn quit(&mut self) -> Option<Message> {
        self.running = false;
        self.abort_server_tx.take().map(|tx| tx.send(()));
        None
    }

    fn store_request(
        &mut self,
        request_id: proxy::RequestId,
        request: proxy::Request,
    ) -> Option<Message> {
        if self.waiting_messages {
            self.screen = self.make_requests_screen();
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
                    RequestsScreenMessage::UpdateTableColumnWidths(Box::new(
                        (&request_entry).into(),
                    ))
                    .into(),
                )
            }
            Err(_) => Some(Message::Quit),
        }
    }

    fn store_response(
        &mut self,
        request_id: proxy::RequestId,
        response: proxy::Response,
    ) -> Option<Message> {
        if self.waiting_messages {
            self.screen = self.make_requests_screen();
            self.waiting_messages = false;
        }

        match self.request_store.lock() {
            Ok(mut store) => {
                if let Some(request_entry) = store.get_mut(&request_id) {
                    request_entry.response = Some(response);

                    return Some(
                        RequestsScreenMessage::UpdateTableColumnWidths(Box::new(
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
}

pub fn is_quit_key_event(event: &Event) -> Option<Message> {
    if let Event::Key(key_event) = event
        && key_event.code == crossterm::event::KeyCode::Char('q')
    {
        return Some(Message::Quit);
    }

    None
}
