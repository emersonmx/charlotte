use crate::{
    app::{Message as AppMessage, RequestEntry, RequestStore, Screen},
    inputmap::{is_accept_pressed, is_down_pressed, is_quit_pressed, is_up_pressed},
    theme,
    widgets::BorderedText,
};
use crossterm::event::Event;
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction::Vertical, Layout, Rect},
    symbols::{self},
    text::Text,
    widgets::{
        Block, Borders, Cell, Row, Scrollbar, ScrollbarOrientation, ScrollbarState, Table,
        TableState,
    },
};

#[derive(Debug, Clone, PartialEq)]
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

#[derive(Debug, Clone, Default, PartialEq)]
pub struct RequestEntryRow {
    pub request_id: String,
    pub method: String,
    pub url: String,
    pub body: String,
    pub status: String,
}

impl From<&RequestEntry> for RequestEntryRow {
    fn from(entry: &RequestEntry) -> Self {
        let status = if let Some(response) = &entry.response {
            response.status.to_string()
        } else {
            "Pending".to_string()
        };
        let body = {
            let request_body_len = entry.request.body.as_bytes().len();
            let response_body_len = entry
                .response
                .as_ref()
                .map_or(0, |response| response.body.as_bytes().len());
            format!(
                "Sent: {} bytes | Received: {} bytes",
                request_body_len, response_body_len
            )
        };

        RequestEntryRow {
            request_id: entry.request_id.to_string(),
            method: entry.request.method.to_string(),
            url: entry.request.url.to_string(),
            body,
            status,
        }
    }
}

impl From<RequestEntryRow> for Row<'_> {
    fn from(row: RequestEntryRow) -> Self {
        Row::new(vec![
            Cell::from(Text::from(row.request_id).alignment(Alignment::Right)),
            Cell::from(row.method),
            Cell::from(row.url),
            Cell::from(row.body),
            Cell::from(row.status),
        ])
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct RequestsTableColumnWidths {
    request_id: u16,
    method: u16,
    url: u16,
    body: u16,
    status: u16,
}

impl RequestsTableColumnWidths {
    pub const TABLE_COLUMN_REQ_ID: &str = "ReqId";
    pub const TABLE_COLUMN_METHOD: &str = "Method";
    pub const TABLE_COLUMN_URL: &str = "URL";
    pub const TABLE_COLUMN_BODY: &str = "Body";
    pub const TABLE_COLUMN_STATUS: &str = "Status";

    pub fn update(&mut self, row: RequestEntryRow) {
        self.request_id = self.request_id.max(row.request_id.len() as u16);
        self.method = self.method.max(row.method.len() as u16);
        self.url = self.url.max(row.url.len() as u16);
        self.body = self.body.max(row.body.len() as u16);
        self.status = self.status.max(row.status.len() as u16);
    }

    pub fn to_table_widths(&self) -> [Constraint; 5] {
        [
            Constraint::Length(self.request_id),
            Constraint::Length(self.method),
            Constraint::Length(self.url),
            Constraint::Min(self.body),
            Constraint::Length(self.status),
        ]
    }
}

impl Default for RequestsTableColumnWidths {
    fn default() -> Self {
        let mut widths = Self {
            request_id: 0,
            method: 0,
            url: 0,
            body: 0,
            status: 0,
        };
        widths.update(RequestEntryRow {
            request_id: Self::TABLE_COLUMN_REQ_ID.to_string(),
            method: Self::TABLE_COLUMN_METHOD.to_string(),
            url: Self::TABLE_COLUMN_URL.to_string(),
            body: Self::TABLE_COLUMN_BODY.to_string(),
            status: Self::TABLE_COLUMN_STATUS.to_string(),
        });
        widths
    }
}

#[derive(Debug, Default, Clone, PartialEq)]
pub struct RequestsScreenState {
    pub selected_request_entry: Option<usize>,
    pub table_column_widths: RequestsTableColumnWidths,
    pub table_state: TableState,
    pub table_scroll_state: ScrollbarState,
}

impl RequestsScreenState {
    fn update_table_column_widths(&mut self, request_entry: &RequestEntry) {
        let row = RequestEntryRow::from(request_entry);
        self.table_column_widths.update(row);
    }

    fn update_table_scroll(&mut self, requests_length: usize) {
        if self.table_state.selected().is_none() {
            self.table_state = self.table_state.with_selected(Some(0));
            self.selected_request_entry = self.table_state.selected();
        }
        self.table_scroll_state = self.table_scroll_state.content_length(requests_length);
    }

    fn select_previous_request_entry(&mut self) {
        self.table_state.select_previous();
        self.table_scroll_state.prev();
        self.selected_request_entry = self.table_state.selected();
    }

    fn select_next_request_entry(&mut self) {
        self.table_state.select_next();
        self.table_scroll_state.next();
        self.selected_request_entry = self.table_state.selected();
    }
}

pub struct RequestsScreen {
    request_store: RequestStore,
    state: RequestsScreenState,
}

impl RequestsScreen {
    pub fn new(request_store: RequestStore, state: RequestsScreenState) -> Self {
        Self {
            request_store,
            state,
        }
    }
}

impl RequestsScreen {
    fn request_entry_updated(&mut self, request_entry: Box<RequestEntry>) -> Option<AppMessage> {
        self.state
            .update_table_column_widths(request_entry.as_ref());
        Some(Message::UpdateTableState.into())
    }

    fn update_table_state(&mut self) -> Option<AppMessage> {
        self.state.update_table_scroll(self.requests_length());
        Some(AppMessage::StoreRequestsScreenState(
            self.state.clone().into(),
        ))
    }

    fn select_previous_row(&mut self) -> Option<AppMessage> {
        self.state.select_previous_request_entry();
        Some(AppMessage::StoreRequestsScreenState(
            self.state.clone().into(),
        ))
    }

    fn select_next_row(&mut self) -> Option<AppMessage> {
        self.state.select_next_request_entry();
        Some(AppMessage::StoreRequestsScreenState(
            self.state.clone().into(),
        ))
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
        let table_widths = &self.state.table_column_widths;

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
        frame.render_stateful_widget(table, rows_layout, &mut self.state.table_state);

        let area = Rect {
            height: rows_layout.height - 1,
            ..rows_layout
        };
        frame.render_stateful_widget(
            Scrollbar::default()
                .orientation(ScrollbarOrientation::VerticalRight)
                .style(theme::styles::reset()),
            area,
            &mut self.state.table_scroll_state,
        );

        let text = "Arrow keys or j/k to navigate, Enter to view details. q to quit.";
        let status_bar_text = BorderedText::new(text);
        frame.render_widget(status_bar_text, status_area);
    }

    fn handle_event(&self, event: Event) -> Option<AppMessage> {
        if is_quit_pressed(&event) {
            return Some(AppMessage::Quit);
        }

        if is_up_pressed(&event) {
            return Some(Message::SelectPreviousRow.into());
        }

        if is_down_pressed(&event) {
            return Some(Message::SelectNextRow.into());
        }

        if is_accept_pressed(&event) {
            return Some(AppMessage::ShowHttpClientScreen);
        }

        None
    }

    fn update(&mut self, message: AppMessage) -> Option<AppMessage> {
        let message = match message {
            AppMessage::RequestEntryUpdated(request_entry) => {
                return self.request_entry_updated(request_entry);
            }
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
