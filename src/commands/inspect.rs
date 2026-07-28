use crate::config::Config;
use crate::error::Result;

/// Milestone 2 stub. Will start an Axum server bound to `config.inspect_port`
/// that receives Daraja callbacks, pretty-prints them live, and decodes
/// ResultCode into plain English.
pub async fn run(config: &Config) -> Result<()> {
    println!("inspect: not yet implemented (Milestone 2)");
    println!();
    println!(
        "Will listen on port {} for incoming Daraja callbacks,",
        config.inspect_port
    );
    println!("pretty-print the JSON as it arrives, and translate ResultCode");
    println!("values (e.g. 1032 cancelled, 1037 timeout) into plain English.");
    Ok(())
}
