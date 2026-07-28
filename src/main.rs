use anyhow::Result;
use clap::{Parser, Subcommand};

mod commands;
mod output;
mod result_codes;

use commands::{doctor::DoctorArgs, inspect::InspectArgs, replay::ReplayArgs, tunnel::TunnelArgs};

/// The missing local development toolkit for M-Pesa Daraja.
///
/// Diagnose config, inspect live callbacks, tunnel to localhost, and replay
/// payloads — no Node, no ngrok, one static binary.
#[derive(Debug, Parser)]
#[command(
    name = "mpesa-dev",
    version,
    about = "The missing local development toolkit for M-Pesa Daraja",
    long_about = None
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    /// Run a pass/fail health-check on credentials, OAuth, callback URL,
    /// HTTPS validity, sandbox reachability, and clock skew.
    Doctor(DoctorArgs),

    /// Start a local server that receives M-Pesa callbacks, pretty-prints
    /// the JSON payload, and decodes every ResultCode into plain English.
    Inspect(InspectArgs),

    /// Expose a public HTTPS tunnel URL that forwards to your local server
    /// (no ngrok required).  Defaults to bore.pub; point --server at your own
    /// relay for full control.
    Tunnel(TunnelArgs),

    /// Resend a stored callback from a file.  Supports --delay, --duplicate,
    /// and --corrupt flags for testing retry logic.
    Replay(ReplayArgs),
}

#[tokio::main]
async fn main() -> Result<()> {
    // Set up a minimal tracing subscriber so library crates can log warnings.
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(tracing::Level::WARN.into()),
        )
        .with_target(false)
        .without_time()
        .init();

    let cli = Cli::parse();

    match cli.command {
        Commands::Doctor(args) => commands::doctor::run(args).await,
        Commands::Inspect(args) => commands::inspect::run(args).await,
        Commands::Tunnel(args) => commands::tunnel::run(args).await,
        Commands::Replay(args) => commands::replay::run(args).await,
    }
}
