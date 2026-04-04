use crate::{
    navigation::{Navigator, Screen, ScreenId},
    requests_screen::RequestsScreen,
    waiting_screen::WaitingScreen,
};
use crossterm::event::EventStream;
use ratatui::{DefaultTerminal, Frame};
use std::{net::SocketAddr, sync::Arc};
use tokio::sync::{mpsc, oneshot};
use tokio_stream::StreamExt;

pub enum Event {
    CrosstermEvent(crossterm::event::Event),
    ProxyMessage(Arc<charlotte::Message>),
}

#[allow(dead_code)]
pub enum Action {
    ShowScreen(ScreenId),
    FowardToScreen(ScreenId, Event),
    GoBack,
    Exit,
    ExitWithError(anyhow::Error),
}

pub struct App {
    navigator: Navigator,
    abort_server_tx: Option<oneshot::Sender<()>>,
    server_host: String,
    server_port: u16,
    running: bool,
    exit_error: Option<anyhow::Error>,
}

impl App {
    const MESSAGE_CHANNEL_BUFFER_SIZE: usize = 128;

    pub fn new(server_host: String, server_port: u16) -> Self {
        Self {
            navigator: Navigator::new(),
            abort_server_tx: None,
            server_host,
            server_port,
            running: false,
            exit_error: None,
        }
    }

    pub async fn run(&mut self, terminal: &mut DefaultTerminal) -> anyhow::Result<()> {
        let mut events = EventStream::new();

        self.show_screen(ScreenId::Waiting);

        let server_addr: SocketAddr =
            format!("{}:{}", self.server_host, self.server_port).parse()?;
        let (abort_server_tx, abort_server_rx) = oneshot::channel();
        let (abort_app_tx, mut abort_app_rx) = oneshot::channel();
        let (message_tx, mut message_rx) = mpsc::channel(Self::MESSAGE_CHANNEL_BUFFER_SIZE);

        self.abort_server_tx = Some(abort_server_tx);

        let server_handle = tokio::spawn(async move {
            let result = charlotte::serve(server_addr, message_tx, abort_server_rx).await;
            let _ = abort_app_tx.send(result);
        });

        self.running = true;
        while self.running {
            terminal.draw(|frame: &mut Frame| self.draw(frame))?;

            let screen = match self.navigator.current() {
                Some(screen) => screen,
                None => {
                    self.exit();
                    continue;
                }
            };

            tokio::select! {
                Some(Ok(event)) = events.next() => {
                    if let Some(action) = screen.handle_event(&Event::CrosstermEvent(event)).await {
                        self.handle_action(action).await;
                    }
                }
                Some(message) = message_rx.recv() => {
                    if let Some(action) = screen.handle_event(&Event::ProxyMessage(Arc::new(message))).await {
                        self.handle_action(action).await;
                    }
                }
                result = &mut abort_app_rx => {
                    match result {
                        Ok(Err(error)) => self.exit_with_error(anyhow::anyhow!(error.to_string())),
                        Err(_) =>  self.exit_with_error(anyhow::anyhow!("Server was aborted unexpectedly")),
                        _ => {}
                    }
                }
            }
        }

        server_handle.await?;

        if let Some(error) = self.exit_error.take() {
            return Err(error);
        }

        Ok(())
    }

    fn draw(&mut self, frame: &mut Frame) {
        let screen = match self.navigator.current() {
            Some(screen) => screen,
            None => return,
        };

        screen.draw(frame);
    }

    async fn handle_action(&mut self, action: Action) {
        let mut next_action = Some(action);
        while let Some(action) = next_action.take() {
            match action {
                Action::Exit => self.exit(),
                Action::ExitWithError(error) => self.exit_with_error(error),
                Action::GoBack => self.go_back(),
                Action::ShowScreen(screen_id) => self.show_screen(screen_id),
                Action::FowardToScreen(screen_id, event) => {
                    self.show_screen(screen_id);
                    if let Some(screen) = self.navigator.current() {
                        next_action = screen.handle_event(&event).await;
                    }
                }
            }
        }
    }

    fn exit(&mut self) {
        self.running = false;
        if let Some(abort_channel) = self.abort_server_tx.take() {
            let _ = abort_channel.send(());
        }
    }

    fn exit_with_error(&mut self, error: anyhow::Error) {
        self.exit();
        self.exit_error = Some(error);
    }

    fn show_screen(&mut self, screen_id: ScreenId) {
        let screen: Box<dyn Screen> = match screen_id {
            ScreenId::Waiting => Box::new(WaitingScreen::new(
                self.server_host.clone(),
                self.server_port,
            )),
            ScreenId::Requests => Box::new(RequestsScreen::new()),
        };
        self.navigator.show_screen(screen);
    }

    fn go_back(&mut self) {
        self.navigator.go_back();
    }
}
