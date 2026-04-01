use crate::app::{ControlFlow, Screen};
use async_trait::async_trait;
use crossterm::event::{self, Event};
use ratatui::{Frame, text::Text, widgets::Widget};

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
            }
            _ => {}
        };

        ControlFlow::Continue
    }

    fn draw(&self, frame: &mut Frame) {
        let text = Text::raw("Waiting for requests on port 8888... (press 'q' to quit)").centered();
        text.render(frame.area(), frame.buffer_mut());
    }
}
