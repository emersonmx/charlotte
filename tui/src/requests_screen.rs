use crate::{
    app::{Message as AppMessage, RequestEntry, RequestStore, Screen, is_quit_key_event},
    theme,
};
use crossterm::event::{Event, KeyCode};
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
            body: String::from_utf8_lossy(entry.request.body.as_bytes()).to_string(),
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

    fn to_table_widths(&self) -> [Constraint; 5] {
        [
            Constraint::Length(self.request_id),
            Constraint::Length(self.method),
            Constraint::Length(self.url),
            Constraint::Min(self.body),
            Constraint::Length(self.status),
        ]
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
    fn update_table_state(&mut self, row: RequestEntryRow) -> Option<AppMessage> {
        self.column_widths.update(row);

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
        let layout = Layout::default()
            .direction(Vertical)
            .constraints([Constraint::Length(3), Constraint::Min(3)]);
        let [header_layout, rows_layout] = frame.area().layout(&layout);
        let table_widths = self.column_widths.to_table_widths();

        let header = RequestEntryRow {
            request_id: Self::TABLE_COLUMN_REQ_ID.to_string(),
            method: Self::TABLE_COLUMN_METHOD.to_string(),
            url: Self::TABLE_COLUMN_URL.to_string(),
            body: Self::TABLE_COLUMN_BODY.to_string(),
            status: Self::TABLE_COLUMN_STATUS.to_string(),
        };
        let border_set = symbols::border::Set {
            bottom_left: symbols::line::VERTICAL_RIGHT,
            bottom_right: symbols::line::VERTICAL_LEFT,
            ..symbols::border::PLAIN
        };
        let table = Table::default()
            .header(header.into())
            .block(Block::bordered().border_set(border_set))
            .widths(table_widths);
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
            .widths(table_widths)
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
    }

    fn handle_event(&self, event: Event) -> Option<AppMessage> {
        if let Some(message) = is_quit_key_event(&event) {
            return Some(message);
        }

        if let Event::Key(key_event) = event {
            match key_event.code {
                KeyCode::Up | KeyCode::Char('k') => return Some(Message::SelectPreviousRow.into()),
                KeyCode::Down | KeyCode::Char('j') => return Some(Message::SelectNextRow.into()),
                KeyCode::Enter | KeyCode::Char('l') => {
                    let selected = self.table_state.selected()?;
                    let request_entry = match self.request_store.lock() {
                        Ok(store) => store.values().nth(selected).cloned(),
                        Err(_) => return None,
                    };

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
            Message::UpdateTableState(row) => self.update_table_state(*row),
            Message::SelectPreviousRow => self.select_previous_row(),
            Message::SelectNextRow => self.select_next_row(),
        }
    }
}
