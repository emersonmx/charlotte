use crate::app::{Message as AppMessage, Screen, is_quit_key_event};
use crossterm::event::Event;
use ratatui::{
    Frame,
    layout::Rect,
    text::Text,
    widgets::{Block, Paragraph},
};

fn centered_area(area: Rect, width: u16, height: u16) -> Rect {
    let x = area.x + (area.width.saturating_sub(width)) / 2;
    let y = area.y + (area.height.saturating_sub(height)) / 2;
    Rect::new(x, y, width, height)
}

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

impl Screen for WaitingScreen {
    fn draw(&mut self, frame: &mut Frame) {
        let message = format!(
            "Waiting for requests on {}:{} (press 'q' to quit)",
            self.server_host, self.server_port
        );
        let border_padding = 2;
        let (x_padding, y_padding) = (2, 1);
        let width = message.chars().count() as u16 + (x_padding + border_padding);
        let height = y_padding + border_padding;
        let centered_area = centered_area(frame.area(), width, height);

        let text = Text::from(message);
        let block = Block::bordered();
        let paragraph = Paragraph::new(text).centered().block(block);

        frame.render_widget(paragraph, centered_area);
    }

    fn handle_event(&self, event: Event) -> Option<AppMessage> {
        is_quit_key_event(&event)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use insta::assert_snapshot;
    use ratatui::{Terminal, backend::TestBackend};
    use rstest::{fixture, rstest};

    #[fixture]
    fn terminal() -> Terminal<TestBackend> {
        Terminal::new(TestBackend::new(80, 24)).unwrap()
    }

    #[fixture]
    fn screen() -> WaitingScreen {
        WaitingScreen::new("localhost".to_string(), 8888)
    }

    #[rstest]
    fn create_waiting_screen(screen: WaitingScreen) {
        assert_eq!(screen.server_host, "localhost");
        assert_eq!(screen.server_port, 8888);
    }

    #[rstest]
    fn show_waiting_message(mut terminal: Terminal<TestBackend>, mut screen: WaitingScreen) {
        terminal.draw(|frame| screen.draw(frame)).unwrap();
        assert_snapshot!(terminal.backend());
    }

    #[rstest]
    fn quit_on_q_key(screen: WaitingScreen) {
        let quit_event = Event::Key(crossterm::event::KeyEvent {
            code: crossterm::event::KeyCode::Char('q'),
            modifiers: crossterm::event::KeyModifiers::NONE,
            kind: crossterm::event::KeyEventKind::Press,
            state: crossterm::event::KeyEventState::NONE,
        });

        let message = screen.handle_event(quit_event);
        assert_eq!(message, Some(AppMessage::Quit));
    }
}
