use std::net::SocketAddr;
use std::time::Duration;

use axum::body::Bytes;
use axum::error_handling::HandleErrorLayer;
use axum::extract::ConnectInfo;
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

/// Starts a local HTTP server that accepts Daraja callbacks on any path and
/// method, pretty-prints them as they arrive, decodes ResultCode into
/// plain English for recognized STK push callbacks, and persists each one
/// to disk so `replay` has something to resend later.
pub async fn run(config: &Config) -> Result<()> {
    banner::header("Inspector");

    let bind_addr = format!("0.0.0.0:{}", config.inspect_port);
    let listener = tokio::net::TcpListener::bind(&bind_addr).await?;

    println!(
        "{} Listening on http://127.0.0.1:{} (bound to {bind_addr})",
        icon::ok(),
        config.inspect_port
    );
    println!(
        "{} Point your Daraja callback_url here (via a public tunnel) and trigger an STK push",
        icon::arrow()
    );
    println!("  Press Ctrl+C to stop\n");

    let app = Router::new().fallback(any(handle_callback)).layer(
        ServiceBuilder::new()
            .layer(HandleErrorLayer::new(handle_timeout_error))
            .layer(TimeoutLayer::new(Duration::from_secs(10))),
    );

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

async fn handle_callback(ConnectInfo(addr): ConnectInfo<SocketAddr>, body: Bytes) -> &'static str {
    print_callback(addr, &body);
    "ok"
}

fn print_callback(addr: SocketAddr, body: &[u8]) {
    let now = Local::now().format("%H:%M:%S");
    let rule = "─".repeat(60);

    println!("{}", rule.dimmed());
    println!("{} [{now}] callback from {addr}", icon::arrow());

    let text = String::from_utf8_lossy(body);
    let Ok(value) = serde_json::from_str::<serde_json::Value>(&text) else {
        println!("{} not valid JSON", icon::warn());
        println!("{text}");
        println!("{}\n", rule.dimmed());
        return;
    };

    println!(
        "{}",
        serde_json::to_string_pretty(&value).unwrap_or_else(|_| text.to_string())
    );

    match serde_json::from_value::<StkCallbackEnvelope>(value.clone()) {
        Ok(envelope) => {
            let callback = envelope.body.stk_callback;
            let (outcome, description) = result_code::describe(callback.result_code);
            let line = format!("ResultCode {}: {description}", callback.result_code);
            println!();
            println!("CheckoutRequestID: {}", callback.checkout_request_id);
            match outcome {
                Outcome::Success => println!("{} {}", icon::ok(), line.green().bold()),
                Outcome::Failure => println!("{} {}", icon::fail(), line.red().bold()),
            }
            println!("  Daraja says: {}", callback.result_desc);
            if let Some(metadata) = &callback.callback_metadata {
                println!("  {}", format_metadata(metadata));
            }
        }
        Err(_) => {
            println!();
            println!("{} not a recognized STK push callback shape", icon::warn());
        }
    }

    match callback_store::save(&value) {
        Ok(path) => println!("  {} saved to {}", icon::arrow(), path.display()),
        Err(e) => println!("  {} failed to save callback: {e}", icon::warn()),
    }

    println!("{}\n", rule.dimmed());
}

fn format_metadata(metadata: &CallbackMetadata) -> String {
    let get = |name: &str| {
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
    };

    format!(
        "Amount: {}   Receipt: {}   Phone: {}",
        get("Amount"),
        get("MpesaReceiptNumber"),
        get("PhoneNumber")
    )
}
