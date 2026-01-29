use crossterm::event::{self, Event, EventStream};
use ratatui::{
    DefaultTerminal, Frame,
    buffer::Buffer,
    layout::{Constraint, Layout, Rect},
    text::Line,
    widgets::{Cell, Paragraph, Row, Table, Widget},
};
use tokio_stream::StreamExt;

#[derive(Debug, Default)]
struct App {
    running: bool,
}

impl App {
    async fn run(&mut self, terminal: &mut DefaultTerminal) -> anyhow::Result<()> {
        self.running = true;
        let mut events = EventStream::new();

        while self.running {
            terminal.draw(|frame: &mut Frame| self.draw(frame))?;

            tokio::select! {
                Some(Ok(event)) = events.next() => self.handle_events(&event).await,
            }
        }

        Ok(())
    }

    async fn handle_events(&mut self, event: &Event) {
        if let event::Event::Key(event) = event
            && let event::KeyCode::Char('q') = event.code
        {
            self.running = false;
        }
    }

    fn draw(&self, frame: &mut Frame) {
        frame.render_widget(self, frame.area());
    }
}

impl Widget for &App {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let layout = Layout::default()
            .direction(ratatui::layout::Direction::Vertical)
            .constraints([Constraint::Max(5), Constraint::Min(5)].as_ref())
            .split(area);

        Paragraph::new(vec![
            Line::raw("Listening on 0.0.0.0:8888"),
            Line::raw("Press 'q' to quit."),
        ])
        .render(layout[0], buf);

        let table_header = ["ReqId", "Method", "URL", "Status"]
            .into_iter()
            .map(Cell::from)
            .collect::<Row>();

        let table_rows = [
            ["1", "GET", "http://example.com", "200"],
            ["2", "POST", "http://example.com/api", "201"],
        ]
        .into_iter()
        .map(|row| row.into_iter().map(Cell::from).collect::<Row>());

        Table::new(
            table_rows,
            [
                Constraint::Min(8),
                Constraint::Length(8),
                Constraint::Percentage(100),
                Constraint::Length(8),
            ],
        )
        .header(table_header)
        .render(layout[1], buf);
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let mut terminal = ratatui::init();
    let app_result = App::default().run(&mut terminal).await;
    ratatui::restore();
    app_result
}
