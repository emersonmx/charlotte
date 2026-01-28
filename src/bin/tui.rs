use crossterm::event::{self, Event, EventStream};
use ratatui::{
    DefaultTerminal, Frame,
    buffer::Buffer,
    layout::Rect,
    style::Stylize,
    symbols::border,
    text::{Line, Text},
    widgets::{Block, Paragraph, Widget},
};
use tokio::sync::mpsc;
use tokio_stream::StreamExt;

#[derive(Debug)]
enum Command {
    Start,
    Stop,
    Restart,
    Quit,
}

#[derive(Debug, Default)]
struct App {
    counter: u64,
    running: bool,
}

impl App {
    async fn run(
        &mut self,
        terminal: &mut DefaultTerminal,
        mut counter_rx: mpsc::Receiver<u64>,
        cmd_tx: mpsc::Sender<Command>,
    ) -> anyhow::Result<()> {
        self.running = true;
        let mut events = EventStream::new();

        while self.running {
            terminal.draw(|frame: &mut Frame| self.draw(frame))?;

            tokio::select! {
                Some(Ok(event)) = events.next() => self.handle_events(&event, &cmd_tx).await,
                Some(value) = counter_rx.recv() => self.counter = value,
            }
        }

        Ok(())
    }

    async fn handle_events(&mut self, event: &Event, cmd_tx: &mpsc::Sender<Command>) {
        if let event::Event::Key(event) = event {
            let cmd = match event.code {
                event::KeyCode::Char('s') => Some(Command::Start),
                event::KeyCode::Char('t') => Some(Command::Stop),
                event::KeyCode::Char('r') => Some(Command::Restart),
                event::KeyCode::Char('q') => {
                    self.running = false;
                    Some(Command::Quit)
                }
                _ => None,
            };

            if let Some(command) = cmd {
                let _ = cmd_tx.send(command).await;
            }
        }
    }

    fn draw(&self, frame: &mut Frame) {
        frame.render_widget(self, frame.area());
    }
}

impl Widget for &App {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let title = Line::from(" Async Counter Application ");
        let instructions = Line::from(vec![
            " Start ".into(),
            "<S>".blue().bold(),
            " Stop ".into(),
            "<T>".blue().bold(),
            " Restart ".into(),
            "<R>".blue().bold(),
            " Quit ".into(),
            "<q>".blue().bold(),
        ]);
        let block = Block::bordered()
            .title(title.centered())
            .title_bottom(instructions.centered())
            .border_set(border::THICK);

        let counter_text = Text::from(vec![Line::from(vec![
            "Value: ".into(),
            self.counter.to_string().yellow(),
        ])]);

        Paragraph::new(counter_text)
            .centered()
            .block(block)
            .render(area, buf);
    }
}

async fn counter_task(mut cmd_rx: mpsc::Receiver<Command>, counter_tx: mpsc::Sender<u64>) {
    const INTERVAL: std::time::Duration = std::time::Duration::from_secs(1);

    let mut counter = 0u64;
    let mut paused = true;
    let mut ticker = tokio::time::interval(INTERVAL);

    let _ = counter_tx.send(counter).await;

    loop {
        tokio::select! {
            _ = ticker.tick() => {
                if !paused {
                    counter += INTERVAL.as_secs();
                    let _ = counter_tx.send(counter).await;
                }
            }
            cmd = cmd_rx.recv() => {
                if let Some(cmd) = cmd {
                    match cmd {
                        Command::Start => paused = false,
                        Command::Stop => paused = true,
                        Command::Restart => {
                            counter = 0;
                            let _ = counter_tx.send(counter).await;
                        }
                        Command::Quit  => break,
                    }
                } else {
                    break;
                }
            }
        };
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let (cmd_tx, cmd_rx) = mpsc::channel::<Command>(32);
    let (counter_tx, counter_rx) = mpsc::channel::<u64>(32);

    tokio::spawn(async move {
        counter_task(cmd_rx, counter_tx).await;
    });

    let mut terminal = ratatui::init();
    let app_result = App::default().run(&mut terminal, counter_rx, cmd_tx).await;
    ratatui::restore();
    app_result
}
