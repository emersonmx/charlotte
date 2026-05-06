pub mod styles {
    use ratatui::style::{Color, Style, palette::tailwind};

    const LIGHT_GREEN: Color = tailwind::GREEN.c300;
    const DARK_GREEN: Color = tailwind::GREEN.c900;

    pub fn reset() -> Style {
        Style::reset()
    }

    pub fn highlight() -> Style {
        Style::default().fg(DARK_GREEN).bg(LIGHT_GREEN)
    }

    pub fn highlight_fg() -> Style {
        Style::default().fg(LIGHT_GREEN)
    }
}
