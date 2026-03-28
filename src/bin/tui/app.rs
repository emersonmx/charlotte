use crossterm::event::{self, Event, EventStream};
use ratatui::{
    DefaultTerminal, Frame,
    text::Text,
    widgets::{ScrollbarState, TableState, Widget},
};
use tokio_stream::StreamExt;

const REQUEST_TABLE_COLUMN_REQ_ID: &str = "ReqId";
const REQUEST_TABLE_COLUMN_METHOD: &str = "Method";
const REQUEST_TABLE_COLUMN_URL: &str = "URL";
const REQUEST_TABLE_COLUMN_BODY: &str = "Body";

#[derive(Debug, Clone, Default, PartialEq)]
struct RequestEntry {
    request_id: usize,
    method: String,
    url: String,
    headers: Vec<(String, String)>,
    body: Vec<u8>,
}

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
pub struct App {
    running: bool,
    requests: Vec<RequestEntry>,
    column_widths: RequestTableColumnWidths,
    table_state: TableState,
    table_scroll_state: ScrollbarState,
}

impl App {
    pub fn new() -> Self {
        let mut column_widths = RequestTableColumnWidths::default();
        column_widths.update(&RequestEntryRow {
            request_id: REQUEST_TABLE_COLUMN_REQ_ID.to_string(),
            method: REQUEST_TABLE_COLUMN_METHOD.to_string(),
            url: REQUEST_TABLE_COLUMN_URL.to_string(),
            body: REQUEST_TABLE_COLUMN_BODY.to_string(),
        });

        Self {
            running: false,
            requests: Vec::new(),
            column_widths,
            table_state: TableState::default().with_selected(Some(0)),
            table_scroll_state: ScrollbarState::default().content_length(0),
        }
    }

    pub async fn run(&mut self, terminal: &mut DefaultTerminal) -> anyhow::Result<()> {
        self.running = true;
        let mut events = EventStream::new();

        while self.running {
            terminal.draw(|frame: &mut Frame| self.draw(frame))?;

            tokio::select! {
                Some(Ok(event)) = events.next() => self.handle_events(&event).await?,
            }
        }

        Ok(())
    }

    async fn handle_events(&mut self, event: &Event) -> anyhow::Result<()> {
        #[allow(clippy::single_match)]
        match event {
            Event::Key(key_event) => self.handle_key_event(key_event).await,
            _ => {}
        }

        Ok(())
    }

    async fn handle_key_event(&mut self, event: &event::KeyEvent) {
        if let event::KeyCode::Char('q') = event.code {
            self.running = false;
        }
    }

    fn draw(&self, frame: &mut Frame) {
        self.show_waiting_screen(frame);
    }

    fn show_waiting_screen(&self, frame: &mut Frame) {
        let text = Text::raw("Waiting for requests on port 8888... (press 'q' to quit)").centered();
        text.render(frame.area(), frame.buffer_mut());
    }

    fn show_requests_screen(&self, frame: &mut Frame) {}
}
