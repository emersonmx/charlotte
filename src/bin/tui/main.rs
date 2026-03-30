use crate::app::App;

mod app;
mod data;
mod requests_screen;
mod waiting_screen;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let mut terminal = ratatui::init();
    let app_result = App::new().run(&mut terminal).await;
    ratatui::restore();
    app_result
}
