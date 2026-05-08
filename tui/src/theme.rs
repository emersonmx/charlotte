#[allow(dead_code)]
pub mod styles {
    use ratatui::style::{Color, Style, palette::tailwind};

    const INFO_COLOR: Color = tailwind::BLUE.c500;
    const WARNING_COLOR: Color = tailwind::YELLOW.c500;
    const ERROR_COLOR: Color = tailwind::RED.c500;

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

    pub fn info() -> Style {
        Style::default().fg(INFO_COLOR)
    }

    pub fn warning() -> Style {
        Style::default().fg(WARNING_COLOR)
    }

    pub fn error() -> Style {
        Style::default().fg(ERROR_COLOR)
    }
}
