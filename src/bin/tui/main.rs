use crate::app::App;
use crate::cli::Cli;
use clap::Parser;

mod app;
mod cli;
mod navigation;
mod requests_screen;
mod waiting_screen;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let (server_host, server_port) = (cli.host, cli.port);

    let mut terminal = ratatui::init();
    let app_result = App::new(server_host, server_port).run(&mut terminal).await;
    ratatui::restore();
    app_result
}
