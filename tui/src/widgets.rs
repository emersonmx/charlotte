use crate::theme;
use ratatui::{
    buffer::Buffer,
    layout::{Alignment, Constraint, Margin, Rect},
    style::Style,
    text::{Line, Span},
    widgets::{
        Block, Paragraph, Row, Scrollbar, ScrollbarOrientation, ScrollbarState, StatefulWidget,
        Table, TableState, Widget,
    },
};

#[derive(Debug, Default, Clone, Eq, PartialEq, Hash)]
pub struct BorderedText {
    title: Option<String>,
    text: String,
    alignment: Alignment,
    is_focused: bool,
    focus_style: Style,
    scroll: (u16, u16),
}

impl BorderedText {
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            title: None,
            text: text.into(),
            alignment: Alignment::Left,
            is_focused: false,
            focus_style: theme::styles::highlight_fg(),
            scroll: (0, 0),
        }
    }

    pub fn title(mut self, title: Option<String>) -> Self {
        self.title = title;
        self
    }

    pub fn focused(mut self, is_focused: bool) -> Self {
        self.is_focused = is_focused;
        self
    }

    pub fn focus_style(mut self, style: Style) -> Self {
        self.focus_style = style;
        self
    }

    pub fn scroll(mut self, scroll: (u16, u16)) -> Self {
        self.scroll = scroll;
        self
    }

    pub fn centered(mut self) -> Self {
        self.alignment = Alignment::Center;
        self
    }
}

impl Widget for BorderedText {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let mut block = Block::bordered();
        if let Some(title) = self.title {
            block = block.title(title);
        }
        if self.is_focused {
            block = block.style(self.focus_style);
        }

        let inner = block.inner(area);
        let text_lines = self.text.lines().count() as u16;
        let max_scroll_y = text_lines.saturating_sub(inner.height);
        let scroll_y = self.scroll.0.min(max_scroll_y);

        let paragraph = Paragraph::new(self.text)
            .scroll((scroll_y, self.scroll.1))
            .block(block)
            .alignment(self.alignment);
        paragraph.render(area, buf);
    }
}

#[derive(Debug, Default, Clone, Eq, PartialEq, Hash)]
pub struct Tabs {
    titles: Vec<String>,
    spacer: String,
    selected: Option<usize>,
}

impl Tabs {
    pub fn titles(mut self, titles: Vec<String>) -> Self {
        self.titles = titles;
        self
    }

    pub fn with_spacer(mut self, spacer: impl Into<String>) -> Self {
        self.spacer = spacer.into();
        self
    }

    pub fn select<T: Into<Option<usize>>>(mut self, selected: T) -> Self {
        self.selected = selected.into();
        self
    }
}

impl Widget for Tabs {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let titles: Vec<Span> = self
            .titles
            .iter()
            .enumerate()
            .map(|(index, title)| {
                let style = if let Some(selected) = self.selected
                    && index == selected
                {
                    theme::styles::highlight_fg()
                } else {
                    Style::default()
                };
                Span::styled(title.clone(), style)
            })
            .fold(Vec::new(), |mut acc, title| {
                if !acc.is_empty() && !self.spacer.is_empty() {
                    acc.push(Span::raw(self.spacer.clone()));
                }
                acc.push(title);
                acc
            });

        let line = Line::from(titles);
        let tabs = Block::bordered().title(line);
        tabs.render(area, buf);
    }
}

#[derive(Debug, Default, Clone, Eq, PartialEq, Hash)]
pub struct KeyValueTableState {
    table_state: TableState,
    scrollbar_state: ScrollbarState,
    is_highlighted: bool,
}

impl KeyValueTableState {
    pub fn content_length(mut self, content_length: usize) -> Self {
        self.scrollbar_state = self.scrollbar_state.content_length(content_length);
        self
    }

    pub fn selected(&self) -> Option<usize> {
        self.table_state.selected()
    }

    pub fn select_next(&mut self) {
        self.table_state.select_next();
        self.scrollbar_state.next();
    }

    pub fn select_previous(&mut self) {
        self.table_state.select_previous();
        self.scrollbar_state.prev();
    }

    pub fn select_first(&mut self) {
        self.table_state.select_first();
        self.scrollbar_state.first();
    }

    pub fn select_last(&mut self) {
        self.table_state.select_last();
        self.scrollbar_state.last();
    }

    pub fn set_highlighted(&mut self, highlighted: bool) {
        self.is_highlighted = highlighted;
        if self.is_highlighted && self.table_state.selected().is_none() {
            self.table_state.select(Some(0));
        }
    }
}

#[derive(Debug, Default, Clone, Eq, PartialEq, Hash)]
pub struct KeyValueTable<'a> {
    rows: Vec<Row<'a>>,
    widths: Vec<Constraint>,
    block: Option<Block<'a>>,
}

impl<'a> KeyValueTable<'a> {
    pub fn rows<T>(mut self, rows: T) -> Self
    where
        T: IntoIterator<Item = Row<'a>>,
    {
        self.rows = rows.into_iter().collect();
        self
    }

    pub fn widths<I>(mut self, widths: I) -> Self
    where
        I: IntoIterator,
        I::Item: Into<Constraint>,
    {
        let widths: Vec<Constraint> = widths
            .into_iter()
            .map(Into::into)
            .map(|width| {
                if let Constraint::Percentage(value) = width
                    && value > 100
                {
                    Constraint::Percentage(100)
                } else {
                    width
                }
            })
            .collect();
        self.widths = widths;
        self
    }

    pub fn block(mut self, block: Block<'a>) -> Self {
        self.block = Some(block);
        self
    }
}

impl StatefulWidget for KeyValueTable<'_> {
    type State = KeyValueTableState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        let mut table = Table::default()
            .rows(self.rows)
            .widths(self.widths)
            .column_spacing(3);
        if state.is_highlighted {
            table = table.row_highlight_style(theme::styles::highlight());
        }
        if let Some(block) = self.block {
            table = table.block(block);
        }

        StatefulWidget::render(table, area, buf, &mut state.table_state);

        let scrollbar = Scrollbar::default()
            .orientation(ScrollbarOrientation::VerticalRight)
            .style(theme::styles::reset());
        StatefulWidget::render(
            scrollbar,
            area.inner(Margin {
                vertical: 1,
                horizontal: 0,
            }),
            buf,
            &mut state.scrollbar_state,
        );
    }
}

#[derive(Debug, Default, Clone, Eq, PartialEq, Hash)]
pub struct TextAreaState {
    scrollbar_state: ScrollbarState,
}

#[allow(dead_code)]
impl TextAreaState {
    pub fn new(content_length: usize) -> Self {
        Self {
            scrollbar_state: ScrollbarState::new(content_length),
        }
    }

    pub fn content_length(mut self, content_length: usize) -> Self {
        self.scrollbar_state = self.scrollbar_state.content_length(content_length);
        self
    }

    pub fn get_position(&self) -> usize {
        self.scrollbar_state.get_position()
    }

    pub fn position(mut self, position: usize) -> Self {
        self.scrollbar_state = self.scrollbar_state.position(position);
        self
    }

    pub fn next(&mut self) {
        self.scrollbar_state.next();
    }

    pub fn prev(&mut self) {
        self.scrollbar_state.prev();
    }

    pub fn first(&mut self) {
        self.scrollbar_state.first();
    }

    pub fn last(&mut self) {
        self.scrollbar_state.last();
    }
}

#[derive(Debug, Default, Clone, Eq, PartialEq, Hash)]
pub struct TextArea<'a> {
    content: String,
    block: Option<Block<'a>>,
}

impl<'a> TextArea<'a> {
    pub fn new(content: impl Into<String>) -> Self {
        Self {
            content: content.into(),
            block: None,
        }
    }

    pub fn block(mut self, block: Block<'a>) -> Self {
        self.block = Some(block);
        self
    }
}

impl StatefulWidget for TextArea<'_> {
    type State = TextAreaState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        let message_height = self.content.lines().count() + 1;
        state.scrollbar_state = state
            .scrollbar_state
            .content_length(message_height.saturating_sub(area.height as usize) + 2);

        let scroll = (state.get_position().try_into().unwrap_or(0), 0);
        let mut paragraph = Paragraph::new(self.content).scroll(scroll);
        if let Some(block) = self.block {
            paragraph = paragraph.block(block);
        }

        paragraph.render(area, buf);

        let scrollbar = Scrollbar::default()
            .orientation(ScrollbarOrientation::VerticalRight)
            .style(theme::styles::reset());
        StatefulWidget::render(
            scrollbar,
            area.inner(Margin {
                vertical: 1,
                horizontal: 0,
            }),
            buf,
            &mut state.scrollbar_state,
        );
    }
}
