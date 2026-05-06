use crate::app::{Message as AppMessage, RequestEntry, Screen, is_quit_key_event};
use crossterm::event::{Event, KeyCode};
use proxy::http::HeaderMap;
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Margin, Rect},
    style::{Style, palette::tailwind},
    symbols,
    text::{Line, Span},
    widgets::{Block, Cell, Paragraph, Row, Table},
};
use std::fmt::Display;

#[derive(Debug, PartialEq)]
pub enum Message {
    NextTab,
    PreviousTab,
    SelectSection(usize),
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

impl Display for Tab {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Tab::Request => write!(f, "Request"),
            Tab::Response => write!(f, "Response"),
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
enum Section {
    #[default]
    Method,
    Url,
    QueryParams,
    Headers,
    Body,
}

impl From<RequestSection> for Section {
    fn from(value: RequestSection) -> Self {
        match value {
            RequestSection::QueryParams => Section::QueryParams,
            RequestSection::Headers => Section::Headers,
            RequestSection::Body => Section::Body,
        }
    }
}

impl From<ResponseSection> for Section {
    fn from(value: ResponseSection) -> Self {
        match value {
            ResponseSection::Headers => Section::Headers,
            ResponseSection::Body => Section::Body,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
enum RequestSection {
    #[default]
    QueryParams,
    Headers,
    Body,
}
impl From<usize> for RequestSection {
    fn from(value: usize) -> Self {
        match value {
            3 => RequestSection::QueryParams,
            4 => RequestSection::Headers,
            5 => RequestSection::Body,
            _ => RequestSection::default(),
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
enum ResponseSection {
    #[default]
    Headers,
    Body,
}

impl From<usize> for ResponseSection {
    fn from(value: usize) -> Self {
        match value {
            3 => ResponseSection::Headers,
            4 => ResponseSection::Body,
            _ => ResponseSection::default(),
        }
    }
}

pub struct HttpClientScreen {
    request_entry: Option<RequestEntry>,
    tab_selected: Tab,
    section_selected: Section,
    request_section_selected: RequestSection,
    response_section_selected: ResponseSection,
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
            section_selected: Section::default(),
            request_section_selected: RequestSection::default(),
            response_section_selected: ResponseSection::default(),
        }
    }

    fn draw_empty(&mut self, frame: &mut Frame) {
        let message = "No request selected. Please select a request from the Requests Screen.";
        let block = Block::bordered();
        let paragraph = Paragraph::new(message).centered().block(block);
        frame.render_widget(paragraph, frame.area());
    }

    fn draw_method_url(&self, frame: &mut Frame, area: Rect) {
        let layout = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Length(20), Constraint::Min(0)]);
        let [method_area, url_area] = area.layout(&layout);

        self.draw_method(frame, method_area);
        self.draw_url(frame, url_area);
    }

    fn draw_method(&self, frame: &mut Frame, area: Rect) {
        let text = match &self.request_entry {
            Some(entry) => entry.request.method.to_string(),
            None => "".to_string(),
        };

        let mut block = Block::bordered().title(format!(
            "[1]{}{}",
            symbols::line::HORIZONTAL,
            Self::METHOD_LABEL
        ));
        if self.section_selected == Section::Method {
            block = block.style(Style::default().fg(tailwind::BLUE.c400));
        }

        let paragraph = Paragraph::new(text).block(block);

        frame.render_widget(paragraph, area);
    }
    fn draw_url(&self, frame: &mut Frame, area: Rect) {
        let text = match &self.request_entry {
            Some(entry) => entry.request.url.to_string(),
            None => "".to_string(),
        };

        let mut block = Block::bordered().title(format!(
            "[2]{}{}",
            symbols::line::HORIZONTAL,
            Self::URL_LABEL
        ));
        if self.section_selected == Section::Url {
            block = block.style(Style::default().fg(tailwind::BLUE.c400));
        }

        let paragraph = Paragraph::new(text).block(block);

        frame.render_widget(paragraph, area);
    }

    fn draw_tabs(&self, frame: &mut Frame, area: Rect) {
        let tab_titles: Vec<Span> = [Tab::Request, Tab::Response]
            .iter()
            .map(|tab| {
                if self.tab_selected == *tab {
                    Span::styled(
                        format!(" {tab} "),
                        Style::default()
                            .bg(tailwind::GRAY.c100)
                            .fg(tailwind::GRAY.c900),
                    )
                } else {
                    Span::styled(format!(" {tab} "), Style::reset())
                }
            })
            .collect();

        let title = Line::from(tab_titles);
        let tabs = Block::bordered().title(title);
        frame.render_widget(tabs, area);
    }

    fn draw_query_params(&self, frame: &mut Frame, area: Rect, select_index: usize) {
        let request_entry = match &self.request_entry {
            Some(entry) => entry,
            None => return,
        };

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
        let title = Line::from(format!(
            "[{}]{}{}",
            select_index,
            symbols::line::HORIZONTAL,
            Self::QUERY_PARAMS_LABEL
        ));
        let mut block = Block::bordered().title(title);
        if self.section_selected == Section::QueryParams {
            block = block.style(Style::default().fg(tailwind::BLUE.c400));
        }
        let params_table = Table::default()
            .rows(params)
            .block(block)
            .widths(vec![
                Constraint::Length(width[0]),
                Constraint::Min(width[1]),
            ])
            .column_spacing(3);
        frame.render_widget(params_table, area);
    }

    fn draw_headers(&self, frame: &mut Frame, area: Rect, headers: HeaderMap, select_index: usize) {
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
        let title = Line::from(format!(
            "[{}]{}{}",
            select_index,
            symbols::line::HORIZONTAL,
            Self::HEADERS_LABEL
        ));
        let mut block = Block::bordered().title(title);
        if self.section_selected == Section::Headers {
            block = block.style(Style::default().fg(tailwind::BLUE.c400));
        }
        let header_table = Table::default()
            .rows(headers)
            .block(block)
            .widths(vec![
                Constraint::Length(widths[0]),
                Constraint::Min(widths[1]),
            ])
            .column_spacing(3);
        frame.render_widget(header_table, area);
    }

    fn draw_body(&self, frame: &mut Frame, area: Rect, body: &[u8], select_index: usize) {
        let title = Line::from(format!(
            "[{}]{}{}",
            select_index,
            symbols::line::HORIZONTAL,
            Self::BODY_LABEL
        ));
        let body_string = String::from_utf8_lossy(body);
        let mut block = Block::bordered().title(title);
        if self.section_selected == Section::Body {
            block = block.style(Style::default().fg(tailwind::BLUE.c400));
        }
        let paragraph = Paragraph::new(body_string).block(block);
        frame.render_widget(paragraph, area);
    }

    fn select_next_tab(&mut self) -> Option<AppMessage> {
        self.tab_selected = match self.tab_selected {
            Tab::Request => Tab::Response,
            Tab::Response => self.tab_selected.clone(),
        };
        self.section_selected = match self.section_selected {
            Section::Method | Section::Url => self.section_selected.clone(),
            _ => match self.tab_selected {
                Tab::Request => self.request_section_selected.clone().into(),
                Tab::Response => self.response_section_selected.clone().into(),
            },
        };
        None
    }

    fn select_previous_tab(&mut self) -> Option<AppMessage> {
        self.tab_selected = match self.tab_selected {
            Tab::Request => self.tab_selected.clone(),
            Tab::Response => Tab::Request,
        };
        self.section_selected = match self.section_selected {
            Section::Method | Section::Url => self.section_selected.clone(),
            _ => match self.tab_selected {
                Tab::Request => self.request_section_selected.clone().into(),
                Tab::Response => self.response_section_selected.clone().into(),
            },
        };
        None
    }

    fn select_section(&mut self, section_index: usize) -> Option<AppMessage> {
        match self.tab_selected {
            Tab::Request => {
                self.request_section_selected = section_index.into();
                self.section_selected = self.request_section_selected.clone().into();
            }
            Tab::Response => {
                self.response_section_selected = section_index.into();
                self.section_selected = self.response_section_selected.clone().into();
            }
        };

        self.section_selected = match section_index {
            1 => Section::Method,
            2 => Section::Url,
            _ => self.section_selected.clone(),
        };

        None
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

        self.draw_method_url(frame, method_url_area);

        self.draw_tabs(frame, tabbed_pane_area);
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

                self.draw_query_params(frame, params_area, 3);
                self.draw_headers(
                    frame,
                    headers_area,
                    request_entry.request.headers.clone(),
                    4,
                );
                self.draw_body(frame, body_area, request_entry.request.body.as_bytes(), 5);
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

                let headers = match &request_entry.response {
                    Some(response) => response.headers.clone(),
                    None => HeaderMap::new(vec![]),
                };
                self.draw_headers(frame, headers_area, headers, 3);
                self.draw_body(
                    frame,
                    body_area,
                    request_entry
                        .response
                        .as_ref()
                        .map(|response| response.body.as_bytes())
                        .unwrap_or_default(),
                    4,
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
                KeyCode::Left | KeyCode::Char('h') => {
                    return Some(Message::PreviousTab.into());
                }
                KeyCode::Right | KeyCode::Char('l') => {
                    return Some(Message::NextTab.into());
                }
                KeyCode::Char(c) => {
                    if let Some(d @ 1..=5) = c.to_digit(10) {
                        return Some(Message::SelectSection(d as usize).into());
                    }
                }
                _ => {}
            }
        }

        None
    }

    fn update(&mut self, message: AppMessage) -> Option<AppMessage> {
        let message = match message {
            AppMessage::HttpClientScreen(message) => message,
            _ => return None,
        };

        match message {
            Message::PreviousTab => self.select_previous_tab(),
            Message::NextTab => self.select_next_tab(),
            Message::SelectSection(section) => self.select_section(section),
        }
    }
}
