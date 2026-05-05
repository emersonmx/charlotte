use crate::app::{Message as AppMessage, RequestEntry, Screen, is_quit_key_event};
use crossterm::event::{Event, KeyCode};
use proxy::http::HeaderMap;
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Margin, Rect},
    widgets::{Block, Cell, Paragraph, Row, Table},
};
use std::fmt::Display;

#[derive(Debug, PartialEq)]
pub enum Message {
    NextTab,
    PreviousTab,
}

impl From<Message> for AppMessage {
    fn from(message: Message) -> Self {
        AppMessage::HttpClientScreen(message)
    }
}

#[derive(Debug, Clone, PartialEq)]
enum Tab {
    Request,
    Response,
}

impl Tab {
    fn toggle(&mut self) {
        *self = match self {
            Tab::Request => Tab::Response,
            Tab::Response => Tab::Request,
        };
    }
}

impl Display for Tab {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Tab::Request => write!(f, "Request"),
            Tab::Response => write!(f, "Response"),
        }
    }
}

pub struct HttpClientScreen {
    request_entry: Option<RequestEntry>,
    tab_selected: Tab,
}

impl HttpClientScreen {
    const METHOD_LABEL: &str = "Method";
    const URL_LABEL: &str = "URL";
    const QUERY_PARAMS_LABEL: &str = "Query Parameters";
    const HEADERS_LABEL: &str = "Headers";
    const BODY_LABEL: &str = "Body";

    pub fn new(request_entry: Option<RequestEntry>) -> Self {
        Self {
            request_entry,
            tab_selected: Tab::Request,
        }
    }

    fn draw_empty(&mut self, frame: &mut Frame) {
        let message = "No request selected. Please select a request from the Requests Screen.";
        let block = Block::bordered();
        let paragraph = Paragraph::new(message).centered().block(block);
        frame.render_widget(paragraph, frame.area());
    }

    fn draw_method_url(&self, frame: &mut Frame, area: Rect, request_entry: &RequestEntry) {
        let method_paragraph = Paragraph::new(request_entry.request.method.to_string())
            .block(Block::bordered().title(Self::METHOD_LABEL));
        let url_paragraph = Paragraph::new(request_entry.request.url.to_string())
            .block(Block::bordered().title(Self::URL_LABEL));
        let layout = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Length(20), Constraint::Min(0)]);
        let [method_area, url_area] = area.layout(&layout);
        frame.render_widget(method_paragraph, method_area);
        frame.render_widget(url_paragraph, url_area);
    }

    fn draw_query_params(&self, frame: &mut Frame, area: Rect, request_entry: &RequestEntry) {
        let mut width = [0, 0];
        let query_string = request_entry.request.url.query().unwrap_or_default();
        let params: Vec<Row> = query_string
            .split('&')
            .filter_map(|param| {
                let mut parts = param.splitn(2, '=');
                let key = parts.next()?;
                let value = parts.next().unwrap_or_default();
                width[0] = width[0].max(key.chars().count() as u16);
                width[1] = width[1].max(value.chars().count() as u16);
                Some(Row::new(vec![Cell::from(key), Cell::from(value)]))
            })
            .collect();
        let params_table = Table::default()
            .rows(params)
            .block(Block::bordered().title(format!(" {} ", Self::QUERY_PARAMS_LABEL)))
            .widths(vec![
                Constraint::Length(width[0]),
                Constraint::Min(width[1]),
            ])
            .column_spacing(3);
        frame.render_widget(params_table, area);
    }

    fn draw_headers(&self, frame: &mut Frame, area: Rect, headers: HeaderMap) {
        let mut widths = [0, 0];
        let headers: Vec<Row> = headers
            .iter()
            .map(|header| {
                let key = header.key().to_string();
                let value = header.value().unwrap_or_default().to_string();
                widths[0] = widths[0].max(key.chars().count() as u16);
                widths[1] = widths[1].max(value.chars().count() as u16);
                Row::new(vec![Cell::from(key), Cell::from(value)])
            })
            .collect();
        let header_table = Table::default()
            .rows(headers)
            .block(Block::bordered().title(format!(" {} ", Self::HEADERS_LABEL)))
            .widths(vec![
                Constraint::Length(widths[0]),
                Constraint::Min(widths[1]),
            ])
            .column_spacing(3);
        frame.render_widget(header_table, area);
    }

    fn draw_body(&self, frame: &mut Frame, area: Rect, body: &[u8]) {
        let body_string = String::from_utf8_lossy(body);
        let paragraph = Paragraph::new(body_string)
            .block(Block::bordered().title(format!(" {} ", Self::BODY_LABEL)));
        frame.render_widget(paragraph, area);
    }
}

impl Screen for HttpClientScreen {
    fn draw(&mut self, frame: &mut Frame) {
        let request_entry = match &self.request_entry {
            Some(entry) => entry,
            None => {
                self.draw_empty(frame);
                return;
            }
        };

        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(3), Constraint::Min(3)]);
        let [method_url_area, tabbed_pane_area] = frame.area().layout(&layout);

        self.draw_method_url(frame, method_url_area, request_entry);

        let tab_titles: Vec<String> = [Tab::Request, Tab::Response]
            .iter()
            .map(|tab| {
                if self.tab_selected == *tab {
                    format!("[{tab}]")
                } else {
                    format!(" {tab} ")
                }
            })
            .collect();
        let tabs = Block::bordered().title(format!(" {} ", tab_titles.join("")));

        match self.tab_selected {
            Tab::Request => {
                let layout = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([
                        Constraint::Length(10),
                        Constraint::Length(10),
                        Constraint::Min(0),
                    ]);
                let [params_area, headers_area, body_area] = tabbed_pane_area
                    .inner(Margin {
                        vertical: 1,
                        horizontal: 1,
                    })
                    .layout(&layout);

                frame.render_widget(tabs, tabbed_pane_area);
                self.draw_query_params(frame, params_area, request_entry);
                self.draw_headers(frame, headers_area, request_entry.request.headers.clone());
                self.draw_body(frame, body_area, request_entry.request.body.as_bytes());
            }
            Tab::Response => {
                let layout = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([Constraint::Length(10), Constraint::Min(0)]);
                let [headers_area, body_area] = tabbed_pane_area
                    .inner(Margin {
                        vertical: 1,
                        horizontal: 1,
                    })
                    .layout(&layout);

                frame.render_widget(tabs, tabbed_pane_area);

                let headers = match &request_entry.response {
                    Some(response) => response.headers.clone(),
                    None => HeaderMap::new(vec![]),
                };
                self.draw_headers(frame, headers_area, headers);
                self.draw_body(
                    frame,
                    body_area,
                    request_entry
                        .response
                        .as_ref()
                        .map(|response| response.body.as_bytes())
                        .unwrap_or_default(),
                );
            }
        }
    }

    fn handle_event(&self, event: Event) -> Option<AppMessage> {
        if let Some(message) = is_quit_key_event(&event) {
            return Some(message);
        }

        if let Event::Key(key_event) = event {
            match key_event.code {
                KeyCode::Char('[') => {
                    return Some(Message::PreviousTab.into());
                }
                KeyCode::Char(']') => {
                    return Some(Message::NextTab.into());
                }
                _ => {}
            }
        }

        None
    }

    fn update(&mut self, message: AppMessage) -> Option<AppMessage> {
        let message = match message {
            AppMessage::HttpClientScreen(message) => message,
            _ => return Some(message),
        };

        match message {
            Message::PreviousTab | Message::NextTab => {
                self.tab_selected.toggle();
            }
        }

        None
    }
}
