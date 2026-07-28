mod cli;
mod commands;
mod config;
mod daraja;
mod error;

use clap::Parser;

use cli::{Cli, Command};
use config::Config;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let config = Config::load()?;

    match cli.command {
        Command::Doctor => commands::doctor::run(&config).await?,
        Command::Inspect => commands::inspect::run(&config).await?,
        Command::Tunnel => commands::tunnel::run(&config).await?,
        Command::Replay {
            file,
            delay,
            duplicate,
            corrupt,
        } => commands::replay::run(&config, file, delay, duplicate, corrupt).await?,
    }

    Ok(())
}
