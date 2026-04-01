use crate::{requests_screen::RequestsScreen, waiting_screen::WaitingScreen};
use async_trait::async_trait;
use crossterm::event::{Event, EventStream};
use ratatui::{DefaultTerminal, Frame};
use tokio_stream::StreamExt;

#[derive(Debug, Clone, PartialEq)]
pub enum ControlFlow {
    Navigate(ScreenRoute),
    Continue,
    Break,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ScreenRoute {
    Waiting,
    Requests,
}

#[async_trait]
pub trait Screen {
    async fn handle_events(&mut self, event: &Event) -> ControlFlow;
    fn draw(&self, frame: &mut Frame);
}

pub struct App {
    running: bool,
    current_screen: Box<dyn Screen>,
}

impl App {
    pub fn new() -> Self {
        Self {
            running: false,
            current_screen: Box::new(WaitingScreen::new()),
        }
    }

    pub async fn run(&mut self, terminal: &mut DefaultTerminal) -> anyhow::Result<()> {
        self.running = true;
        let mut events = EventStream::new();

        while self.running {
            terminal.draw(|frame: &mut Frame| self.draw(frame))?;

            loop {
                tokio::select! {
                    Some(Ok(event)) = events.next() => {
                        match self.current_screen.handle_events(&event).await {
                            ControlFlow::Navigate(route) => self.navigate_to(route),
                            ControlFlow::Continue => {}
                            ControlFlow::Break => {
                                self.quit_app();
                                break;
                            }
                        }
                    }
                    else => {
                        break;
                    }
                }
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
