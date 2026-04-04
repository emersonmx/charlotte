use crate::{
    app::{Action, Event},
    navigation::{Screen, ScreenId},
};
use async_trait::async_trait;
use crossterm::event::KeyCode;
use ratatui::{
    Frame,
    text::Text,
    widgets::{Block, Paragraph},
};
use std::sync::Arc;

#[derive(Debug)]
pub struct WaitingScreen {
    server_host: String,
    server_port: u16,
}

impl WaitingScreen {
    pub fn new(server_host: String, server_port: u16) -> Self {
        Self {
            server_port,
            server_host,
        }
    }
}

#[async_trait]
impl Screen for WaitingScreen {
    fn id(&self) -> ScreenId {
        ScreenId::Waiting
    }

    async fn handle_event(&mut self, event: &Event) -> Option<Action> {
        match event {
            Event::CrosstermEvent(event) => {
                if let crossterm::event::Event::Key(key_event) = event
                    && let KeyCode::Char('q') = key_event.code
                {
                    return Some(Action::Exit);
                }
            }
            Event::ProxyMessage(message) => {
                if let charlotte::Message::RequestSent(_) = message.as_ref() {
                    return Some(Action::FowardToScreen(
                        ScreenId::Requests,
                        Event::ProxyMessage(Arc::clone(message)),
                    ));
                }
            }
        };

        None
    }

    fn draw(&mut self, frame: &mut Frame) {
        let message = format!(
            "Waiting for requests on {}:{} (press 'q' to quit)",
            self.server_host, self.server_port
        );
        let text = Text::raw(&message);
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
