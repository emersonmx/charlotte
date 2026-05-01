use crate::app::{Message as AppMessage, RequestEntry, RequestStore, Screen, is_quit_key_event};
use crossterm::event::{Event, KeyCode};
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Margin},
    style::palette::tailwind,
    text::Text,
    widgets::{Cell, Row, Scrollbar, ScrollbarOrientation, ScrollbarState, Table, TableState},
};

pub enum Message {
    UpdateTableColumnWidths(Box<RequestEntryRow>),
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
    table_scroll_state: ScrollbarState,
}

impl RequestsScreen {
    const TABLE_SCROLL_CONTENT_LENGTH: usize = 50;
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
            table_scroll_state: ScrollbarState::default()
                .content_length(Self::TABLE_SCROLL_CONTENT_LENGTH),
        }
    }
}

impl RequestsScreen {
    fn draw_table(&mut self, frame: &mut Frame) {
        let area = frame.area();
        let requests_len = match self.request_store.lock() {
            Ok(store) => store.len(),
            Err(_) => 0,
        };

        let visible_rows = area.height.saturating_sub(1) as usize;
        let selected = self.table_state.selected().unwrap_or(0);
        let start = if selected >= visible_rows {
            selected - visible_rows + 1
        } else {
            0
        };
        let end = (start + visible_rows).min(requests_len);
        let mut visible_state = TableState::default();
        visible_state.select(Some(selected - start));

        let header = RequestEntryRow {
            request_id: Self::TABLE_COLUMN_REQ_ID.to_string(),
            method: Self::TABLE_COLUMN_METHOD.to_string(),
            url: Self::TABLE_COLUMN_URL.to_string(),
            body: Self::TABLE_COLUMN_BODY.to_string(),
            status: Self::TABLE_COLUMN_STATUS.to_string(),
        };
        let rows: Vec<RequestEntryRow> = match self.request_store.lock() {
            Ok(store) => store
                .range(start..=end)
                .map(|(_, entry)| entry.into())
                .collect(),
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
        .header(header.into())
        .row_highlight_style(tailwind::BLUE.c400);

        frame.render_stateful_widget(table, frame.area(), &mut self.table_state);
    }

    fn draw_scrollbar(&mut self, frame: &mut Frame) {
        let area = frame.area();
        frame.render_stateful_widget(
            Scrollbar::default()
                .orientation(ScrollbarOrientation::VerticalRight)
                .style(tailwind::GRAY.c400),
            area.inner(Margin {
                vertical: 1,
                horizontal: 1,
            }),
            &mut self.table_scroll_state,
        );
    }

    fn select_previous_row(&mut self) {
        let selected = self.table_state.selected().unwrap_or(0);
        if selected > 0 {
            self.table_state.select(Some(selected - 1));
        }
    }

    fn select_next_row(&mut self) {
        let len = match self.request_store.lock() {
            Ok(store) => store.len(),
            Err(_) => 0,
        };
        let selected = self.table_state.selected().unwrap_or(0);
        if selected < len - 1 {
            self.table_state.select(Some(selected + 1));
        }
    }
}

impl Screen for RequestsScreen {
    fn draw(&mut self, frame: &mut Frame) {
        self.draw_table(frame);
        self.draw_scrollbar(frame);
    }

    fn handle_event(&mut self, event: Event) -> Option<AppMessage> {
        if let Some(message) = is_quit_key_event(&event) {
            return Some(message);
        }

        if let Event::Key(key_event) = event {
            match key_event.code {
                KeyCode::Up => return Some(Message::SelectPreviousRow.into()),
                KeyCode::Down => return Some(Message::SelectNextRow.into()),
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
            Message::UpdateTableColumnWidths(row) => {
                self.column_widths.update(*row);
            }
            Message::SelectPreviousRow => self.select_previous_row(),
            Message::SelectNextRow => self.select_next_row(),
        }
        None
    }
}
