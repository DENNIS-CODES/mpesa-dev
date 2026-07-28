use clap::builder::styling::AnsiColor;
use clap::builder::Styles;
use clap::{Parser, Subcommand};

/// Colored `--help` output: bold green headers/usage, bold cyan literals
/// (subcommand and flag names), matching the same palette the rest of the
/// tool uses for its "everything looks like one product" styling.
fn styles() -> Styles {
    Styles::styled()
        .header(AnsiColor::Green.on_default().bold())
        .usage(AnsiColor::Green.on_default().bold())
        .literal(AnsiColor::Cyan.on_default().bold())
        .placeholder(AnsiColor::Cyan.on_default())
}

#[derive(Debug, Parser)]
#[command(
    name = "mpesa-dev",
    version,
    about = "A single-binary CLI for M-Pesa Daraja local development.",
    long_about = "Diagnose config, inspect live callbacks, tunnel to localhost, and replay payloads. No Node, no ngrok.",
    styles = styles()
)]
pub struct Cli {
    /// Run with no subcommand to see the banner and jump straight into `inspect`.
    #[command(subcommand)]
    pub command: Option<Command>,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Run sandbox connectivity and config checks
    Doctor,

    /// Start a local server that prints incoming Daraja callbacks live
    Inspect,

    /// Expose a local port to the internet so Daraja can reach your callback URL
    Tunnel,

    /// Resend a previously captured callback to a local endpoint
    Replay {
        /// Stored callback to resend: its index or filename from `mpesa-dev replay`
        /// with no arguments. Omit to list what's available.
        selector: Option<String>,

        /// Delay the resend by this many milliseconds
        #[arg(long)]
        delay: Option<u64>,

        /// Send the callback twice, back to back
        #[arg(long)]
        duplicate: bool,

        /// Corrupt the payload before sending, to test error handling
        #[arg(long)]
        corrupt: bool,
    },
}
