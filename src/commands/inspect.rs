use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use axum::body::Bytes;
use axum::error_handling::HandleErrorLayer;
use axum::extract::{ConnectInfo, State};
use axum::http::StatusCode;
use axum::routing::any;
use axum::{BoxError, Router};
use chrono::Local;
use colored::Colorize;
use tower::timeout::TimeoutLayer;
use tower::ServiceBuilder;

use crate::callback_store;
use crate::config::Config;
use crate::daraja::models::{CallbackMetadata, StkCallbackEnvelope};
use crate::daraja::result_code::{self, Outcome};
use crate::error::Result;
use mpesa_dev::banner::{self, icon};

/// Running counts shown after every event, so the terminal reads as a
/// live dashboard instead of a scrolling log with no sense of history.
#[derive(Default)]
struct Stats {
    payments: u64,
    callbacks: u64,
    errors: u64,
}

type SharedStats = Arc<Mutex<Stats>>;

/// Starts a local HTTP server that accepts Daraja callbacks on any path and
/// method, pretty-prints them as they arrive, decodes ResultCode into
/// plain English for recognized STK push callbacks, and persists each one
/// to disk so `replay` has something to resend later.
pub async fn run(config: &Config) -> Result<()> {
    banner::header("Inspector");

    let bind_addr = format!("0.0.0.0:{}", config.inspect_port);
    let listener = tokio::net::TcpListener::bind(&bind_addr).await?;
    let local_url = format!("http://127.0.0.1:{}", config.inspect_port);

    let rule = "━".repeat(58);
    println!("{}", rule.dimmed());
    println!("  {}", "Inspector running".bold());
    println!("  URL      {}", local_url.green().bold());
    println!("  Waiting for callbacks...");
    println!("{}", rule.dimmed());
    println!();
    println!(
        "{} Point your Daraja callback_url here (via a public tunnel) and trigger an STK push",
        icon::arrow()
    );
    println!("  (bound to all interfaces at {bind_addr}, for tunnel/relay forwarding)");
    println!("  Press Ctrl+C to stop\n");

    let stats: SharedStats = Arc::new(Mutex::new(Stats::default()));

    let app = Router::new()
        .fallback(any(handle_callback))
        .layer(
            ServiceBuilder::new()
                .layer(HandleErrorLayer::new(handle_timeout_error))
                .layer(TimeoutLayer::new(Duration::from_secs(10))),
        )
        .with_state(stats);

    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .with_graceful_shutdown(shutdown_signal())
    .await?;

    Ok(())
}

async fn shutdown_signal() {
    let _ = tokio::signal::ctrl_c().await;
}

async fn handle_timeout_error(err: BoxError) -> (StatusCode, String) {
    if err.is::<tower::timeout::error::Elapsed>() {
        (StatusCode::REQUEST_TIMEOUT, "request timed out".to_string())
    } else {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("unhandled internal error: {err}"),
        )
    }
}

async fn handle_callback(
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    State(stats): State<SharedStats>,
    body: Bytes,
) -> &'static str {
    print_callback(addr, &body, &stats);
    "ok"
}

fn print_callback(addr: SocketAddr, body: &[u8], stats: &SharedStats) {
    let now = Local::now().format("%H:%M:%S");
    let rule = "━".repeat(58);
    let text = String::from_utf8_lossy(body);

    println!("{}", rule.dimmed());

    let Ok(value) = serde_json::from_str::<serde_json::Value>(&text) else {
        stats.lock().unwrap().errors += 1;
        println!("{} [{now}] callback from {addr}", icon::arrow());
        println!("{} not valid JSON", icon::warn());
        println!("{text}");
        print_tally(stats);
        println!("{}\n", rule.dimmed());
        return;
    };

    match serde_json::from_value::<StkCallbackEnvelope>(value.clone()) {
        Ok(envelope) => {
            let callback = envelope.body.stk_callback;
            let (outcome, description) = result_code::describe(callback.result_code);
            let status = status_label(description);

            {
                let mut s = stats.lock().unwrap();
                s.callbacks += 1;
                if outcome == Outcome::Success {
                    s.payments += 1;
                }
            }

            println!(
                "{} Incoming STK Callback  {}",
                icon::arrow(),
                now.to_string().dimmed()
            );
            match outcome {
                Outcome::Success => println!("  Status        {}", status.green().bold()),
                Outcome::Failure => println!("  Status        {}", status.red().bold()),
            }
            if let Some(metadata) = &callback.callback_metadata {
                println!("  Amount        KES {}", metadata_value(metadata, "Amount"));
                println!(
                    "  Receipt       {}",
                    metadata_value(metadata, "MpesaReceiptNumber")
                );
                println!(
                    "  Phone         {}",
                    mask_phone(&metadata_value(metadata, "PhoneNumber"))
                );
            } else {
                println!("  Reason        {description}");
                println!(
                    "  {}",
                    format!("Daraja says: {}", callback.result_desc).dimmed()
                );
            }
            println!("  CheckoutRequestID  {}", callback.checkout_request_id);
            println!();
            println!(
                "{}",
                serde_json::to_string_pretty(&value)
                    .unwrap_or_else(|_| text.to_string())
                    .dimmed()
            );
        }
        Err(_) => {
            stats.lock().unwrap().errors += 1;
            println!("{} [{now}] callback from {addr}", icon::arrow());
            println!("{} not a recognized STK push callback shape", icon::warn());
            println!(
                "{}",
                serde_json::to_string_pretty(&value)
                    .unwrap_or_else(|_| text.to_string())
                    .dimmed()
            );
        }
    }

    match callback_store::save(&value) {
        Ok(path) => println!("  {} saved to {}", icon::arrow(), path.display()),
        Err(e) => println!("  {} failed to save callback: {e}", icon::warn()),
    }

    print_tally(stats);
    println!("{}\n", rule.dimmed());
}

fn print_tally(stats: &SharedStats) {
    let s = stats.lock().unwrap();
    println!(
        "  {}",
        format!(
            "Totals so far: {} · {} · {}",
            pluralize(s.payments, "payment"),
            pluralize(s.callbacks, "callback"),
            pluralize(s.errors, "error"),
        )
        .dimmed()
    );
}

fn pluralize(count: u64, noun: &str) -> String {
    if count == 1 {
        format!("{count} {noun}").to_string()
    } else {
        format!("{count} {noun}s").to_string()
    }
}

/// The short label before the em dash in a ResultCode description, e.g.
/// "Cancelled — the customer pressed..." -> "CANCELLED".
fn status_label(description: &str) -> String {
    description
        .split(" — ")
        .next()
        .unwrap_or(description)
        .to_uppercase()
}

/// Masks all but the first 6 and last 2 digits of a phone number, e.g.
/// "254708374149" -> "254708••••49".
fn mask_phone(phone: &str) -> String {
    if phone.len() <= 8 {
        return phone.to_string();
    }
    let start = &phone[..6];
    let end = &phone[phone.len() - 2..];
    format!("{start}••••{end}").to_string()
}

fn metadata_value(metadata: &CallbackMetadata, name: &str) -> String {
    metadata
        .item
        .iter()
        .find(|item| item.name == name)
        .and_then(|item| item.value.as_ref())
        .map(|value| match value {
            serde_json::Value::String(s) => s.clone(),
            other => other.to_string(),
        })
        .unwrap_or_else(|| "?".to_string())
}
