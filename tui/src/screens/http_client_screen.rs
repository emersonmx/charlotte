use crate::{
    app::{Message as AppMessage, RequestEntry, Screen},
    inputmap::{Input, map_event_to_input},
    theme,
    widgets::{BorderedText, KeyValueTable, KeyValueTableState, Tabs, TextArea, TextAreaState},
};
use crossterm::event::Event;
use proxy::http::HeaderMap;
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Margin, Rect},
    symbols,
    text::Line,
    widgets::{Block, Cell, Row},
};
use std::fmt::Display;

#[derive(Debug, Clone, PartialEq)]
pub enum Message {
    PreviousRow,
    NextRow,
    SelectSection(usize),
    PreviousTab,
    NextTab,
    CopySelectedToClipboard,
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

#[derive(Debug, Clone, Default, PartialEq)]
struct TableColumnWidths {
    key: u16,
    value: u16,
}

impl TableColumnWidths {
    fn update(&mut self, key: &str, value: &str) {
        self.key = self.key.max(key.chars().count() as u16);
        self.value = self.value.max(value.chars().count() as u16);
    }

    fn to_table_widths(&self) -> [Constraint; 2] {
        [Constraint::Length(self.key), Constraint::Min(0)]
    }
}

impl From<Vec<(&str, &str)>> for TableColumnWidths {
    fn from(pairs: Vec<(&str, &str)>) -> Self {
        let mut widths = TableColumnWidths::default();
        for (key, value) in pairs {
            widths.update(key, value);
        }
        widths
    }
}

impl From<HeaderMap> for TableColumnWidths {
    fn from(headers: HeaderMap) -> Self {
        let mut widths = TableColumnWidths::default();
        for header in headers.iter() {
            let key = header.key().to_string();
            let value = header.value().unwrap_or_default().to_string();
            widths.update(&key, &value);
        }
        widths
    }
}

pub struct HttpClientScreen {
    request_entry: RequestEntry,
    tab_selected: Tab,
    section_selected: Section,
    request_section_selected: RequestSection,
    response_section_selected: ResponseSection,
    query_params_table_state: KeyValueTableState,
    request_query_params_column_widths: TableColumnWidths,
    request_headers_column_widths: TableColumnWidths,
    request_headers_table_state: KeyValueTableState,
    request_body_state: TextAreaState,
    response_headers_column_widths: TableColumnWidths,
    response_headers_table_state: KeyValueTableState,
    response_body_state: TextAreaState,
}

impl HttpClientScreen {
    const METHOD_LABEL: &str = "Method";
    const URL_LABEL: &str = "URL";
    const QUERY_PARAMS_LABEL: &str = "Query Parameters";
    const HEADERS_LABEL: &str = "Headers";
    const BODY_LABEL: &str = "Body";

    pub fn new(request_entry: RequestEntry) -> Self {
        let request_query_params_column_widths = request_entry.request.url.query_pairs().into();
        let query_params_table_state = {
            let count = request_entry.request.url.query_pairs().len();
            KeyValueTableState::default().content_length(count)
        };
        let request_headers_column_widths = request_entry.request.headers.clone().into();
        let request_headers_table_state = {
            let count = request_entry.request.headers.len();
            KeyValueTableState::default().content_length(count)
        };
        let response_headers_column_widths = match &request_entry.response {
            Some(response) => response.headers.clone().into(),
            None => TableColumnWidths::default(),
        };
        let response_headers_table_state = match &request_entry.response {
            Some(response) => {
                let count = response.headers.len();
                KeyValueTableState::default().content_length(count)
            }
            None => KeyValueTableState::default(),
        };
        let request_body_state = {
            let body_string = String::from_utf8_lossy(request_entry.request.body.as_bytes());
            let lines = body_string.lines().count();
            TextAreaState::new(lines)
        };
        let response_body_state = match &request_entry.response {
            Some(response) => {
                let body_string = String::from_utf8_lossy(response.body.as_bytes());
                let lines = body_string.lines().count();
                TextAreaState::new(lines)
            }
            None => TextAreaState::default(),
        };

        Self {
            request_entry,
            tab_selected: Tab::Request,
            section_selected: Section::default(),
            request_section_selected: RequestSection::default(),
            response_section_selected: ResponseSection::default(),
            request_query_params_column_widths,
            query_params_table_state,
            request_headers_column_widths,
            request_headers_table_state,
            request_body_state,
            response_headers_column_widths,
            response_headers_table_state,
            response_body_state,
        }
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
        let text = self.request_entry.request.method.to_string();
        let bordered_text = BorderedText::new(text)
            .title(Some(format!(
                "[1]{}{}",
                symbols::line::HORIZONTAL,
                Self::METHOD_LABEL
            )))
            .focused(self.section_selected == Section::Method);

        frame.render_widget(bordered_text, area);
    }

    fn draw_url(&self, frame: &mut Frame, area: Rect) {
        let text = self.request_entry.request.url.to_string();
        let bordered_text = BorderedText::new(text)
            .title(Some(format!(
                "[2]{}{}",
                symbols::line::HORIZONTAL,
                Self::URL_LABEL
            )))
            .focused(self.section_selected == Section::Url);

        frame.render_widget(bordered_text, area);
    }

    fn draw_tabs(&self, frame: &mut Frame, area: Rect) {
        let status = match &self.request_entry.response {
            Some(response) => response.status.to_string(),
            None => String::default(),
        };

        let tab_line = Tabs::default()
            .titles(vec![
                format!(" {} ", Tab::Request),
                format!(" {} ({}) ", Tab::Response, status),
            ])
            .with_spacer(symbols::line::HORIZONTAL)
            .select(match self.tab_selected {
                Tab::Request => 0,
                Tab::Response => 1,
            });
        frame.render_widget(tab_line, area);
    }

    fn draw_query_params(&mut self, frame: &mut Frame, area: Rect, select_index: usize) {
        let request_entry = &self.request_entry;

        let params: Vec<Row> = request_entry
            .request
            .url
            .query_pairs()
            .iter()
            .map(|(key, value)| {
                Row::new(vec![
                    Cell::from(key.to_string()),
                    Cell::from(value.to_string()),
                ])
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
            block = block.style(theme::styles::highlight_fg());
        }

        let kvtable = KeyValueTable::default()
            .rows(params)
            .block(block)
            .widths(self.request_query_params_column_widths.to_table_widths());
        frame.render_stateful_widget(kvtable, area, &mut self.query_params_table_state);
    }

    fn draw_headers(
        &mut self,
        frame: &mut Frame,
        area: Rect,
        headers: HeaderMap,
        select_index: usize,
    ) {
        let table_column_widths = match self.tab_selected {
            Tab::Request => self.request_headers_column_widths.to_table_widths(),
            Tab::Response => self.response_headers_column_widths.to_table_widths(),
        };

        let headers: Vec<Row> = headers
            .iter()
            .map(|header| {
                let key = header.key().to_string();
                let value = header.value().unwrap_or_default().to_string();
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
            block = block.style(theme::styles::highlight_fg());
        }

        let state = match self.tab_selected {
            Tab::Request => &mut self.request_headers_table_state,
            Tab::Response => &mut self.response_headers_table_state,
        };
        let kvtable = KeyValueTable::default()
            .rows(headers)
            .block(block)
            .widths(table_column_widths);
        frame.render_stateful_widget(kvtable, area, state);
    }

    fn draw_body(&mut self, frame: &mut Frame, area: Rect, body: &[u8], select_index: usize) {
        let title = Line::from(format!(
            "[{}]{}{} ({} bytes)",
            select_index,
            symbols::line::HORIZONTAL,
            Self::BODY_LABEL,
            body.len()
        ));
        let body_string = String::from_utf8_lossy(body);
        let mut block = Block::bordered().title(title);
        if self.section_selected == Section::Body {
            block = block.style(theme::styles::highlight_fg());
        }

        let state = match self.tab_selected {
            Tab::Request => &mut self.request_body_state,
            Tab::Response => &mut self.response_body_state,
        };

        let text_area = TextArea::new(body_string.to_string()).block(block);
        frame.render_stateful_widget(text_area, area, state);
    }

    fn draw_status_bar(&self, frame: &mut Frame, area: Rect) {
        let text = "1-5 to select a section. Arrow keys or h/j/k/l to navigate. y to copy selected value to clipboard. Backspace to go back. q to quit.";

        let bordered_text = BorderedText::new(text);
        frame.render_widget(bordered_text, area);
    }

    fn request_entry_updated(&mut self, request_entry: RequestEntry) -> Option<AppMessage> {
        if self.request_entry.request_id != request_entry.request_id {
            return None;
        }

        *self = Self::new(request_entry);
        None
    }

    fn previous_row(&mut self) -> Option<AppMessage> {
        match self.tab_selected {
            Tab::Request => match self.section_selected {
                Section::QueryParams => {
                    self.query_params_table_state.select_previous();
                }
                Section::Headers => {
                    self.request_headers_table_state.select_previous();
                }
                Section::Body => {
                    self.request_body_state.prev();
                }
                _ => {}
            },
            Tab::Response => match self.section_selected {
                Section::Headers => {
                    self.response_headers_table_state.select_previous();
                }
                Section::Body => {
                    self.response_body_state.prev();
                }
                _ => {}
            },
        }

        None
    }

    fn next_row(&mut self) -> Option<AppMessage> {
        match self.tab_selected {
            Tab::Request => match self.section_selected {
                Section::QueryParams => {
                    self.query_params_table_state.select_next();
                }
                Section::Headers => {
                    self.request_headers_table_state.select_next();
                }
                Section::Body => {
                    self.request_body_state.next();
                }
                _ => {}
            },
            Tab::Response => match self.section_selected {
                Section::Headers => {
                    self.response_headers_table_state.select_next();
                }
                Section::Body => {
                    self.response_body_state.next();
                }
                _ => {}
            },
        }

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

        self.highlight_selected();

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
        self.highlight_selected();
        None
    }

    fn highlight_selected(&mut self) {
        self.query_params_table_state.set_highlighted(false);
        self.request_headers_table_state.set_highlighted(false);
        self.response_headers_table_state.set_highlighted(false);

        match self.tab_selected {
            Tab::Request => match self.section_selected {
                Section::QueryParams => self.query_params_table_state.set_highlighted(true),
                Section::Headers => self.request_headers_table_state.set_highlighted(true),
                _ => {}
            },
            Tab::Response => {
                if self.section_selected == Section::Headers {
                    self.response_headers_table_state.set_highlighted(true)
                }
            }
        };
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
        self.highlight_selected();
        None
    }

    fn copy_selected_to_clipboard(&mut self) -> Option<AppMessage> {
        let request_entry = &self.request_entry;

        let text = match (&self.tab_selected, &self.section_selected) {
            (_, Section::Method) => request_entry.request.method.to_string(),
            (_, Section::Url) => request_entry.request.url.to_string(),
            (Tab::Request, Section::QueryParams) => self.get_query_param_selected(),
            (Tab::Request, Section::Headers) => self.get_request_header_selected(),
            (Tab::Request, Section::Body) => self.get_request_body(),
            (Tab::Response, Section::Headers) => self.get_response_header_selected(),
            (Tab::Response, Section::Body) => self.get_response_body(),
            _ => String::default(),
        };

        Some(AppMessage::CopyToClipboard(text))
    }

    fn get_query_param_selected(&self) -> String {
        if self.section_selected != Section::QueryParams {
            return String::default();
        }

        let selected = self.query_params_table_state.selected().unwrap_or(0);
        self.request_entry
            .request
            .url
            .query_pairs()
            .get(selected)
            .map(|(key, value)| format!("{}={}", key, value))
            .unwrap_or_default()
    }

    fn get_request_header_selected(&self) -> String {
        let selected = self.request_headers_table_state.selected().unwrap_or(0);
        self.request_entry
            .request
            .headers
            .iter()
            .nth(selected)
            .map(|header| {
                let key = header.key().to_string();
                let value = header.value().unwrap_or_default().to_string();
                format!("{}: {}", key, value)
            })
            .unwrap_or_default()
    }

    fn get_request_body(&self) -> String {
        String::from_utf8_lossy(self.request_entry.request.body.as_bytes()).to_string()
    }

    fn get_response_header_selected(&self) -> String {
        let selected = self.response_headers_table_state.selected().unwrap_or(0);
        self.request_entry
            .response
            .as_ref()
            .and_then(|response| response.headers.iter().nth(selected))
            .map(|header| {
                let key = header.key().to_string();
                let value = header.value().unwrap_or_default().to_string();
                format!("{}: {}", key, value)
            })
            .unwrap_or_default()
    }

    fn get_response_body(&self) -> String {
        self.request_entry
            .response
            .as_ref()
            .map(|response| String::from_utf8_lossy(response.body.as_bytes()).to_string())
            .unwrap_or_default()
    }
}

impl Screen for HttpClientScreen {
    fn draw(&mut self, frame: &mut Frame) {
        let request_entry = &self.request_entry.clone();

        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Min(3),
                Constraint::Length(3),
            ]);
        let [method_url_area, tabbed_pane_area, status_bar_area] = frame.area().layout(&layout);

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

        self.draw_status_bar(frame, status_bar_area);
    }

    fn handle_event(&self, event: Event) -> Option<AppMessage> {
        match map_event_to_input(&event) {
            Some(Input::Up) => Some(Message::PreviousRow.into()),
            Some(Input::Down) => Some(Message::NextRow.into()),
            Some(Input::Section(section @ 1..=5)) => Some(Message::SelectSection(section).into()),
            Some(Input::PreviousTab) => Some(Message::PreviousTab.into()),
            Some(Input::NextTab) => Some(Message::NextTab.into()),
            Some(Input::Copy) => Some(Message::CopySelectedToClipboard.into()),
            Some(Input::Back) => Some(AppMessage::ShowRequestsScreen),
            Some(Input::Help) => Some(AppMessage::ShowShortcutsModal),
            Some(Input::Quit) => Some(AppMessage::Quit),
            _ => None,
        }
    }

    fn update(&mut self, message: AppMessage) -> Option<AppMessage> {
        let message = match message {
            AppMessage::HttpClientScreen(message) => message,
            AppMessage::RequestEntryUpdated(request_entry) => {
                return self.request_entry_updated(*request_entry);
            }
            _ => return None,
        };

        match message {
            Message::PreviousRow => self.previous_row(),
            Message::NextRow => self.next_row(),
            Message::SelectSection(section) => self.select_section(section),
            Message::PreviousTab => self.select_previous_tab(),
            Message::NextTab => self.select_next_tab(),
            Message::CopySelectedToClipboard => self.copy_selected_to_clipboard(),
        }
    }
}
