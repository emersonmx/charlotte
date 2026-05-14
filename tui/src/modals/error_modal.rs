use crate::{
    app::{Message as AppMessage, Screen},
    inputmap::{Input, map_event_to_input},
    theme,
    widgets::BorderedText,
};
use crossterm::event::Event;
use ratatui::{
    Frame,
    layout::{Constraint, Margin},
    widgets::{Scrollbar, ScrollbarOrientation, ScrollbarState},
};

#[allow(clippy::enum_variant_names)]
#[derive(Debug, Clone, PartialEq)]
pub enum Message {
    ScrollUp,
    ScrollDown,
    ScrollLeft,
    ScrollRight,
}

impl From<Message> for AppMessage {
    fn from(message: Message) -> Self {
        AppMessage::ErrorModal(message)
    }
}

pub struct ErrorModal {
    content: String,
    scrollbar_vertical_state: ScrollbarState,
    scrollbar_horizontal_state: ScrollbarState,
}

#[allow(dead_code)]
impl ErrorModal {
    const X_SCROLL_STEP: usize = 10;

    pub fn new(content: &str) -> Self {
        let scrollbar_vertical_state = ScrollbarState::default();
        let scrollbar_horizontal_state = ScrollbarState::default();

        Self {
            content: content.to_string(),
            scrollbar_vertical_state,
            scrollbar_horizontal_state,
        }
    }

    fn scroll_up(&mut self) -> Option<AppMessage> {
        self.scrollbar_vertical_state.prev();
        None
    }

    fn scroll_down(&mut self) -> Option<AppMessage> {
        self.scrollbar_vertical_state.next();
        None
    }

    fn scroll_left(&mut self) -> Option<AppMessage> {
        for _ in 0..Self::X_SCROLL_STEP {
            self.scrollbar_horizontal_state.prev();
        }
        None
    }

    fn scroll_right(&mut self) -> Option<AppMessage> {
        for _ in 0..Self::X_SCROLL_STEP {
            self.scrollbar_horizontal_state.next();
        }
        None
    }
}

impl Screen for ErrorModal {
    fn draw(&mut self, frame: &mut Frame) {
        let area = frame.area();
        let max_width = area.width as usize / 3;
        let max_height = area.height as usize / 4;
        let content = &self.content;
        let message_width = content.lines().map(|line| line.len()).max().unwrap_or(0);
        let message_height = content.lines().count();
        let area = area.centered(
            Constraint::Max(message_width.min(max_width) as u16 + 2),
            Constraint::Max(message_height.min(max_height) as u16 + 2),
        );

        self.scrollbar_horizontal_state = self
            .scrollbar_horizontal_state
            .content_length(message_width.saturating_sub(max_width) + 1);
        self.scrollbar_vertical_state = self
            .scrollbar_vertical_state
            .content_length(message_height.saturating_sub(max_height) + 1);

        let bordered_text = BorderedText::new(content)
            .title(Some("Error".to_string()))
            .scroll((
                self.scrollbar_vertical_state
                    .get_position()
                    .try_into()
                    .unwrap_or(0),
                self.scrollbar_horizontal_state
                    .get_position()
                    .try_into()
                    .unwrap_or(0),
            ))
            .focused(true)
            .focus_style(theme::styles::error());
        frame.render_widget(bordered_text, area);

        let scrollbar = Scrollbar::default()
            .orientation(ScrollbarOrientation::VerticalRight)
            .style(theme::styles::error());
        frame.render_stateful_widget(
            scrollbar,
            area.inner(Margin {
                vertical: 1,
                horizontal: 0,
            }),
            &mut self.scrollbar_vertical_state,
        );

        let scrollbar = Scrollbar::default()
            .orientation(ScrollbarOrientation::HorizontalBottom)
            .style(theme::styles::error());
        frame.render_stateful_widget(
            scrollbar,
            area.inner(Margin {
                vertical: 0,
                horizontal: 1,
            }),
            &mut self.scrollbar_horizontal_state,
        );
    }

    fn handle_event(&self, event: Event) -> Option<AppMessage> {
        match map_event_to_input(&event) {
            Some(Input::Up) => Some(Message::ScrollUp.into()),
            Some(Input::Down) => Some(Message::ScrollDown.into()),
            Some(Input::Left) => Some(Message::ScrollLeft.into()),
            Some(Input::Right) => Some(Message::ScrollRight.into()),
            Some(Input::Quit) => Some(AppMessage::Quit),
            Some(Input::AnyKey) => Some(AppMessage::CloseModal),
            _ => None,
        }
    }

    fn update(&mut self, message: AppMessage) -> Option<AppMessage> {
        let message = match message {
            AppMessage::ErrorModal(message) => message,
            _ => return None,
        };

        match message {
            Message::ScrollUp => self.scroll_up(),
            Message::ScrollDown => self.scroll_down(),
            Message::ScrollLeft => self.scroll_left(),
            Message::ScrollRight => self.scroll_right(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEventKind};
    use insta::assert_snapshot;
    use ratatui::{Terminal, backend::TestBackend};
    use rstest::{fixture, rstest};

    #[fixture]
    fn terminal() -> Terminal<TestBackend> {
        Terminal::new(TestBackend::new(80, 24)).unwrap()
    }

    #[fixture]
    fn modal() -> ErrorModal {
        // Create a long error message to test vertical and horizontal scrolling
        let txt = [
            "This is a very long error message that should trigger both vertical and horizontal scrolling in the error modal. It contains multiple lines of text to ensure that the vertical scrollbar is needed.",
            "Line 2: Additional details about the error that might be helpful for debugging purposes.",
            "Line 3: More information about the error, including possible causes and solutions.",
            "Line 4: Even more details to make sure we have enough content to test the scrollbars properly.",
            "Line 5: Final line of the error message to complete our test case for the error modal.",
            "Line 6: This line should be hidden initially and only visible after scrolling down.",
            "Line 7: This line should also be hidden initially and only visible after scrolling down.",
            "Line 8: This line should also be hidden initially and only visible after scrolling down.",
            "Line 9: This line should also be hidden initially and only visible after scrolling down.",
            "Line 10: This line should also be hidden initially and only visible after scrolling down.",
        ].join("\n");
        ErrorModal::new(&txt)
    }

    #[rstest]
    fn create_modal(modal: ErrorModal) {
        assert!(modal.content.contains("This is a very long error message"));
        assert_eq!(modal.scrollbar_vertical_state.get_position(), 0);
        assert_eq!(modal.scrollbar_horizontal_state.get_position(), 0);
    }

    #[rstest]
    fn scroll_up(mut terminal: Terminal<TestBackend>, mut modal: ErrorModal) {
        terminal.draw(|frame| modal.draw(frame)).unwrap();
        for _ in 0..5 {
            let message = modal.scroll_down();
            assert_eq!(message, None);
        }
        for _ in 0..3 {
            let message = modal.scroll_up();
            assert_eq!(message, None);
        }

        assert_eq!(modal.scrollbar_vertical_state.get_position(), 1);
    }

    #[rstest]
    fn scroll_down(mut terminal: Terminal<TestBackend>, mut modal: ErrorModal) {
        terminal.draw(|frame| modal.draw(frame)).unwrap();
        for _ in 0..5 {
            let message = modal.scroll_down();
            assert_eq!(message, None);
        }

        assert_eq!(modal.scrollbar_vertical_state.get_position(), 4);
    }

    #[rstest]
    fn scroll_left(mut terminal: Terminal<TestBackend>, mut modal: ErrorModal) {
        terminal.draw(|frame| modal.draw(frame)).unwrap();
        modal.scroll_right();
        modal.scroll_right();
        modal.scroll_left();

        assert_eq!(
            modal.scrollbar_horizontal_state.get_position(),
            ErrorModal::X_SCROLL_STEP
        );
    }

    #[rstest]
    fn scroll_right(mut terminal: Terminal<TestBackend>, mut modal: ErrorModal) {
        terminal.draw(|frame| modal.draw(frame)).unwrap();
        modal.scroll_right();

        assert_eq!(
            modal.scrollbar_horizontal_state.get_position(),
            ErrorModal::X_SCROLL_STEP
        );
    }

    #[rstest]
    fn draw_modal(mut terminal: Terminal<TestBackend>, mut modal: ErrorModal) {
        terminal.draw(|frame| modal.draw(frame)).unwrap();
        assert_snapshot!(terminal.backend());
    }

    #[rstest]
    fn update_scrollbar_state_on_draw(mut terminal: Terminal<TestBackend>, mut modal: ErrorModal) {
        assert_eq!(modal.scrollbar_vertical_state.get_position(), 0);
        assert_eq!(modal.scrollbar_horizontal_state.get_position(), 0);
        modal.scroll_down();
        modal.scroll_right();
        assert_eq!(modal.scrollbar_vertical_state.get_position(), 0);
        assert_eq!(modal.scrollbar_horizontal_state.get_position(), 0);

        terminal.draw(|frame| modal.draw(frame)).unwrap();

        modal.scroll_down();
        modal.scroll_right();
        assert_eq!(modal.scrollbar_vertical_state.get_position(), 1);
        assert_eq!(
            modal.scrollbar_horizontal_state.get_position(),
            ErrorModal::X_SCROLL_STEP
        );
    }

    #[rstest]
    fn event_to_message(modal: ErrorModal) {
        let up_event = Event::Key(KeyEvent::new(KeyCode::Up, KeyModifiers::NONE));
        let down_event = Event::Key(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE));
        let left_event = Event::Key(KeyEvent::new(KeyCode::Left, KeyModifiers::NONE));
        let right_event = Event::Key(KeyEvent::new(KeyCode::Right, KeyModifiers::NONE));
        let quit_event = Event::Key(KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE));
        let any_key_event = Event::Key(KeyEvent::new(KeyCode::Char('x'), KeyModifiers::NONE));
        let non_key_event = Event::Mouse(crossterm::event::MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: 0,
            row: 0,
            modifiers: KeyModifiers::NONE,
        });

        assert_eq!(modal.handle_event(up_event), Some(Message::ScrollUp.into()));
        assert_eq!(
            modal.handle_event(down_event),
            Some(Message::ScrollDown.into())
        );
        assert_eq!(
            modal.handle_event(left_event),
            Some(Message::ScrollLeft.into())
        );
        assert_eq!(
            modal.handle_event(right_event),
            Some(Message::ScrollRight.into())
        );
        assert_eq!(modal.handle_event(quit_event), Some(AppMessage::Quit));
        assert_eq!(
            modal.handle_event(any_key_event),
            Some(AppMessage::CloseModal)
        );
        assert_eq!(modal.handle_event(non_key_event), None);
    }

    #[rstest]
    fn update_on_message(mut terminal: Terminal<TestBackend>, mut modal: ErrorModal) {
        terminal.draw(|frame| modal.draw(frame)).unwrap();
        for _ in 0..5 {
            modal.scroll_down();
        }
        for _ in 0..2 {
            modal.scroll_right();
        }

        assert_eq!(modal.update(Message::ScrollUp.into()), None);
        assert_eq!(modal.scrollbar_vertical_state.get_position(), 3);

        assert_eq!(modal.update(Message::ScrollDown.into()), None);
        assert_eq!(modal.scrollbar_vertical_state.get_position(), 4);

        assert_eq!(modal.update(Message::ScrollLeft.into()), None);
        assert_eq!(
            modal.scrollbar_horizontal_state.get_position(),
            ErrorModal::X_SCROLL_STEP
        );

        assert_eq!(modal.update(Message::ScrollRight.into()), None);
        assert_eq!(
            modal.scrollbar_horizontal_state.get_position(),
            ErrorModal::X_SCROLL_STEP * 2
        );
    }

    #[rstest]
    fn ignore_non_modal_messages(mut modal: ErrorModal) {
        let non_modal_message = AppMessage::CopyToClipboard("test".to_string());
        assert_eq!(modal.update(non_modal_message), None);
    }
}
