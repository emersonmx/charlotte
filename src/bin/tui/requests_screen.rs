use crate::app::{ControlFlow, Screen};
use async_trait::async_trait;
use crossterm::event::{self, Event};
use ratatui::widgets::{ScrollbarState, TableState};
use ratatui::{Frame, text::Text, widgets::Widget};

#[derive(Debug, Clone, Default, PartialEq)]
struct RequestEntryRow {
    request_id: String,
    method: String,
    url: String,
    body: String,
}

#[derive(Debug, Clone, Default, PartialEq)]
struct RequestTableColumnWidths {
    request_id: usize,
    method: usize,
    url: usize,
    body: usize,
}

impl RequestTableColumnWidths {
    fn update(&mut self, row: &RequestEntryRow) {
        self.request_id = self.request_id.max(row.request_id.len());
        self.method = self.method.max(row.method.len());
        self.url = self.url.max(row.url.len());
        self.body = self.body.max(row.body.len());
    }
}

#[derive(Debug)]
pub struct RequestsScreen {
    column_widths: RequestTableColumnWidths,
    table_state: TableState,
    table_scroll_state: ScrollbarState,
}

impl RequestsScreen {
    const TABLE_COLUMN_REQ_ID: &str = "ReqId";
    const TABLE_COLUMN_METHOD: &str = "Method";
    const TABLE_COLUMN_URL: &str = "URL";
    const TABLE_COLUMN_BODY: &str = "Body";

    pub fn new() -> Self {
        let mut column_widths = RequestTableColumnWidths::default();
        column_widths.update(&RequestEntryRow {
            request_id: Self::TABLE_COLUMN_REQ_ID.to_string(),
            method: Self::TABLE_COLUMN_METHOD.to_string(),
            url: Self::TABLE_COLUMN_URL.to_string(),
            body: Self::TABLE_COLUMN_BODY.to_string(),
        });
        Self {
            column_widths,
            table_state: TableState::default().with_selected(Some(0)),
            table_scroll_state: ScrollbarState::default().content_length(0),
        }
    }
}

#[async_trait]
impl Screen for RequestsScreen {
    async fn handle_events(&mut self, event: &Event) -> ControlFlow {
        #[allow(clippy::single_match)]
        match event {
            Event::Key(key_event) => {
                if let event::KeyCode::Char('q') = key_event.code {
                    return ControlFlow::Break;
                }
            }
            _ => {}
        };

        ControlFlow::Continue
    }

    fn draw(&self, frame: &mut Frame) {
        let text = Text::raw("Waiting for requests on port 8888... (press 'q' to quit)").centered();
        text.render(frame.area(), frame.buffer_mut());
    }
}
