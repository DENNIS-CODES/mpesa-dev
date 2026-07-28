use crate::config::Config;
use crate::error::Result;

/// Milestone 1 stub. Will run sequential checks (consumer key/secret
/// validity, OAuth round trip, callback URL reachability, HTTPS cert
/// validity, sandbox reachability, clock skew) and print a pass/fail
/// checklist with suggested fixes.
pub async fn run(_config: &Config) -> Result<()> {
    println!("doctor: not yet implemented (Milestone 1)");
    println!();
    println!("Planned checks:");
    println!("  - consumer key/secret validity");
    println!("  - OAuth round trip");
    println!("  - callback URL reachability");
    println!("  - HTTPS cert validity");
    println!("  - sandbox reachability");
    println!("  - clock skew vs. Safaricom server time");
    Ok(())
}
