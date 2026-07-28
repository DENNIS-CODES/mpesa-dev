//! The public half of `mpesa-dev tunnel`.
//!
//! Deployed once on a cheap VPS behind a wildcard DNS record and a
//! TLS-terminating reverse proxy (Caddy handles cert issuance for
//! `*.{RELAY_PUBLIC_BASE}` — this binary only ever speaks plain HTTP/WS).
//! Accepts a websocket connection per `mpesa-dev tunnel` client, hands back
//! a public subdomain, and forwards any HTTP request that arrives on that
//! subdomain down the socket for the client to replay against localhost.
//!
//! Configured entirely by environment variables:
//!   RELAY_BIND_ADDR   address to listen on (default "0.0.0.0:7000")
//!   RELAY_TOKEN       shared secret clients must present to connect (required)
//!   RELAY_PUBLIC_BASE base domain, e.g. "tunnel.example.com" (required)

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use axum::body::{Body, Bytes};
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::{Query, State};
use axum::http::header::HOST;
use axum::http::{HeaderMap, Method, StatusCode, Uri};
use axum::response::{IntoResponse, Response};
use axum::routing::{any, get};
use axum::Router;
use futures_util::{SinkExt, StreamExt};
use rand::Rng;
use serde::Deserialize;
use tokio::sync::{mpsc, oneshot};

use mpesa_dev::tunnel_protocol::{
    ClientToRelay, ForwardedRequest, ForwardedResponse, RelayToClient,
};

const FORWARD_TIMEOUT: Duration = Duration::from_secs(15);

#[derive(Clone)]
struct Tunnel {
    to_client: mpsc::UnboundedSender<RelayToClient>,
    pending: Arc<Mutex<HashMap<String, oneshot::Sender<ForwardedResponse>>>>,
}

#[derive(Clone)]
struct AppState {
    token: String,
    public_base: String,
    registry: Arc<Mutex<HashMap<String, Tunnel>>>,
}

#[tokio::main]
async fn main() {
    let bind_addr = std::env::var("RELAY_BIND_ADDR").unwrap_or_else(|_| "0.0.0.0:7000".to_string());
    let token = std::env::var("RELAY_TOKEN").unwrap_or_else(|_| {
        eprintln!(
            "RELAY_TOKEN env var is required: a shared secret the tunnel client must present"
        );
        std::process::exit(1);
    });
    let public_base = std::env::var("RELAY_PUBLIC_BASE").unwrap_or_else(|_| {
        eprintln!(
            "RELAY_PUBLIC_BASE env var is required, e.g. tunnel.example.com \
             (point a wildcard DNS record *.tunnel.example.com at this host, and put a \
             TLS-terminating reverse proxy such as Caddy in front for cert issuance)"
        );
        std::process::exit(1);
    });

    let state = AppState {
        token,
        public_base,
        registry: Arc::new(Mutex::new(HashMap::new())),
    };

    let app = Router::new()
        .route("/tunnel/ws", get(ws_handler))
        .fallback(any(http_handler))
        .with_state(state);

    println!("mpesa-relay listening on {bind_addr}");
    let listener = tokio::net::TcpListener::bind(&bind_addr)
        .await
        .unwrap_or_else(|e| panic!("failed to bind {bind_addr}: {e}"));
    axum::serve(listener, app.into_make_service())
        .await
        .expect("relay server error");
}

#[derive(Deserialize)]
struct ConnectParams {
    token: String,
}

async fn ws_handler(
    Query(params): Query<ConnectParams>,
    State(state): State<AppState>,
    ws: WebSocketUpgrade,
) -> Response {
    if params.token != state.token {
        return (StatusCode::UNAUTHORIZED, "invalid token").into_response();
    }
    ws.on_upgrade(move |socket| handle_socket(socket, state))
}

async fn handle_socket(socket: WebSocket, state: AppState) {
    let id = generate_id();
    let public_url = format!("https://{id}.{}", state.public_base);

    let (mut ws_tx, mut ws_rx) = socket.split();
    let (tx, mut rx) = mpsc::unbounded_channel::<RelayToClient>();
    let pending = Arc::new(Mutex::new(HashMap::new()));

    state.registry.lock().unwrap().insert(
        id.clone(),
        Tunnel {
            to_client: tx.clone(),
            pending: pending.clone(),
        },
    );

    let _ = tx.send(RelayToClient::Connected {
        public_url: public_url.clone(),
    });
    println!("[{id}] connected -> {public_url}");

    let send_task = tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            let Ok(text) = serde_json::to_string(&msg) else {
                continue;
            };
            if ws_tx.send(Message::Text(text.into())).await.is_err() {
                break;
            }
        }
    });

    while let Some(Ok(msg)) = ws_rx.next().await {
        if let Message::Text(text) = msg {
            if let Ok(ClientToRelay::Response(resp)) = serde_json::from_str::<ClientToRelay>(&text)
            {
                if let Some(sender) = pending.lock().unwrap().remove(&resp.id) {
                    let _ = sender.send(resp);
                }
            }
        }
    }

    send_task.abort();
    state.registry.lock().unwrap().remove(&id);
    println!("[{id}] disconnected");
}

async fn http_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    method: Method,
    uri: Uri,
    body: Bytes,
) -> Response {
    let host = headers
        .get(HOST)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    let Some(id) = tunnel_id_from_host(host, &state.public_base) else {
        return (StatusCode::NOT_FOUND, "no tunnel for this host").into_response();
    };

    let tunnel = state.registry.lock().unwrap().get(&id).cloned();
    let Some(tunnel) = tunnel else {
        return (StatusCode::BAD_GATEWAY, "tunnel client is not connected").into_response();
    };

    let request_id = generate_id();
    let (resp_tx, resp_rx) = oneshot::channel();
    tunnel
        .pending
        .lock()
        .unwrap()
        .insert(request_id.clone(), resp_tx);

    let forwarded = ForwardedRequest {
        id: request_id.clone(),
        method: method.to_string(),
        path: uri
            .path_and_query()
            .map(|p| p.as_str().to_string())
            .unwrap_or_else(|| "/".to_string()),
        headers: headers
            .iter()
            .filter(|(name, _)| **name != HOST)
            .filter_map(|(k, v)| v.to_str().ok().map(|v| (k.to_string(), v.to_string())))
            .collect(),
        body: String::from_utf8_lossy(&body).into_owned(),
    };

    if tunnel
        .to_client
        .send(RelayToClient::Forward(forwarded))
        .is_err()
    {
        tunnel.pending.lock().unwrap().remove(&request_id);
        return (StatusCode::BAD_GATEWAY, "tunnel client disconnected").into_response();
    }

    match tokio::time::timeout(FORWARD_TIMEOUT, resp_rx).await {
        Ok(Ok(response)) => {
            let mut builder = Response::builder().status(response.status);
            for (k, v) in &response.headers {
                builder = builder.header(k, v);
            }
            builder.body(Body::from(response.body)).unwrap_or_else(|_| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "failed to build response",
                )
                    .into_response()
            })
        }
        _ => {
            tunnel.pending.lock().unwrap().remove(&request_id);
            (
                StatusCode::GATEWAY_TIMEOUT,
                "tunnel client did not respond in time",
            )
                .into_response()
        }
    }
}

fn tunnel_id_from_host(host: &str, public_base: &str) -> Option<String> {
    let host_only = host.split(':').next().unwrap_or(host);
    let suffix = format!(".{public_base}");
    let prefix = host_only.strip_suffix(&suffix)?;
    if prefix.is_empty() {
        None
    } else {
        Some(prefix.to_string())
    }
}

fn generate_id() -> String {
    const CHARS: &[u8] = b"abcdefghijklmnopqrstuvwxyz0123456789";
    let mut rng = rand::thread_rng();
    (0..8)
        .map(|_| CHARS[rng.gen_range(0..CHARS.len())] as char)
        .collect()
}
