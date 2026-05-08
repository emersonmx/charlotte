use crate::{
    app::{Message as AppMessage, Screen, is_quit_key_event},
    theme,
    widgets::BorderedText,
};
use crossterm::event::{Event, KeyCode};
use ratatui::{
    Frame,
    layout::Margin,
    widgets::{Scrollbar, ScrollbarState},
};

#[derive(Debug, Clone, PartialEq)]
pub enum Message {
    ScrollUp,
    ScrollDown,
}

impl From<Message> for AppMessage {
    fn from(message: Message) -> Self {
        AppMessage::ErrorModal(message)
    }
}

pub struct ErrorModal {
    content: String,
    scrollbar_state: ScrollbarState,
}

#[allow(dead_code)]
impl ErrorModal {
    pub fn new(content: &str) -> Self {
        let scrollbar_state = ScrollbarState::default();

        Self {
            content: content.to_string(),
            scrollbar_state,
        }
    }

    fn scroll_up(&mut self) -> Option<AppMessage> {
        self.scrollbar_state.prev();
        None
    }

    fn scroll_down(&mut self) -> Option<AppMessage> {
        self.scrollbar_state.next();
        None
    }
}

impl Screen for ErrorModal {
    fn draw(&mut self, frame: &mut Frame) {
        let area = frame.area();
        let mut area = area.inner(Margin {
            vertical: (area.height as f32 * 0.4).ceil() as u16,
            horizontal: (area.width as f32 * 0.35).ceil() as u16,
        });
        let content_length = textwrap::wrap(&self.content, area.width as usize).len();
        area.height = area.height.min(content_length as u16 + 2);

        self.scrollbar_state = self
            .scrollbar_state
            .content_length(content_length.saturating_sub(area.height as usize - 4));

        let bordered_text = BorderedText::new(self.content.clone())
            .title(Some("Error".to_string()))
            .scroll((
                self.scrollbar_state.get_position().try_into().unwrap_or(0),
                0,
            ))
            .focused(true)
            .focus_style(theme::styles::error());
        frame.render_widget(bordered_text, area);

        let scrollbar = Scrollbar::default()
            .orientation(ratatui::widgets::ScrollbarOrientation::VerticalRight)
            .style(theme::styles::error());
        frame.render_stateful_widget(
            scrollbar,
            area.inner(Margin {
                vertical: 1,
                horizontal: 0,
            }),
            &mut self.scrollbar_state,
        );
    }

    fn handle_event(&self, event: Event) -> Option<AppMessage> {
        if let Some(AppMessage::Quit) = is_quit_key_event(&event) {
            return Some(AppMessage::Quit);
        }

        if let Event::Key(key_event) = event {
            match key_event.code {
                KeyCode::Up | KeyCode::Char('k') => return Some(Message::ScrollUp.into()),
                KeyCode::Down | KeyCode::Char('j') => return Some(Message::ScrollDown.into()),
                _ => return Some(AppMessage::CloseModal),
            }
        }

        None
    }

    fn update(&mut self, message: AppMessage) -> Option<AppMessage> {
        let message = match message {
            AppMessage::ErrorModal(message) => message,
            _ => return None,
        };

        match message {
            Message::ScrollUp => self.scroll_up(),
            Message::ScrollDown => self.scroll_down(),
        }
    }
}
