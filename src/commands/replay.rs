use crate::config::Config;
use crate::error::Result;

/// Milestone 4 stub. Will read a callback JSON file persisted by `inspect`
/// and resend it to a local endpoint, optionally delayed, duplicated, or
/// corrupted to exercise error handling.
pub async fn run(
    _config: &Config,
    file: Option<String>,
    delay: Option<u64>,
    duplicate: bool,
    corrupt: bool,
) -> Result<()> {
    println!("replay: not yet implemented (Milestone 4)");
    println!();
    match file {
        Some(path) => println!("Would replay callback stored at: {path}"),
        None => println!("No file given: would list stored callbacks to choose from."),
    }
    if let Some(ms) = delay {
        println!("  --delay {ms}ms");
    }
    if duplicate {
        println!("  --duplicate: would send the callback twice");
    }
    if corrupt {
        println!("  --corrupt: would mangle the payload before sending");
    }
    Ok(())
}
