use futures_util::{SinkExt, StreamExt};
use mpesa_dev::tunnel_protocol::{
    ClientToRelay, ForwardedRequest, ForwardedResponse, RelayToClient,
};
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::Message as WsMessage;

use crate::config::Config;
use crate::error::{Error, Result};

/// Connects to an `mpesa-relay` over a websocket, prints the public URL it
/// hands back, and replays every forwarded request against
/// `http://127.0.0.1:{inspect_port}` — the same port `inspect` listens on.
pub async fn run(config: &Config) -> Result<()> {
    let (relay_url, relay_token) = config.require_relay()?;
    let connect_url = if relay_url.contains('?') {
        format!("{relay_url}&token={relay_token}")
    } else {
        format!("{relay_url}?token={relay_token}")
    };

    println!("mpesa-dev tunnel — connecting to {relay_url} ...");

    let (ws_stream, _) = connect_async(&connect_url)
        .await
        .map_err(|e| Error::Api(format!("failed to connect to relay: {e}")))?;
    let (mut write, mut read) = ws_stream.split();

    let local_base = format!("http://127.0.0.1:{}", config.inspect_port);
    let http = reqwest::Client::new();

    while let Some(message) = read.next().await {
        let message = message.map_err(|e| Error::Api(format!("relay connection error: {e}")))?;
        let WsMessage::Text(text) = message else {
            continue;
        };
        let Ok(relay_message) = serde_json::from_str::<RelayToClient>(&text) else {
            continue;
        };

        match relay_message {
            RelayToClient::Connected { public_url } => {
                println!("Public URL: {public_url}");
                println!("Paste this as your Daraja callback_url. Forwarding to {local_base}.");
                println!("Press Ctrl+C to stop.\n");
            }
            RelayToClient::Forward(request) => {
                let response = replay_locally(&http, &local_base, &request).await;
                let payload =
                    serde_json::to_string(&ClientToRelay::Response(response)).unwrap_or_default();
                if write.send(WsMessage::Text(payload)).await.is_err() {
                    break;
                }
            }
        }
    }

    Ok(())
}

async fn replay_locally(
    http: &reqwest::Client,
    local_base: &str,
    request: &ForwardedRequest,
) -> ForwardedResponse {
    let url = format!("{local_base}{}", request.path);
    let method =
        reqwest::Method::from_bytes(request.method.as_bytes()).unwrap_or(reqwest::Method::GET);

    let mut builder = http.request(method, &url).body(request.body.clone());
    for (name, value) in &request.headers {
        if name.eq_ignore_ascii_case("host") {
            continue;
        }
        builder = builder.header(name, value);
    }

    match builder.send().await {
        Ok(response) => {
            let status = response.status().as_u16();
            let headers = response
                .headers()
                .iter()
                .filter_map(|(k, v)| v.to_str().ok().map(|v| (k.to_string(), v.to_string())))
                .collect();
            let body = response.text().await.unwrap_or_default();
            ForwardedResponse {
                id: request.id.clone(),
                status,
                headers,
                body,
            }
        }
        Err(e) => ForwardedResponse {
            id: request.id.clone(),
            status: 502,
            headers: Vec::new(),
            body: format!("mpesa-dev tunnel: failed to reach local server at {local_base}: {e}"),
        },
    }
}
