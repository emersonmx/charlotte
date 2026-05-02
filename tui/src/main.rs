use crate::app::App;
use crate::cli::Cli;
use crate::config::Config;
use clap::Parser;

mod app;
mod cli;
mod config;
mod requests_screen;
mod waiting_screen;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let (server_host, server_port) = (cli.host, cli.port);
    let config = Config::new(server_host, server_port)?;

    let mut terminal = ratatui::init();
    let app_result = App::new(config).run(&mut terminal).await;
    ratatui::restore();
    app_result
}
