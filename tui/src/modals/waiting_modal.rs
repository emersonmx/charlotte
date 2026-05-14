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
    use crate::app::RequestEntry;

    use super::*;
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    use insta::assert_snapshot;
    use ratatui::{Terminal, backend::TestBackend};
    use rstest::{fixture, rstest};

    #[fixture]
    fn terminal() -> Terminal<TestBackend> {
        Terminal::new(TestBackend::new(200, 5)).unwrap()
    }

    #[fixture]
    fn modal() -> WaitingModal {
        WaitingModal::new("localhost", 8888)
    }

    #[rstest]
    fn create_modal(modal: WaitingModal) {
        assert_eq!(modal.server_host, "localhost");
        assert_eq!(modal.server_port, 8888);
    }

    #[rstest]
    fn draw_modal(mut terminal: Terminal<TestBackend>, mut modal: WaitingModal) {
        terminal.draw(|frame| modal.draw(frame)).unwrap();
        assert_snapshot!(terminal.backend());
    }

    #[rstest]
    fn event_to_message(modal: WaitingModal) {
        let quit_event = Event::Key(KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE));
        let non_quit_event = Event::Key(KeyEvent::new(KeyCode::Char('x'), KeyModifiers::NONE));

        assert_eq!(modal.handle_event(quit_event), Some(AppMessage::Quit));
        assert_eq!(modal.handle_event(non_quit_event), None);
    }

    #[rstest]
    fn update_on_message(mut modal: WaitingModal) {
        assert_eq!(
            modal.update(AppMessage::RequestEntryUpdated(
                RequestEntry::default().into()
            )),
            Some(AppMessage::CloseModal)
        );
    }

    #[rstest]
    fn ignore_non_modal_messages(mut modal: WaitingModal) {
        assert_eq!(
            modal.update(AppMessage::CopyToClipboard("test".to_string())),
            None
        );
    }
}
