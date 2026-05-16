use crate::{
    app::{Message as AppMessage, Screen},
    inputmap::{Input, map_event_to_input},
    widgets::BorderedText,
};
use crossterm::event::Event;
use ratatui::{
    Frame,
    layout::Constraint,
    widgets::{Scrollbar, ScrollbarOrientation, ScrollbarState},
};

#[derive(Debug, Clone, PartialEq)]
pub enum Message {
    ScrollUp,
    ScrollDown,
    Home,
    End,
    PageUp,
    PageDown,
}

impl From<Message> for AppMessage {
    fn from(message: Message) -> Self {
        AppMessage::ShortcutsModal(message)
    }
}

const SHORTCUTS: &[(&str, &str)] = &[
    ("<up>/<k>", "Move Up"),
    ("<down>/<j>", "Move Down"),
    ("<left>/<h>", "Move Left"),
    ("<right>/<l>", "Move Right"),
    ("<pgup>", "Page Up"),
    ("<pgdown>", "Page Down"),
    ("<home>", "Go To First Item"),
    ("<end>", "Go To Last Item"),
    ("<0-9>", "Select Section (Details Screen)"),
    ("<enter>", "Select/Confirm"),
    ("<esc>/<backspace>", "Back/Close Modal"),
    ("<y>", "Copy Selected Value"),
    ("<tab>/<]>", "Next Tab"),
    ("<s-tab>/<[>", "Previous Tab"),
    ("<?>", "Help"),
    ("<q>", "Quit"),
];

pub struct ShortcutsModal {
    scroll: usize,
    scrollbar_state: ScrollbarState,
}

impl ShortcutsModal {
    const PAGE_SCROLL_STEP: usize = 10;

    pub fn new() -> Self {
        let content_length = SHORTCUTS.len();
        Self {
            scroll: 0,
            scrollbar_state: ScrollbarState::new(content_length),
        }
    }

    fn scroll_up(&mut self) -> Option<AppMessage> {
        if self.scroll > 0 {
            self.scroll -= 1;
            self.scrollbar_state.prev();
        }
        None
    }

    fn scroll_down(&mut self) -> Option<AppMessage> {
        if self.scroll + 1 < SHORTCUTS.len() {
            self.scroll += 1;
            self.scrollbar_state.next();
        }
        None
    }

    fn home(&mut self) -> Option<AppMessage> {
        self.scroll = 0;
        self.scrollbar_state = self.scrollbar_state.position(0);
        None
    }

    fn end(&mut self) -> Option<AppMessage> {
        if !SHORTCUTS.is_empty() {
            self.scroll = SHORTCUTS.len() - 1;
            self.scrollbar_state = self.scrollbar_state.position(self.scroll);
        }
        None
    }

    fn page_up(&mut self) -> Option<AppMessage> {
        if self.scroll >= Self::PAGE_SCROLL_STEP {
            self.scroll -= Self::PAGE_SCROLL_STEP;
        } else {
            self.scroll = 0;
        }
        self.scrollbar_state = self.scrollbar_state.position(self.scroll);
        None
    }

    fn page_down(&mut self) -> Option<AppMessage> {
        let max = SHORTCUTS.len().saturating_sub(1);
        self.scroll = (self.scroll + Self::PAGE_SCROLL_STEP).min(max);
        self.scrollbar_state = self.scrollbar_state.position(self.scroll);
        None
    }
}

impl Screen for ShortcutsModal {
    fn draw(&mut self, frame: &mut Frame) {
        let area = frame.area();
        let max_width = area.width as usize / 2;
        let max_height = area.height as usize / 4;
        let width = max_width.min(80) as u16 + 2;
        let height = SHORTCUTS.len().min(max_height) as u16 + 2;
        let centered_area = area.centered(Constraint::Max(width), Constraint::Max(height));

        let max_key_len = SHORTCUTS
            .iter()
            .map(|(key, _)| key.len())
            .max()
            .unwrap_or(0);
        let lines: Vec<String> = SHORTCUTS
            .iter()
            .map(|(key, desc)| format!("{:<width$} {}", key, desc, width = max_key_len))
            .collect();
        let text = lines.join("\n");
        let visible_lines = centered_area.height.saturating_sub(2) as usize;
        let max_scroll = lines.len().saturating_sub(visible_lines);
        let scroll = self.scroll.min(max_scroll);
        let bordered_text = BorderedText::new(text)
            .title(Some("Shortcuts".to_string()))
            .scroll((scroll as u16, 0));
        frame.render_widget(bordered_text, centered_area);

        self.scrollbar_state = self
            .scrollbar_state
            .content_length(max_scroll)
            .position(scroll);

        let scrollbar = Scrollbar::default().orientation(ScrollbarOrientation::VerticalRight);
        frame.render_stateful_widget(
            scrollbar,
            centered_area.inner(ratatui::layout::Margin {
                vertical: 1,
                horizontal: 0,
            }),
            &mut self.scrollbar_state,
        );
    }

    fn handle_event(&self, event: Event) -> Option<AppMessage> {
        match map_event_to_input(&event) {
            Some(Input::Up) => Some(Message::ScrollUp.into()),
            Some(Input::Down) => Some(Message::ScrollDown.into()),
            Some(Input::Home) => Some(Message::Home.into()),
            Some(Input::End) => Some(Message::End.into()),
            Some(Input::PageUp) => Some(Message::PageUp.into()),
            Some(Input::PageDown) => Some(Message::PageDown.into()),
            Some(Input::Back | Input::Quit) => Some(AppMessage::CloseModal),
            _ => None,
        }
    }

    fn update(&mut self, message: AppMessage) -> Option<AppMessage> {
        let message = match message {
            AppMessage::ShortcutsModal(msg) => msg,
            _ => return None,
        };

        match message {
            Message::ScrollUp => self.scroll_up(),
            Message::ScrollDown => self.scroll_down(),
            Message::Home => self.home(),
            Message::End => self.end(),
            Message::PageUp => self.page_up(),
            Message::PageDown => self.page_down(),
        }
    }
}
