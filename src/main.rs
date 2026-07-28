mod callback_store;
mod cli;
mod commands;
mod config;
mod daraja;
mod error;

use clap::{CommandFactory, Parser};
use mpesa_dev::banner;

use cli::{Cli, Command};
use config::Config;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let config = Config::load()?;

    match cli.command {
        None => {
            // Pulled from clap's own metadata (the `Command` enum's doc
            // comments) rather than duplicated here, so the banner's
            // command guide can't drift out of sync with `--help`.
            let subcommand_docs: Vec<(String, String)> = Cli::command()
                .get_subcommands()
                .filter(|cmd| cmd.get_name() != "help")
                .map(|cmd| {
                    (
                        cmd.get_name().to_string(),
                        cmd.get_about().map(|s| s.to_string()).unwrap_or_default(),
                    )
                })
                .collect();
            banner::print_full(&config.environment, &subcommand_docs);
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
