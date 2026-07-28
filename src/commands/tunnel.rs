use crate::config::Config;
use crate::error::Result;

/// Milestone 3 stub. Will open a websocket connection to the relay, receive
/// a public HTTPS URL, and forward inbound requests to `config.inspect_port`
/// on localhost.
pub async fn run(_config: &Config) -> Result<()> {
    println!("tunnel: not yet implemented (Milestone 3)");
    println!();
    println!("Will connect to the mpesa-dev relay over a websocket, receive a");
    println!("public HTTPS subdomain, and forward inbound callback requests to");
    println!("localhost so you can use it as a Daraja callback URL without ngrok.");
    Ok(())
}
