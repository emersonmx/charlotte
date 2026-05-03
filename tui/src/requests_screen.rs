use crate::app::{Message as AppMessage, RequestEntry, RequestStore, Screen, is_quit_key_event};
use crossterm::event::{Event, KeyCode};
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Margin},
    style::{Style, palette::tailwind},
    text::Text,
    widgets::{Cell, Row, Scrollbar, ScrollbarOrientation, ScrollbarState, Table, TableState},
};

#[derive(Debug, PartialEq)]
pub enum Message {
    UpdateTableState(Box<RequestEntryRow>),
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
    request_id: String,
    method: String,
    url: String,
    body: String,
    status: String,
}

impl From<&RequestEntry> for RequestEntryRow {
    fn from(entry: &RequestEntry) -> Self {
        let status = if let Some(response) = &entry.response {
            response.status.to_string()
        } else {
            "Pending".to_string()
        };

        RequestEntryRow {
            request_id: entry.request_id.to_string(),
            method: entry.request.method.to_string(),
            url: entry.request.url.to_string(),
            body: String::from_utf8_lossy(&entry.request.body).to_string(),
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

#[derive(Debug, Clone, Default, PartialEq)]
struct RequestTableColumnWidths {
    request_id: u16,
    method: u16,
    url: u16,
    body: u16,
    status: u16,
}

impl RequestTableColumnWidths {
    fn update(&mut self, row: RequestEntryRow) {
        self.request_id = self.request_id.max(row.request_id.len() as u16);
        self.method = self.method.max(row.method.len() as u16);
        self.url = self.url.max(row.url.len() as u16);
        self.body = self.body.max(row.body.len() as u16);
        self.status = self.status.max(row.status.len() as u16);
    }
}

pub struct RequestsScreen {
    request_store: RequestStore,
    column_widths: RequestTableColumnWidths,
    table_state: TableState,
    table_scroll_state: ScrollbarState,
}

impl RequestsScreen {
    const TABLE_COLUMN_REQ_ID: &str = "ReqId";
    const TABLE_COLUMN_METHOD: &str = "Method";
    const TABLE_COLUMN_URL: &str = "URL";
    const TABLE_COLUMN_BODY: &str = "Body";
    const TABLE_COLUMN_STATUS: &str = "Status";

    pub fn new(request_store: RequestStore) -> Self {
        let mut column_widths = RequestTableColumnWidths::default();
        column_widths.update(RequestEntryRow {
            request_id: Self::TABLE_COLUMN_REQ_ID.to_string(),
            method: Self::TABLE_COLUMN_METHOD.to_string(),
            url: Self::TABLE_COLUMN_URL.to_string(),
            body: Self::TABLE_COLUMN_BODY.to_string(),
            status: Self::TABLE_COLUMN_STATUS.to_string(),
        });
        Self {
            request_store,
            column_widths,
            table_state: TableState::default(),
            table_scroll_state: ScrollbarState::default(),
        }
    }
}

impl RequestsScreen {
    fn draw_table(&mut self, frame: &mut Frame) {
        let header = RequestEntryRow {
            request_id: Self::TABLE_COLUMN_REQ_ID.to_string(),
            method: Self::TABLE_COLUMN_METHOD.to_string(),
            url: Self::TABLE_COLUMN_URL.to_string(),
            body: Self::TABLE_COLUMN_BODY.to_string(),
            status: Self::TABLE_COLUMN_STATUS.to_string(),
        };
        let rows = match self.request_store.lock() {
            Ok(store) => store
                .values()
                .map(RequestEntryRow::from)
                .map(Row::from)
                .collect(),
            Err(_) => vec![],
        };

        let table = Table::default()
            .header(header.into())
            .widths([
                Constraint::Length(self.column_widths.request_id),
                Constraint::Length(self.column_widths.method),
                Constraint::Length(self.column_widths.url),
                Constraint::Min(self.column_widths.body),
                Constraint::Length(self.column_widths.status),
            ])
            .column_spacing(1)
            .rows(rows)
            .row_highlight_style(
                Style::default()
                    .bg(tailwind::GRAY.c100)
                    .fg(tailwind::GRAY.c900),
            );

        frame.render_stateful_widget(table, frame.area(), &mut self.table_state);
    }

    fn draw_scrollbar(&mut self, frame: &mut Frame) {
        frame.render_stateful_widget(
            Scrollbar::default()
                .orientation(ScrollbarOrientation::VerticalRight)
                .style(tailwind::GRAY.c400),
            frame.area().inner(Margin {
                vertical: 1,
                horizontal: 0,
            }),
            &mut self.table_scroll_state,
        );
    }

    fn update_table_state(&mut self, row: RequestEntryRow) {
        self.column_widths.update(row);

        if self.table_state.selected().is_none() {
            self.table_state.select_first();
        }
        self.table_scroll_state = self
            .table_scroll_state
            .content_length(self.requests_length());
    }

    fn select_previous_row(&mut self) {
        self.table_state.select_previous();
        self.table_scroll_state.prev();
    }

    fn select_next_row(&mut self) {
        self.table_state.select_next();
        self.table_scroll_state.next();
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
        self.draw_table(frame);
        self.draw_scrollbar(frame);
    }

    fn handle_event(&self, event: Event) -> Option<AppMessage> {
        if let Some(message) = is_quit_key_event(&event) {
            return Some(message);
        }

        if let Event::Key(key_event) = event {
            match key_event.code {
                KeyCode::Up | KeyCode::Char('k') => return Some(Message::SelectPreviousRow.into()),
                KeyCode::Down | KeyCode::Char('j') => return Some(Message::SelectNextRow.into()),
                _ => {}
            }
        }

        None
    }

    fn update(&mut self, message: AppMessage) -> Option<AppMessage> {
        let message = match message {
            AppMessage::RequestsScreen(message) => message,
            message => return Some(message),
        };

        match message {
            Message::UpdateTableState(row) => self.update_table_state(*row),
            Message::SelectPreviousRow => self.select_previous_row(),
            Message::SelectNextRow => self.select_next_row(),
        }
        None
    }
}
