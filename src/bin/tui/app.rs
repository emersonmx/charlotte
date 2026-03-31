use crate::{requests_screen::RequestsScreen, waiting_screen::WaitingScreen};
use async_trait::async_trait;
use crossterm::event::{Event, EventStream};
use ratatui::{DefaultTerminal, Frame};
use tokio::sync::mpsc::{Receiver, Sender};
use tokio_stream::StreamExt;

#[derive(Debug, Clone, PartialEq)]
pub enum ScreenRoute {
    Waiting,
    Requests,
}

#[async_trait]
pub trait Screen {
    async fn handle_events(&mut self, event: &Event, sender: &Sender<Action>);
    fn draw(&self, frame: &mut Frame);
}

pub enum Action {
    Quit,
    Navigate(ScreenRoute),
}

pub struct App {
    running: bool,
    action_sender: Sender<Action>,
    action_receiver: Receiver<Action>,
    current_screen: Box<dyn Screen>,
}

impl App {
    const APP_ACTION_CHANNEL_CAPACITY: usize = 100;

    pub fn new() -> Self {
        let (action_sender, action_receiver) =
            tokio::sync::mpsc::channel(Self::APP_ACTION_CHANNEL_CAPACITY);
        Self {
            running: false,
            action_sender,
            action_receiver,
            current_screen: Box::new(WaitingScreen::new()),
        }
    }

    pub async fn run(&mut self, terminal: &mut DefaultTerminal) -> anyhow::Result<()> {
        self.running = true;
        let mut events = EventStream::new();

        while self.running {
            terminal.draw(|frame: &mut Frame| self.draw(frame))?;

            tokio::select! {
                Some(Ok(event)) = events.next() => self.current_screen
                    .handle_events(&event, &self.action_sender)
                    .await,
                Some(action) = self.action_receiver.recv() => match action {
                    Action::Quit => self.quit_app(),
                    Action::Navigate(route) => self.navigate_to(route),
                },
            }
        }

        Ok(())
    }

    fn draw(&self, frame: &mut Frame) {
        self.current_screen.draw(frame);
    }

    fn quit_app(&mut self) {
        self.running = false;
    }

    fn navigate_to(&mut self, route: ScreenRoute) {
        self.current_screen = match route {
            ScreenRoute::Waiting => Box::new(WaitingScreen::new()),
            ScreenRoute::Requests => Box::new(RequestsScreen::new()),
        };
    }
}
