use crate::{
    navigation::{Navigator, Screen, ScreenId},
    waiting_screen::WaitingScreen,
};
use crossterm::event::EventStream;
use ratatui::{DefaultTerminal, Frame};
use tokio_stream::StreamExt;

#[derive(Debug, Clone, PartialEq)]
pub enum Action {
    ShowScreen(ScreenId),
    GoBack,
    Exit,
}

pub struct App {
    navigator: Navigator,
    running: bool,
}

impl App {
    pub fn new() -> Self {
        let mut navigator = Navigator::new();
        navigator.show_screen(Box::new(WaitingScreen::new()));

        Self {
            running: false,
            navigator,
        }
    }

    pub async fn run(&mut self, terminal: &mut DefaultTerminal) -> anyhow::Result<()> {
        self.running = true;
        let mut events = EventStream::new();

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
                        if let Some(action) = screen.handle_event(&event).await {
                            self.handle_action(action);
                            break;
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
        let screen = match self.navigator.current() {
            Some(screen) => screen,
            None => return,
        };

        screen.draw(frame);
    }

    fn handle_action(&mut self, action: Action) {
        match action {
            Action::Exit => self.exit(),
            Action::ShowScreen(screen_id) => self.show_screen(screen_id),
            Action::GoBack => self.go_back(),
        }
    }

    fn exit(&mut self) {
        self.running = false;
    }

    fn show_screen(&mut self, screen_id: ScreenId) {
        let screen: Box<dyn Screen> = match screen_id {
            ScreenId::Waiting => Box::new(WaitingScreen::new()),
            ScreenId::Requests => Box::new(crate::requests_screen::RequestsScreen::new()),
        };
        self.navigator.show_screen(screen);
    }

    fn go_back(&mut self) {
        self.navigator.go_back();
    }
}
