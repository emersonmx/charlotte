use crate::{
    app::{Action, Event, Screen, ScreenId},
    proxy,
};
use async_trait::async_trait;
use crossterm::event::KeyCode;
use ratatui::{
    Frame,
    layout::{Alignment, Constraint},
    text::Text,
    widgets::{Cell, Row, Table, TableState},
};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Default, PartialEq)]
pub struct RequestEntry {
    pub request_id: proxy::RequestId,
    pub request: proxy::Request,
    pub response: Option<proxy::Response>,
}

#[derive(Debug, Clone, Default, PartialEq)]
struct RequestEntryRow {
    request_id: String,
    method: String,
    url: String,
    body: String,
    status: String,
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

#[derive(Debug)]
pub struct RequestsScreen {
    requests: BTreeMap<proxy::RequestId, RequestEntry>,
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

    pub fn new() -> Self {
        let mut column_widths = RequestTableColumnWidths::default();
        column_widths.update(RequestEntryRow {
            request_id: Self::TABLE_COLUMN_REQ_ID.to_string(),
            method: Self::TABLE_COLUMN_METHOD.to_string(),
            url: Self::TABLE_COLUMN_URL.to_string(),
            body: Self::TABLE_COLUMN_BODY.to_string(),
            status: Self::TABLE_COLUMN_STATUS.to_string(),
        });
        Self {
            requests: BTreeMap::new(),
            column_widths,
            table_state: TableState::default().with_selected(Some(0)),
        }
    }
}

#[async_trait]
impl Screen for RequestsScreen {
    fn id(&self) -> ScreenId {
        ScreenId::Requests
    }

    async fn handle_event(&mut self, event: &Event) -> Option<Action> {
        match event {
            Event::CrosstermEvent(event) => {
                if let crossterm::event::Event::Key(key_event) = event {
                    if let KeyCode::Char('q') = key_event.code {
                        return Some(Action::Exit);
                    }
                }
            }
            Event::ProxyMessage(message) => match message.as_ref() {
                proxy::Message::RequestSent((request_id, request)) => {
                    let request_entry = RequestEntry {
                        request_id: *request_id,
                        request: request.clone(),
                        response: None,
                    };
                    self.column_widths.update(RequestEntryRow {
                        request_id: request_id.to_string(),
                        method: request.method.clone(),
                        url: request.url.clone(),
                        body: String::from_utf8_lossy(&request.body).to_string(),
                        status: Self::ROW_STATUS_PENDING.to_string(),
                    });
                    self.requests.insert(*request_id, request_entry);
                }
                proxy::Message::ResponseReceived((request_id, response)) => {
                    if let Some(request_entry) = self.requests.get_mut(request_id) {
                        self.column_widths.update(RequestEntryRow {
                            request_id: request_id.to_string(),
                            method: request_entry.request.method.clone(),
                            url: request_entry.request.url.clone(),
                            body: String::from_utf8_lossy(&request_entry.request.body).to_string(),
                            status: response.status.to_string(),
                        });
                        request_entry.response = Some(response.clone());
                    }
                }
                _ => {}
            },
        };

        None
    }

    fn draw(&mut self, frame: &mut Frame) {
        let header = RequestEntryRow {
            request_id: Self::TABLE_COLUMN_REQ_ID.to_string(),
            method: Self::TABLE_COLUMN_METHOD.to_string(),
            url: Self::TABLE_COLUMN_URL.to_string(),
            body: Self::TABLE_COLUMN_BODY.to_string(),
            status: Self::TABLE_COLUMN_STATUS.to_string(),
        };
        let rows = self.requests.values().map(|entry| {
            let status = if let Some(response) = &entry.response {
                response.status.to_string()
            } else {
                Self::ROW_STATUS_PENDING.to_string()
            };

            RequestEntryRow {
                request_id: entry.request_id.to_string(),
                method: entry.request.method.clone(),
                url: entry.request.url.clone(),
                body: String::from_utf8_lossy(&entry.request.body).to_string(),
                status,
            }
        });

        let table = Table::new(
            rows,
            &[
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
}
