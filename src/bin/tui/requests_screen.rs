use crate::app::{Action, Event};
use crate::data::RequestEntry;
use crate::navigation::{Screen, ScreenId};
use async_trait::async_trait;
use crossterm::event::KeyCode;
use ratatui::widgets::{ScrollbarState, TableState};
use ratatui::{Frame, text::Text};

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
    requests: Vec<RequestEntry>,
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
            requests: vec![],
            column_widths,
            table_state: TableState::default().with_selected(Some(0)),
            table_scroll_state: ScrollbarState::default().content_length(0),
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
                    if let KeyCode::Char('b') = key_event.code {
                        return Some(Action::GoBack);
                    }
                }
            }
            Event::ProxyMessage(_) => {}
        };

        None
    }

    fn draw(&self, frame: &mut Frame) {
        let text = Text::raw("REQUESTS SCREEN").centered();
        frame.render_widget(text, frame.area());
    }
}
