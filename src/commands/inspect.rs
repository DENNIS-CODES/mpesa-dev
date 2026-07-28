use anyhow::Result;
use axum::{
    body::Body,
    extract::State,
    http::{Request, StatusCode},
    response::IntoResponse,
    routing::any,
    Router,
};
use chrono::Local;
use clap::Args;
use colored::Colorize;
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::sync::mpsc;

use crate::result_codes;

/// Arguments for the `inspect` subcommand.
#[derive(Debug, Args)]
pub struct InspectArgs {
    /// Local port to listen on
    #[arg(short, long, default_value_t = 9090, env = "MPESA_INSPECT_PORT")]
    pub port: u16,

    /// Save received callbacks to this file (one JSON per line, NDJSON format)
    #[arg(long)]
    pub save: Option<std::path::PathBuf>,
}

struct AppState {
    tx: mpsc::UnboundedSender<ReceivedCallback>,
}

#[derive(Debug, Clone)]
struct ReceivedCallback {
    method: String,
    path: String,
    headers: Vec<(String, String)>,
    body: serde_json::Value,
}

/// Run the inspect server.
pub async fn run(args: InspectArgs) -> Result<()> {
    let (tx, mut rx) = mpsc::unbounded_channel::<ReceivedCallback>();

    let state = Arc::new(AppState { tx });

    let app = Router::new()
        .route("/{*path}", any(handle_callback))
        .route("/", any(handle_callback))
        .with_state(state);

    let addr = format!("0.0.0.0:{}", args.port);
    let listener = TcpListener::bind(&addr).await?;

    println!(
        "\n{}\n",
        format!(
            "mpesa-dev inspect — listening on http://localhost:{}",
            args.port
        )
        .bold()
    );
    println!("  {}", "Waiting for M-Pesa callbacks...".dimmed());
    println!(
        "  {}",
        format!(
            "Point your callback URL to: http://localhost:{}/callback",
            args.port
        )
        .cyan()
    );
    println!();

    let save_path = args.save.clone();

    // Spawn the printer task
    tokio::spawn(async move {
        let mut counter: u64 = 0;
        while let Some(cb) = rx.recv().await {
            counter += 1;
            print_callback(counter, &cb, save_path.as_deref()).await;
        }
    });

    axum::serve(listener, app).await?;
    Ok(())
}

async fn handle_callback(
    State(state): State<Arc<AppState>>,
    req: Request<Body>,
) -> impl IntoResponse {
    let method = req.method().to_string();
    let path = req.uri().path().to_string();

    let headers: Vec<(String, String)> = req
        .headers()
        .iter()
        .map(|(k, v)| (k.as_str().to_string(), v.to_str().unwrap_or("").to_string()))
        .collect();

    let body_bytes = axum::body::to_bytes(req.into_body(), 1024 * 1024)
        .await
        .unwrap_or_default();

    let body: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap_or(
        serde_json::Value::String(String::from_utf8_lossy(&body_bytes).to_string()),
    );

    let _ = state.tx.send(ReceivedCallback {
        method,
        path,
        headers,
        body,
    });

    (
        StatusCode::OK,
        "{\"ResultCode\":0,\"ResultDesc\":\"Accepted\"}",
    )
        .into_response()
}

async fn print_callback(counter: u64, cb: &ReceivedCallback, save_path: Option<&std::path::Path>) {
    let now = Local::now().format("%H:%M:%S%.3f");

    println!(
        "{}",
        format!(
            "─── #{} {} {} {} ───",
            counter,
            now,
            cb.method.cyan().bold(),
            cb.path.yellow()
        )
        .bold()
    );

    // Print selected headers
    for (k, v) in &cb.headers {
        let key_lower = k.to_lowercase();
        if matches!(
            key_lower.as_str(),
            "content-type" | "x-forwarded-for" | "user-agent"
        ) {
            println!("  {}: {}", k.dimmed(), v);
        }
    }

    // Pretty-print JSON body
    let pretty = serde_json::to_string_pretty(&cb.body).unwrap_or_default();
    println!("{}", pretty.green());

    // Decode ResultCode if present
    decode_result_codes_in_value(&cb.body, 0);

    // Save to file if requested
    if let Some(path) = save_path {
        use tokio::io::AsyncWriteExt;
        if let Ok(mut file) = tokio::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .await
        {
            let line = format!("{}\n", serde_json::to_string(&cb.body).unwrap_or_default());
            let _ = file.write_all(line.as_bytes()).await;
        }
    }

    println!();
}

fn decode_result_codes_in_value(value: &serde_json::Value, depth: usize) {
    if depth > 5 {
        return;
    }
    match value {
        serde_json::Value::Object(map) => {
            for (key, val) in map {
                let key_lower = key.to_lowercase();
                if key_lower.contains("resultcode") || key_lower == "errorcode" {
                    if let Some(code) = val.as_i64() {
                        let desc = result_codes::decode(code);
                        println!(
                            "  {} {} = {}",
                            "ResultCode".yellow().bold(),
                            format!("{}", code).bold(),
                            desc.cyan()
                        );
                    }
                }
                decode_result_codes_in_value(val, depth + 1);
            }
        }
        serde_json::Value::Array(arr) => {
            for item in arr {
                decode_result_codes_in_value(item, depth + 1);
            }
        }
        _ => {}
    }
}
