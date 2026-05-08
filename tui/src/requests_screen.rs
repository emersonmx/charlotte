use crate::{
    app::{
        Message as AppMessage, RequestEntryRow, RequestStore, RequestsTableColumnWidths, Screen,
        SharedRequestsTableColumnWidths, is_quit_key_event,
    },
    theme,
    widgets::BorderedText,
};
use crossterm::event::{Event, KeyCode};
use ratatui::{
    Frame,
    layout::{Constraint, Direction::Vertical, Layout, Rect},
    symbols::{self},
    widgets::{
        Block, Borders, Row, Scrollbar, ScrollbarOrientation, ScrollbarState, Table, TableState,
    },
};

#[derive(Debug, PartialEq)]
pub enum Message {
    UpdateTableState,
    SelectPreviousRow,
    SelectNextRow,
}

impl From<Message> for AppMessage {
    fn from(message: Message) -> Self {
        AppMessage::RequestsScreen(message)
    }
}

pub struct RequestsScreen {
    request_store: RequestStore,
    column_widths: SharedRequestsTableColumnWidths,
    table_state: TableState,
    table_scroll_state: ScrollbarState,
}

impl RequestsScreen {
    pub fn new(
        request_store: RequestStore,
        column_widths: SharedRequestsTableColumnWidths,
    ) -> Self {
        Self {
            request_store,
            column_widths,
            table_state: TableState::default(),
            table_scroll_state: ScrollbarState::default(),
        }
    }
}

impl RequestsScreen {
    fn update_table_state(&mut self) -> Option<AppMessage> {
        if self.table_state.selected().is_none() {
            self.table_state.select_first();
        }
        self.table_scroll_state = self
            .table_scroll_state
            .content_length(self.requests_length());

        None
    }

    fn select_previous_row(&mut self) -> Option<AppMessage> {
        self.table_state.select_previous();
        self.table_scroll_state.prev();
        None
    }

    fn select_next_row(&mut self) -> Option<AppMessage> {
        self.table_state.select_next();
        self.table_scroll_state.next();
        None
    }

    fn requests_length(&self) -> usize {
        match self.request_store.lock() {
            Ok(store) => store.len(),
            Err(_) => 0,
        }
    }
}

impl Screen for RequestsScreen {
    fn draw(&mut self, frame: &mut Frame) {
        let layout = Layout::default().direction(Vertical).constraints([
            Constraint::Length(3),
            Constraint::Min(3),
            Constraint::Length(3),
        ]);
        let [header_layout, rows_layout, status_area] = frame.area().layout(&layout);
        let table_widths = if let Ok(widths) = self.column_widths.lock() {
            widths.clone()
        } else {
            RequestsTableColumnWidths::default()
        };

        let header = RequestEntryRow {
            request_id: RequestsTableColumnWidths::TABLE_COLUMN_REQ_ID.to_string(),
            method: RequestsTableColumnWidths::TABLE_COLUMN_METHOD.to_string(),
            url: RequestsTableColumnWidths::TABLE_COLUMN_URL.to_string(),
            body: RequestsTableColumnWidths::TABLE_COLUMN_BODY.to_string(),
            status: RequestsTableColumnWidths::TABLE_COLUMN_STATUS.to_string(),
        };
        let border_set = symbols::border::Set {
            bottom_left: symbols::line::VERTICAL_RIGHT,
            bottom_right: symbols::line::VERTICAL_LEFT,
            ..symbols::border::PLAIN
        };
        let table = Table::default()
            .header(header.into())
            .block(Block::bordered().border_set(border_set))
            .widths(table_widths.to_table_widths());
        frame.render_widget(table, header_layout);

        let rows = match self.request_store.lock() {
            Ok(store) => store
                .values()
                .map(RequestEntryRow::from)
                .map(Row::from)
                .collect(),
            Err(_) => vec![],
        };
        let table = Table::default()
            .rows(rows)
            .block(Block::bordered().borders(Borders::BOTTOM | Borders::LEFT | Borders::RIGHT))
            .widths(table_widths.to_table_widths())
            .row_highlight_style(theme::styles::highlight());
        frame.render_stateful_widget(table, rows_layout, &mut self.table_state);

        let area = Rect {
            height: rows_layout.height - 1,
            ..rows_layout
        };
        frame.render_stateful_widget(
            Scrollbar::default()
                .orientation(ScrollbarOrientation::VerticalRight)
                .style(theme::styles::reset()),
            area,
            &mut self.table_scroll_state,
        );

        let text = "Arrow keys or j/k to navigate, Enter to view details. q to quit.";
        let status_bar_text = BorderedText::new(text);
        frame.render_widget(status_bar_text, status_area);
    }

    fn handle_event(&self, event: Event) -> Option<AppMessage> {
        if let Some(message) = is_quit_key_event(&event) {
            return Some(message);
        }

        if let Event::Key(key_event) = event {
            match key_event.code {
                KeyCode::Up | KeyCode::Char('k') => return Some(Message::SelectPreviousRow.into()),
                KeyCode::Down | KeyCode::Char('j') => return Some(Message::SelectNextRow.into()),
                KeyCode::Enter => {
                    let selected = self.table_state.selected()?;
                    let store = self.request_store.lock().ok()?;
                    let request_entry = store.values().nth(selected).cloned()?;
                    return Some(AppMessage::ShowHttpClientScreen(Box::new(request_entry)));
                }
                _ => {}
            }
        }

        None
    }

    fn update(&mut self, message: AppMessage) -> Option<AppMessage> {
        let message = match message {
            AppMessage::RequestsScreen(message) => message,
            _ => return None,
        };

        match message {
            Message::UpdateTableState => self.update_table_state(),
            Message::SelectPreviousRow => self.select_previous_row(),
            Message::SelectNextRow => self.select_next_row(),
        }
    }
}
