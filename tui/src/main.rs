use crate::app::App;
use crate::cli::Cli;
use crate::config::Config;
use clap::Parser;

mod app;
mod cli;
mod clipboard;
mod config;
mod modals;
mod screens;
mod theme;
mod widgets;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let (server_host, server_port) = (cli.host, cli.port);
    let config = Config::new(server_host, server_port)?;

    let mut terminal = ratatui::init();
    let app_result = match App::new(config).as_mut() {
        Ok(app) => app.run(&mut terminal).await,
        Err(err) => Err(anyhow::anyhow!("Failed to initialize app: {err}")),
    };
    ratatui::restore();
    app_result
}
