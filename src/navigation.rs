use crate::app::{Action, Event};
use async_trait::async_trait;
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
