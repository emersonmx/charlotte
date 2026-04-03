use crate::app::Action;
use async_trait::async_trait;
use crossterm::event::Event;
use ratatui::Frame;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ScreenId {
    Waiting,
    Requests,
}

#[async_trait]
pub trait Screen {
    fn id(&self) -> ScreenId;
    async fn handle_event(&mut self, event: &Event) -> Option<Action>;
    fn draw(&self, frame: &mut Frame);
}

pub struct Navigator {
    stack: Vec<Box<dyn Screen>>,
}

impl Navigator {
    pub fn new() -> Self {
        Self { stack: vec![] }
    }

    pub fn show_screen(&mut self, screen: Box<dyn Screen>) {
        let screen_id = screen.id();
        if let Some(pos) = self.stack.iter().position(|s| s.id() == screen_id) {
            self.stack.truncate(pos + 1);
        } else {
            self.stack.push(screen);
        }
    }

    pub fn push(&mut self, screen: Box<dyn Screen>) {
        self.stack.push(screen);
    }

    pub fn pop(&mut self) {
        self.stack.pop();
    }

    pub fn has_screen(&self) -> bool {
        !self.stack.is_empty()
    }

    pub fn current(&self) -> Option<&dyn Screen> {
        self.stack.last().map(|v| &**v)
    }

    pub fn current_mut(&mut self) -> Option<&mut Box<dyn Screen>> {
        self.stack.last_mut()
    }
}
