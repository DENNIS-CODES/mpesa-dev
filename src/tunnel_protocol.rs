use serde::{Deserialize, Serialize};

/// Header names that must never be replayed across a proxy hop: the
/// RFC 7230 §6.1 hop-by-hop set, plus `host` since it names the wrong
/// destination on the other side of the tunnel. Checked case-insensitively.
const NON_FORWARDABLE_HEADERS: &[&str] = &[
    "connection",
    "keep-alive",
    "proxy-authenticate",
    "proxy-authorization",
    "te",
    "trailer",
    "transfer-encoding",
    "upgrade",
    "host",
];

/// Whether `name` is a header that should be dropped rather than forwarded
/// when replaying a request or response across the tunnel.
pub fn is_forwardable_header(name: &str) -> bool {
    !NON_FORWARDABLE_HEADERS
        .iter()
        .any(|h| h.eq_ignore_ascii_case(name))
}

/// Messages the relay sends down the websocket to the `tunnel` CLI client.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum RelayToClient {
    /// Sent once, immediately after the client connects, with the public
    /// URL Daraja callbacks should be pointed at.
    Connected { public_url: String },
    /// An inbound HTTP request the relay received on the client's public
    /// URL, to be replayed against the client's local server.
    Forward(ForwardedRequest),
}

/// Messages the `tunnel` CLI client sends back up the websocket to the relay.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ClientToRelay {
    Response(ForwardedResponse),
}

/// An HTTP request the relay received on the public URL, serialized so it
/// can cross the websocket and be replayed locally.
///
/// `body` assumes UTF-8 text (Daraja callbacks are JSON) — binary bodies
/// are out of scope for v1.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForwardedRequest {
    /// Correlates this request with its `ForwardedResponse`; a relay may
    /// have several requests in flight at once over one connection.
    pub id: String,
    pub method: String,
    pub path: String,
    pub headers: Vec<(String, String)>,
    pub body: String,
}

/// The client's response to a [`ForwardedRequest`], sent back to the relay
/// to complete the original HTTP request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForwardedResponse {
    pub id: String,
    pub status: u16,
    pub headers: Vec<(String, String)>,
    pub body: String,
}
