use crate::{proxy, requests_screen::RequestsScreen, waiting_screen::WaitingScreen};
use async_trait::async_trait;
use crossterm::event::EventStream;
use ratatui::{DefaultTerminal, Frame};
use std::{net::SocketAddr, sync::Arc};
use tokio::sync::{mpsc, oneshot};
use tokio_stream::StreamExt;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ScreenId {
    Waiting,
    Requests,
}

#[async_trait]
pub trait Screen {
    fn id(&self) -> ScreenId;
    async fn handle_event(&mut self, event: &Event) -> Option<Action>;
    fn draw(&mut self, frame: &mut Frame);
}

pub struct Navigator {
    stack: Vec<Box<dyn Screen>>,
}

impl Navigator {
    pub fn new() -> Self {
        Self { stack: vec![] }
    }

    pub fn push(&mut self, screen: Box<dyn Screen>) {
        self.stack.push(screen);
    }

    pub fn pop(&mut self) {
        self.stack.pop();
    }

    pub fn pop_to(&mut self, screen: Box<dyn Screen>) {
        let screen_id = screen.id();
        if let Some(pos) = self.stack.iter().position(|s| s.id() == screen_id) {
            self.stack.truncate(pos + 1);
        } else {
            self.push(screen);
        }
    }

    pub fn clear(&mut self) {
        self.stack.clear();
    }

    pub fn current(&mut self) -> Option<&mut Box<dyn Screen>> {
        self.stack.last_mut()
    }
}

pub enum Event {
    CrosstermEvent(crossterm::event::Event),
    ProxyMessage(Arc<proxy::Message>),
}

#[allow(dead_code)]
pub enum NavigationPolicy {
    Push,
    Pop,
    PopTo,
    Clear,
}

#[allow(dead_code)]
pub enum Action {
    ShowScreen(ScreenId, NavigationPolicy),
    ForwardToScreen(ScreenId, Event, NavigationPolicy),
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
        }
    }

    pub async fn run(&mut self, terminal: &mut DefaultTerminal) -> anyhow::Result<()> {
        let mut events = EventStream::new();
        self.running = true;

        self.change_screen(ScreenId::Waiting, NavigationPolicy::Push);

        let server_addr: SocketAddr =
            format!("{}:{}", self.server_host, self.server_port).parse()?;
        let (abort_server_tx, abort_server_rx) = oneshot::channel();
        let (abort_app_tx, mut abort_app_rx) = oneshot::channel();
        let (message_tx, mut message_rx) = mpsc::channel(Self::MESSAGE_CHANNEL_BUFFER_SIZE);

        self.abort_server_tx = Some(abort_server_tx);

        let server = proxy::Server::new(server_addr, message_tx.clone());
        let server_handle = tokio::spawn(async move {
            let result = server.run(abort_server_rx).await;
            let _ = abort_app_tx.send(result);
        });

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
                        self.handle_action(action).await?;
                    }
                }
                Some(message) = message_rx.recv() => {
                    if let Some(action) = screen.handle_event(&Event::ProxyMessage(Arc::new(message))).await {
                        self.handle_action(action).await?;
                    }
                }
                result = &mut abort_app_rx => {
                    match result {
                        Ok(Err(error)) => {
                            self.exit();
                            return Err(anyhow::anyhow!(error.to_string()));
                        },
                        Err(_) =>  {
                            self.exit();
                            return Err(anyhow::anyhow!("Server was aborted unexpectedly"))
                        },
                        _ => {}
                    }
                }
            }
        }

        server_handle.await?;

        Ok(())
    }

    fn draw(&mut self, frame: &mut Frame) {
        if let Some(screen) = self.navigator.current() {
            screen.draw(frame);
        }
    }

    async fn handle_action(&mut self, action: Action) -> anyhow::Result<()> {
        let mut next_action = Some(action);
        while let Some(action) = next_action.take() {
            match action {
                Action::Exit => self.exit(),
                Action::ExitWithError(error) => {
                    self.exit();
                    return Err(error);
                }
                Action::GoBack => self.navigator.pop(),
                Action::ShowScreen(screen_id, policy) => self.change_screen(screen_id, policy),
                Action::ForwardToScreen(screen_id, event, policy) => {
                    self.change_screen(screen_id, policy);
                    if let Some(screen) = self.navigator.current() {
                        next_action = screen.handle_event(&event).await;
                    }
                }
            }
        }

        Ok(())
    }

    fn exit(&mut self) {
        self.running = false;
        if let Some(abort_channel) = self.abort_server_tx.take() {
            let _ = abort_channel.send(());
        }
    }

    fn change_screen(&mut self, screen_id: ScreenId, policy: NavigationPolicy) {
        let screen: Box<dyn Screen> = match screen_id {
            ScreenId::Waiting => Box::new(WaitingScreen::new(
                self.server_host.clone(),
                self.server_port,
            )),
            ScreenId::Requests => Box::new(RequestsScreen::new()),
        };

        match policy {
            NavigationPolicy::Push => self.navigator.push(screen),
            NavigationPolicy::Pop => self.navigator.pop(),
            NavigationPolicy::Clear => {
                self.navigator.clear();
                self.navigator.push(screen);
            }
            NavigationPolicy::PopTo => self.navigator.pop_to(screen),
        }
    }
}
