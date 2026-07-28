use clap::{Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(
    name = "mpesa-dev",
    version,
    about = "A single-binary CLI for M-Pesa Daraja local development.",
    long_about = "Diagnose config, inspect live callbacks, tunnel to localhost, and replay payloads. No Node, no ngrok."
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
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
        /// Path to a stored callback JSON file (see `inspect` for how these are saved)
        file: Option<String>,

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
