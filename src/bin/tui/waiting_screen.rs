use crate::app::{ControlFlow, Screen, ScreenRoute};
use async_trait::async_trait;
use crossterm::event::{self, Event};
use ratatui::{
    Frame,
    text::Text,
    widgets::{Block, Paragraph},
};

#[derive(Debug)]
pub struct WaitingScreen;

impl WaitingScreen {
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait]
impl Screen for WaitingScreen {
    async fn handle_events(&mut self, event: &Event) -> ControlFlow {
        #[allow(clippy::single_match)]
        match event {
            Event::Key(key_event) => {
                if let event::KeyCode::Char('q') = key_event.code {
                    return ControlFlow::Break;
                }
                if let event::KeyCode::Char('r') = key_event.code {
                    return ControlFlow::Navigate(ScreenRoute::Requests);
                }
            }
            _ => {}
        };

        ControlFlow::Continue
    }

    fn draw(&self, frame: &mut Frame) {
        let message = "Waiting for requests on port 8888... (press 'q' to quit)";
        let text = Text::raw(message);
        let block = Block::bordered();
        let paragraph = Paragraph::new(text).centered().block(block);

        let area = frame.area();
        let (xpad, ypad) = (4, 1);
        let width = message.chars().count() as u16 + xpad;
        let height = ypad + 2;
        let x = area.x + (area.width - width) / 2;
        let y = area.y + (area.height - height) / 2;
        let centered_area = ratatui::layout::Rect::new(x, y, width, height);

        frame.render_widget(paragraph, centered_area);
    }
}
