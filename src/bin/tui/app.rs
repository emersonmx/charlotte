use crate::{
    navigation::{Navigator, Screen, ScreenId},
    requests_screen::RequestsScreen,
    waiting_screen::WaitingScreen,
};
use crossterm::event::EventStream;
use ratatui::{DefaultTerminal, Frame};
use std::net::SocketAddr;
use tokio::sync::{mpsc, oneshot};
use tokio_stream::StreamExt;

pub enum Event {
    CrosstermEvent(crossterm::event::Event),
    ProxyMessage(Box<charlotte::Message>),
}

#[derive(Debug, Clone, PartialEq)]
pub enum Action {
    ShowScreen(ScreenId),
    GoBack,
    Exit,
}

pub struct App {
    navigator: Navigator,
    abort_server_tx: Option<oneshot::Sender<()>>,
    running: bool,
    exit_error: Option<anyhow::Error>,
}

impl App {
    const SERVER_HOST: &str = "0.0.0.0";
    const SERVER_PORT: u16 = 8888;
    const MESSAGE_CHANNEL_BUFFER_SIZE: usize = 128;

    pub fn new() -> Self {
        let mut navigator = Navigator::new();
        navigator.show_screen(Box::new(WaitingScreen::new()));

        Self {
            navigator,
            abort_server_tx: None,
            running: false,
            exit_error: None,
        }
    }

    pub async fn run(&mut self, terminal: &mut DefaultTerminal) -> anyhow::Result<()> {
        let mut events = EventStream::new();

        let server_addr: SocketAddr =
            format!("{}:{}", Self::SERVER_HOST, Self::SERVER_PORT).parse()?;
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
            if !self.navigator.has_screen() {
                self.exit();
                continue;
            }

            terminal.draw(|frame: &mut Frame| self.draw(frame))?;

            while self.navigator.has_screen() {
                let screen = match self.navigator.current_mut() {
                    Some(screen) => screen,
                    None => break,
                };

                tokio::select! {
                    Some(Ok(event)) = events.next() => {
                        if let Some(action) = screen.handle_event(&Event::CrosstermEvent(event)).await {
                            self.handle_action(action).await;
                            break;
                        }
                    }
                    Some(message) = message_rx.recv() => {
                        if let Some(action) = screen.handle_event(&Event::ProxyMessage(Box::new(message))).await {
                            self.handle_action(action).await;
                            break;
                        }
                    }
                    result = &mut abort_app_rx => {
                        match result {
                            Ok(Err(error)) => self.exit_with_error(anyhow::anyhow!(error.to_string())),
                            Err(_) =>  self.exit_with_error(anyhow::anyhow!("Server was aborted unexpectedly")),
                            _ => {}
                        }
                        break;
                    }
                    else => {
                        break;
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

    fn draw(&self, frame: &mut Frame) {
        let screen = match self.navigator.current() {
            Some(screen) => screen,
            None => return,
        };

        screen.draw(frame);
    }

    async fn handle_action(&mut self, action: Action) {
        match action {
            Action::Exit => self.exit(),
            Action::ShowScreen(screen_id) => self.show_screen(screen_id),
            Action::GoBack => self.go_back(),
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
            ScreenId::Waiting => Box::new(WaitingScreen::new()),
            ScreenId::Requests => Box::new(RequestsScreen::new()),
        };
        self.navigator.show_screen(screen);
    }

    fn go_back(&mut self) {
        self.navigator.go_back();
    }
}
