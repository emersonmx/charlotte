use crate::app::{Message as AppMessage, RequestEntry, RequestStore, Screen, is_quit_key_event};
use crossterm::event::Event;
use ratatui::{
    Frame,
    layout::{Alignment, Constraint},
    text::Text,
    widgets::{Cell, Row, Table, TableState},
};

pub enum Message {
    UpdateTableColumnWidths(Box<RequestEntryRow>),
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
            RequestsScreen::ROW_STATUS_PENDING.to_string()
        };

        RequestEntryRow {
            request_id: entry.request_id.to_string(),
            method: entry.request.method.clone(),
            url: entry.request.url.clone(),
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
}

impl RequestsScreen {
    const TABLE_COLUMN_REQ_ID: &str = "ReqId";
    const TABLE_COLUMN_METHOD: &str = "Method";
    const TABLE_COLUMN_URL: &str = "URL";
    const TABLE_COLUMN_BODY: &str = "Body";
    const TABLE_COLUMN_STATUS: &str = "Status";
    const ROW_STATUS_PENDING: &str = "Pending";

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
            table_state: TableState::default().with_selected(Some(0)),
        }
    }
}

impl Screen for RequestsScreen {
    fn draw(&mut self, frame: &mut Frame) {
        let header = RequestEntryRow {
            request_id: Self::TABLE_COLUMN_REQ_ID.to_string(),
            method: Self::TABLE_COLUMN_METHOD.to_string(),
            url: Self::TABLE_COLUMN_URL.to_string(),
            body: Self::TABLE_COLUMN_BODY.to_string(),
            status: Self::TABLE_COLUMN_STATUS.to_string(),
        };
        let rows: Vec<RequestEntryRow> = match self.request_store.lock() {
            Ok(store) => store.values().map(|entry| entry.into()).collect(),
            Err(_) => vec![],
        };

        let table = Table::new(
            rows,
            [
                Constraint::Length(self.column_widths.request_id + 2),
                Constraint::Length(self.column_widths.method + 2),
                Constraint::Length(self.column_widths.url + 2),
                Constraint::Min(self.column_widths.body + 2),
                Constraint::Length(self.column_widths.status + 2),
            ],
        )
        .header(header.into());
        frame.render_stateful_widget(table, frame.area(), &mut self.table_state);
    }

    fn handle_event(&mut self, event: Event) -> Option<AppMessage> {
        is_quit_key_event(event)
    }

    fn update(&mut self, message: AppMessage) -> Option<AppMessage> {
        let message = match message {
            AppMessage::RequestsScreen(message) => message,
            message => return Some(message),
        };

        match message {
            Message::UpdateTableColumnWidths(row) => {
                self.column_widths.update(*row);
            }
        }
        None
    }
}
