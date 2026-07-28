use std::time::Duration;

use colored::Colorize;

use crate::callback_store;
use crate::config::Config;
use crate::error::Result;
use mpesa_dev::banner::{self, icon};

/// Resends a callback `inspect` previously persisted to disk, against the
/// local `inspect` server. With no selector, lists what's stored instead.
pub async fn run(
    config: &Config,
    selector: Option<String>,
    delay: Option<u64>,
    duplicate: bool,
    corrupt: bool,
) -> Result<()> {
    banner::header("Replay");

    let Some(selector) = selector else {
        return print_list();
    };

    let path = callback_store::resolve(&selector)?;
    let body = std::fs::read_to_string(&path)?;

    println!("{} Replaying {}", icon::arrow(), path.display());
    println!("  {}", callback_store::summarize(&path));

    let body = if corrupt {
        let corrupted = corrupt_payload(&body);
        println!("  {} payload corrupted before sending:", icon::warn());
        println!();
        print_diff(&body, &corrupted);
        corrupted
    } else {
        body
    };

    if let Some(ms) = delay {
        println!("  {} delaying {ms}ms before sending", icon::arrow());
        tokio::time::sleep(Duration::from_millis(ms)).await;
    }

    let target = format!("http://127.0.0.1:{}", config.inspect_port);
    let http = reqwest::Client::new();
    println!();
    send_once(&http, &target, &body).await;

    if duplicate {
        send_once(&http, &target, &body).await;
    }

    Ok(())
}

fn print_list() -> Result<()> {
    let files = callback_store::list()?;
    if files.is_empty() {
        println!("{} no stored callbacks yet", icon::skip());
        println!("  run `mpesa-dev inspect` and trigger a callback first, then come back here");
        return Ok(());
    }

    println!("{}\n", "Stored callbacks (newest first):".bold());
    for (i, path) in files.iter().enumerate() {
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("?");
        let index = format!("{:>2}", i + 1);
        println!("  {}) {name}", index.cyan().bold());
        println!("      {}", callback_store::summarize(path));
    }
    println!();
    println!(
        "Run `mpesa-dev replay <number-or-filename>` to resend one, e.g. `mpesa-dev replay 1`."
    );
    Ok(())
}

async fn send_once(http: &reqwest::Client, target: &str, body: &str) {
    match http
        .post(target)
        .header("Content-Type", "application/json")
        .body(body.to_string())
        .send()
        .await
    {
        Ok(response) => {
            let status = response.status();
            if status.is_success() {
                println!("{} {target} responded {status}", icon::ok());
            } else {
                println!("{} {target} responded {status}", icon::warn());
            }
        }
        Err(e) => {
            println!("{} failed to reach {target}: {e}", icon::fail());
        }
    }
}

/// Prints a git-diff-style, line-by-line comparison of the original and
/// corrupted payload: unchanged lines dimmed, removed lines on a red
/// background, added/changed lines on a green background — so it's
/// obvious at a glance exactly what `--corrupt` did before it goes out
/// over the wire.
fn print_diff(original: &str, corrupted: &str) {
    let orig_lines: Vec<&str> = original.lines().collect();
    let new_lines: Vec<&str> = corrupted.lines().collect();
    let total = orig_lines.len().max(new_lines.len());

    for i in 0..total {
        let num = i + 1;
        match (orig_lines.get(i), new_lines.get(i)) {
            (Some(a), Some(b)) if a == b => {
                println!("  {num:>3}   {}", a.dimmed());
            }
            (Some(a), Some(b)) => {
                println!("  {num:>3} {} {}", "-".black().on_red(), a.black().on_red());
                println!(
                    "  {num:>3} {} {}",
                    "+".black().on_green(),
                    b.black().on_green()
                );
            }
            (Some(a), None) => {
                println!("  {num:>3} {} {}", "-".black().on_red(), a.black().on_red());
            }
            (None, Some(b)) => {
                println!(
                    "  {num:>3} {} {}",
                    "+".black().on_green(),
                    b.black().on_green()
                );
            }
            (None, None) => {}
        }
    }
    println!();
}

/// Truncates the payload partway through and appends a marker, reliably
/// producing invalid JSON to exercise a receiver's error handling — a
/// stand-in for a real-world truncated or corrupted request body.
fn corrupt_payload(body: &str) -> String {
    let mut cut = body.len() * 4 / 5;
    while cut > 0 && !body.is_char_boundary(cut) {
        cut -= 1;
    }
    let mut corrupted = body[..cut].to_string();
    corrupted.push_str(" <-- truncated by mpesa-dev replay --corrupt");
    corrupted
}
