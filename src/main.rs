mod callback_store;
mod cli;
mod commands;
mod config;
mod daraja;
mod error;

use clap::Parser;
use mpesa_dev::banner;

use cli::{Cli, Command};
use config::Config;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let config = Config::load()?;

    match cli.command {
        None => {
            banner::print_full();
            commands::inspect::run(&config).await?
        }
        Some(Command::Doctor) => commands::doctor::run(&config).await?,
        Some(Command::Inspect) => commands::inspect::run(&config).await?,
        Some(Command::Tunnel) => commands::tunnel::run(&config).await?,
        Some(Command::Replay {
            selector,
            delay,
            duplicate,
            corrupt,
        }) => commands::replay::run(&config, selector, delay, duplicate, corrupt).await?,
    }

    Ok(())
}
