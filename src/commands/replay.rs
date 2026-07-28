use anyhow::{bail, Context, Result};
use clap::Args;
use colored::Colorize;
use rand::Rng;
use reqwest::Client;
use serde_json::Value;
use std::path::PathBuf;
use tokio::time::{sleep, Duration};

/// Arguments for the `replay` subcommand.
#[derive(Debug, Args)]
pub struct ReplayArgs {
    /// Path to the stored callback file (JSON or NDJSON).
    /// Use `mpesa-dev inspect --save callbacks.ndjson` to create one.
    pub file: PathBuf,

    /// Target URL to replay the callback against
    #[arg(short, long, env = "MPESA_CALLBACK_URL")]
    pub target: String,

    /// Delay between each replay (e.g. "500ms", "2s", "1m")
    #[arg(long, default_value = "0")]
    pub delay: String,

    /// Send each payload this many times (to test deduplication / idempotency)
    #[arg(long, default_value_t = 1)]
    pub duplicate: u32,

    /// Corrupt the payload (randomly remove/mutate fields) to test error handling
    #[arg(long, default_value_t = false)]
    pub corrupt: bool,

    /// Print the outgoing payload before sending
    #[arg(long, default_value_t = false)]
    pub verbose: bool,
}

pub async fn run(args: ReplayArgs) -> Result<()> {
    println!("\n{}", "mpesa-dev replay".bold());

    let raw = tokio::fs::read_to_string(&args.file)
        .await
        .with_context(|| format!("Cannot read file: {}", args.file.display()))?;

    // Support both plain JSON (single object) and NDJSON (one JSON object per line).
    let payloads = parse_payloads(&raw)?;

    if payloads.is_empty() {
        bail!("No JSON payloads found in {}", args.file.display());
    }

    let delay_ms = parse_delay_ms(&args.delay)?;

    let client = Client::builder().timeout(Duration::from_secs(15)).build()?;

    println!(
        "  Replaying {} payload(s) → {} (×{} duplicates){}\n",
        payloads.len(),
        args.target.cyan(),
        args.duplicate,
        if args.corrupt {
            " [CORRUPT mode]".red().to_string()
        } else {
            String::new()
        }
    );

    let mut total_sent = 0u64;
    let mut total_ok = 0u64;

    for (idx, original) in payloads.iter().enumerate() {
        for dup in 1..=args.duplicate {
            let mut payload = original.clone();

            if args.corrupt {
                payload = corrupt_payload(payload);
            }

            if args.verbose {
                println!("  {} #{}.{}  payload:", "→".cyan(), idx + 1, dup);
                println!(
                    "{}",
                    serde_json::to_string_pretty(&payload)
                        .unwrap_or_default()
                        .dimmed()
                );
            }

            let result = client.post(&args.target).json(&payload).send().await;

            total_sent += 1;
            match result {
                Ok(resp) => {
                    let status = resp.status();
                    let status_str = status.as_str().to_string();
                    let ok = status.is_success();
                    if ok {
                        total_ok += 1;
                    }
                    let status_colored = if ok {
                        status_str.green()
                    } else {
                        status_str.red()
                    };
                    println!(
                        "  {} #{}.{}  {}",
                        if ok { "✓".green() } else { "✗".red() },
                        idx + 1,
                        dup,
                        status_colored
                    );
                }
                Err(e) => {
                    eprintln!("  {} #{}.{}  error: {}", "✗".red(), idx + 1, dup, e);
                }
            }

            if delay_ms > 0 {
                sleep(Duration::from_millis(delay_ms)).await;
            }
        }
    }

    println!();
    println!(
        "  Done. {}/{} requests succeeded.",
        total_ok.to_string().green().bold(),
        total_sent
    );

    Ok(())
}

/// Parse the input file as either a single JSON object or NDJSON.
fn parse_payloads(raw: &str) -> Result<Vec<Value>> {
    let trimmed = raw.trim();

    if trimmed.is_empty() {
        return Ok(vec![]);
    }

    // Try to parse as a single top-level JSON value.
    if let Ok(val) = serde_json::from_str::<Value>(trimmed) {
        match val {
            Value::Array(items) => return Ok(items),
            single => return Ok(vec![single]),
        }
    }

    // Fall back to NDJSON: one JSON object per non-empty line.
    let mut results = Vec::new();
    for line in trimmed.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let v: Value = serde_json::from_str(line)
            .with_context(|| format!("Failed to parse line as JSON: {}", line))?;
        results.push(v);
    }
    Ok(results)
}

/// Parse a human-readable duration string into milliseconds.
/// Supports: "500ms", "2s", "1m", "0"
fn parse_delay_ms(s: &str) -> Result<u64> {
    let s = s.trim();
    if s == "0" || s.is_empty() {
        return Ok(0);
    }
    if let Some(rest) = s.strip_suffix("ms") {
        return Ok(rest.trim().parse::<u64>()?);
    }
    if let Some(rest) = s.strip_suffix('s') {
        return Ok(rest.trim().parse::<u64>()? * 1000);
    }
    if let Some(rest) = s.strip_suffix('m') {
        return Ok(rest.trim().parse::<u64>()? * 60_000);
    }
    // Plain number → milliseconds
    Ok(s.parse::<u64>()?)
}

/// Corrupt a JSON payload for chaos testing.
/// Randomly: removes a field, sets a numeric value to -1, or truncates a string.
fn corrupt_payload(mut value: Value) -> Value {
    let mut rng = rand::thread_rng();
    if let Value::Object(ref mut map) = value {
        if map.is_empty() {
            return value;
        }
        let keys: Vec<String> = map.keys().cloned().collect();
        let action = rng.gen_range(0..3u8);
        let target_key = &keys[rng.gen_range(0..keys.len())];
        match action {
            0 => {
                // Remove a random field
                map.remove(target_key);
            }
            1 => {
                // Set a numeric field to -1
                if let Some(v) = map.get_mut(target_key) {
                    if v.is_number() {
                        *v = Value::Number((-1i64).into());
                    }
                }
            }
            _ => {
                // Truncate a string field
                if let Some(Value::String(s)) = map.get_mut(target_key) {
                    s.truncate(s.len() / 2);
                }
            }
        }
    }
    value
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_delay() {
        assert_eq!(parse_delay_ms("0").unwrap(), 0);
        assert_eq!(parse_delay_ms("500ms").unwrap(), 500);
        assert_eq!(parse_delay_ms("2s").unwrap(), 2000);
        assert_eq!(parse_delay_ms("1m").unwrap(), 60_000);
        assert_eq!(parse_delay_ms("100").unwrap(), 100);
    }

    #[test]
    fn test_parse_payloads_single() {
        let raw = r#"{"ResultCode":0,"ResultDesc":"Success"}"#;
        let payloads = parse_payloads(raw).unwrap();
        assert_eq!(payloads.len(), 1);
    }

    #[test]
    fn test_parse_payloads_ndjson() {
        let raw = "{\"a\":1}\n{\"b\":2}\n{\"c\":3}";
        let payloads = parse_payloads(raw).unwrap();
        assert_eq!(payloads.len(), 3);
    }

    #[test]
    fn test_parse_payloads_array() {
        let raw = r#"[{"a":1},{"b":2}]"#;
        let payloads = parse_payloads(raw).unwrap();
        assert_eq!(payloads.len(), 2);
    }

    #[test]
    fn test_corrupt_removes_or_changes_field() {
        let original = serde_json::json!({"ResultCode": 0, "ResultDesc": "Success", "TransactionID": "ABCD1234"});
        // Run corruption 10 times; the result should differ from the original at least once.
        let same_count = (0..10)
            .filter(|_| corrupt_payload(original.clone()) == original)
            .count();
        assert!(same_count < 10, "corrupt_payload never changed anything");
    }
}
