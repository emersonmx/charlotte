use crate::{requests_screen::RequestsScreen, waiting_screen::WaitingScreen};
use async_trait::async_trait;
use crossterm::event::{Event, EventStream};
use ratatui::{DefaultTerminal, Frame};
use std::collections::HashMap;
use tokio_stream::StreamExt;

#[derive(Debug, Clone, PartialEq)]
pub enum ControlFlow {
    Navigate(ScreenRoute),
    Continue,
    Quit,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
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
    current_screen_route: ScreenRoute,
    screen_router: HashMap<ScreenRoute, Box<dyn Screen>>,
}

impl App {
    pub fn new() -> Self {
        let mut router: HashMap<ScreenRoute, Box<dyn Screen>> = HashMap::new();
        router.insert(ScreenRoute::Waiting, Box::new(WaitingScreen::new()));
        router.insert(ScreenRoute::Requests, Box::new(RequestsScreen::new()));

        Self {
            running: false,
            current_screen_route: ScreenRoute::Waiting,
            screen_router: router,
        }
    }

    pub async fn run(&mut self, terminal: &mut DefaultTerminal) -> anyhow::Result<()> {
        self.running = true;
        let mut events = EventStream::new();

        while self.running {
            if !self.has_current_screen_route() {
                return Err(anyhow::anyhow!(format!(
                    "Screen `{:?}` not found in screen router",
                    self.current_screen_route
                )));
            }

            terminal.draw(|frame: &mut Frame| self.draw(frame))?;

            while self.has_current_screen_route() {
                tokio::select! {
                    Some(Ok(event)) = events.next() => {
                        let current_screen = match self.screen_router.get_mut(&self.current_screen_route) {
                            Some(screen) => screen,
                            None => break,
                        };
                        match current_screen.handle_events(&event).await {
                            ControlFlow::Navigate(route) => {
                                self.navigate_to(route);
                                break;
                            },
                            ControlFlow::Quit => {
                                self.quit_app();
                                break;
                            },
                            _ => {}
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
        let current_screen = self.screen_router.get(&self.current_screen_route);
        if let Some(current_screen) = current_screen {
            current_screen.draw(frame);
        }
    }

    fn has_current_screen_route(&self) -> bool {
        self.screen_router.contains_key(&self.current_screen_route)
    }

    fn quit_app(&mut self) {
        self.running = false;
    }

    fn navigate_to(&mut self, route: ScreenRoute) {
        self.current_screen_route = route;
    }
}
