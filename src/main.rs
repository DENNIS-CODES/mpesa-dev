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
            // command guide/picker can't drift out of sync with `--help`.
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

            match banner::print_full_and_choose(&config.environment, &subcommand_docs) {
                banner::Choice::Selected(index) => {
                    match subcommand_docs.get(index).map(|(name, _)| name.as_str()) {
                        Some("doctor") => commands::doctor::run(&config).await?,
                        Some("inspect") => commands::inspect::run(&config).await?,
                        Some("tunnel") => commands::tunnel::run(&config).await?,
                        Some("replay") => {
                            commands::replay::run(&config, None, None, false, false).await?
                        }
                        _ => {}
                    }
                }
                // No real terminal to run the picker in — keep the old
                // default so scripts/CI piping mpesa-dev still get inspect.
                banner::Choice::NonInteractive => commands::inspect::run(&config).await?,
                // User backed out of the picker (Esc/Ctrl+C) — run nothing.
                banner::Choice::Cancelled => {}
            }
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
