use crate::{
    app::{Message as AppMessage, Screen},
    inputmap::{Input, map_event_to_yesno},
    theme,
    widgets::BorderedText,
};
use crossterm::event::Event;
use ratatui::{Frame, layout::Constraint};

#[derive(Debug, Clone, PartialEq)]
pub enum Message {
    Confirm,
    Cancel,
}

impl From<Message> for AppMessage {
    fn from(message: Message) -> Self {
        AppMessage::ConfirmQuitModal(message)
    }
}

pub struct ConfirmQuitModal;

impl ConfirmQuitModal {
    pub fn new() -> Self {
        Self
    }

    fn confirm(&self) -> Option<AppMessage> {
        Some(AppMessage::Quit)
    }

    fn cancel(&self) -> Option<AppMessage> {
        Some(AppMessage::CloseModal)
    }
}

impl Screen for ConfirmQuitModal {
    fn draw(&mut self, frame: &mut Frame) {
        let area = frame.area();
        let message = "Are you sure you want to quit? (y/n)";
        let width = message.len() as u16 + 4;
        let height = 3u16;
        let area = area.centered(Constraint::Max(width), Constraint::Max(height));
        let bordered_text = BorderedText::new(message)
            .title(Some("Confirm Quit".to_string()))
            .focused(true)
            .centered()
            .focus_style(theme::styles::warning());
        frame.render_widget(bordered_text, area);
    }

    fn handle_event(&self, event: Event) -> Option<AppMessage> {
        match map_event_to_yesno(&event) {
            Some(Input::Confirm) => Some(Message::Confirm.into()),
            Some(Input::Cancel) => Some(Message::Cancel.into()),
            _ => None,
        }
    }

    fn update(&mut self, message: AppMessage) -> Option<AppMessage> {
        let message = match message {
            AppMessage::ConfirmQuitModal(message) => message,
            _ => return None,
        };

        match message {
            Message::Confirm => self.confirm(),
            Message::Cancel => self.cancel(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    use insta::assert_snapshot;
    use ratatui::{Terminal, backend::TestBackend};
    use rstest::{fixture, rstest};

    #[fixture]
    fn terminal() -> Terminal<TestBackend> {
        Terminal::new(TestBackend::new(40, 10)).unwrap()
    }

    #[fixture]
    fn modal() -> ConfirmQuitModal {
        ConfirmQuitModal::new()
    }

    #[rstest]
    fn draw_modal(mut terminal: Terminal<TestBackend>, mut modal: ConfirmQuitModal) {
        terminal.draw(|frame| modal.draw(frame)).unwrap();

        assert_snapshot!(terminal.backend());
    }

    #[rstest]
    fn handle_event_confirm(modal: ConfirmQuitModal) {
        let y_event = Event::Key(KeyEvent::new(KeyCode::Char('y'), KeyModifiers::NONE));
        let non_y_event = Event::Key(KeyEvent::new(KeyCode::Char('x'), KeyModifiers::NONE));

        assert_eq!(modal.handle_event(y_event), Some(Message::Confirm.into()));
        assert_eq!(modal.handle_event(non_y_event), None);
    }

    #[rstest]
    fn handle_event_cancel(modal: ConfirmQuitModal) {
        let n_event = Event::Key(KeyEvent::new(KeyCode::Char('n'), KeyModifiers::NONE));
        let non_n_event = Event::Key(KeyEvent::new(KeyCode::Char('x'), KeyModifiers::NONE));

        assert_eq!(modal.handle_event(n_event), Some(Message::Cancel.into()));
        assert_eq!(modal.handle_event(non_n_event), None);
    }

    #[rstest]
    fn update_confirm_cancel(mut modal: ConfirmQuitModal) {
        assert_eq!(
            modal.update(Message::Confirm.into()),
            Some(AppMessage::Quit)
        );
        assert_eq!(
            modal.update(Message::Cancel.into()),
            Some(AppMessage::CloseModal)
        );
    }
}
