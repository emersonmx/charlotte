use crate::{
    app::{Message as AppMessage, Screen, is_quit_key_event},
    widgets::BorderedText,
};
use crossterm::event::Event;
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
        let message = textwrap::wrap(&message, line_length);
        let message_width = line_length as u16 + 2;
        let message_height = message.len() as u16 + 2;
        let area = area.centered(
            Constraint::Max(message_width),
            Constraint::Max(message_height),
        );

        let bordered_text = BorderedText::new(message.join("\n"))
            .focused(true)
            .centered();
        frame.render_widget(bordered_text, area);
    }

    fn handle_event(&self, event: Event) -> Option<AppMessage> {
        is_quit_key_event(&event)
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
    use insta::assert_snapshot;
    use ratatui::{Terminal, backend::TestBackend};
    use rstest::{fixture, rstest};

    #[fixture]
    fn terminal() -> Terminal<TestBackend> {
        Terminal::new(TestBackend::new(80, 24)).unwrap()
    }

    #[fixture]
    fn screen() -> WaitingModal {
        WaitingModal::new("localhost", 8888)
    }

    #[rstest]
    fn create_waiting_screen(screen: WaitingModal) {
        assert_eq!(screen.server_host, "localhost");
        assert_eq!(screen.server_port, 8888);
    }

    #[rstest]
    fn show_waiting_message(mut terminal: Terminal<TestBackend>, mut screen: WaitingModal) {
        terminal.draw(|frame| screen.draw(frame)).unwrap();
        assert_snapshot!(terminal.backend());
    }

    #[rstest]
    fn quit_on_q_key(screen: WaitingModal) {
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
