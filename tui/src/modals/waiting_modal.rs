use crate::{
    app::{Message as AppMessage, Screen},
    inputmap::{Input, map_event_to_input},
    widgets::BorderedText,
};
use crossterm::event::Event;
use formatter::wrap_text;
use ratatui::{Frame, layout::Constraint};

#[derive(Debug, Clone, PartialEq)]
pub struct WaitingModal {
    server_host: String,
    server_port: u16,
}

impl WaitingModal {
    pub fn new(server_host: impl Into<String>, server_port: u16) -> Self {
        Self {
            server_host: server_host.into(),
            server_port,
        }
    }
}

impl Screen for WaitingModal {
    fn draw(&mut self, frame: &mut Frame) {
        let area = frame.area();
        let line_length = area.width as usize / 3;
        let message = format!(
            "Waiting for requests on {}:{} (press 'q' to quit)",
            self.server_host, self.server_port
        );
        let message = wrap_text(&message, line_length);
        let lines = message.lines().count();
        let message_width = line_length as u16 + 2;
        let message_height = lines as u16 + 2;
        let area = area.centered(
            Constraint::Max(message_width),
            Constraint::Max(message_height),
        );

        let bordered_text = BorderedText::new(message).focused(true).centered();
        frame.render_widget(bordered_text, area);
    }

    fn handle_event(&self, event: Event) -> Option<AppMessage> {
        match map_event_to_input(&event) {
            Some(Input::Quit) => Some(AppMessage::Quit),
            _ => None,
        }
    }

    fn update(&mut self, message: AppMessage) -> Option<AppMessage> {
        match message {
            AppMessage::RequestEntryUpdated(_) => Some(AppMessage::CloseModal),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyCode, KeyModifiers};
    use insta::assert_snapshot;
    use ratatui::{Terminal, backend::TestBackend};
    use rstest::{fixture, rstest};

    #[fixture]
    fn terminal() -> Terminal<TestBackend> {
        Terminal::new(TestBackend::new(200, 5)).unwrap()
    }

    #[fixture]
    fn screen() -> WaitingModal {
        WaitingModal::new("localhost", 8888)
    }

    #[rstest]
    fn create_modal(screen: WaitingModal) {
        assert_eq!(screen.server_host, "localhost");
        assert_eq!(screen.server_port, 8888);
    }

    #[rstest]
    fn quit_on_q_key(screen: WaitingModal) {
        let quit_event = Event::Key(crossterm::event::KeyEvent::new(
            KeyCode::Char('q'),
            KeyModifiers::NONE,
        ));

        let message = screen.handle_event(quit_event);
        assert_eq!(message, Some(AppMessage::Quit));
    }

    #[rstest]
    fn draw_modal(mut terminal: Terminal<TestBackend>, mut screen: WaitingModal) {
        terminal.draw(|frame| screen.draw(frame)).unwrap();
        assert_snapshot!(terminal.backend());
    }
}
