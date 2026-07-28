use anyhow::{bail, Context, Result};
use clap::{Args, Subcommand};
use colored::Colorize;
use serde::{Deserialize, Serialize};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
};
use uuid::Uuid;

/// Arguments for the `tunnel` subcommand.
#[derive(Debug, Args)]
pub struct TunnelArgs {
    #[command(subcommand)]
    pub command: Option<TunnelCommand>,

    /// Local port to tunnel (e.g. 8080)
    #[arg(short, long, env = "MPESA_TUNNEL_PORT")]
    pub port: Option<u16>,

    /// Relay server address (host:port)
    #[arg(long, default_value = "bore.pub:7835", env = "MPESA_TUNNEL_SERVER")]
    pub server: String,

    /// Optional secret for authenticating with the relay server
    #[arg(long, env = "MPESA_TUNNEL_SECRET")]
    pub secret: Option<String>,
}

#[derive(Debug, Subcommand)]
pub enum TunnelCommand {
    /// Start a relay server that clients can connect to
    Serve(ServeArgs),
}

/// Arguments for `tunnel serve`.
#[derive(Debug, Args)]
pub struct ServeArgs {
    /// Address to bind the relay on (e.g. 0.0.0.0:7835)
    #[arg(long, default_value = "0.0.0.0:7835")]
    pub bind: String,

    /// Optional secret that clients must provide
    #[arg(long)]
    pub secret: Option<String>,
}

// ─── Bore-compatible wire protocol ────────────────────────────────────────────
//
// Each message is framed as:
//   [u32 big-endian length][JSON bytes]

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ClientMessage {
    Hello(u16),
    Accept(Uuid),
    Authenticate(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ServerMessage {
    Hello(u16),
    Heartbeat,
    Connection(Uuid),
    Error(String),
}

async fn send_msg<T: Serialize>(stream: &mut TcpStream, msg: &T) -> Result<()> {
    let payload = serde_json::to_vec(msg)?;
    let len = payload.len() as u32;
    stream.write_all(&len.to_be_bytes()).await?;
    stream.write_all(&payload).await?;
    stream.flush().await?;
    Ok(())
}

async fn recv_msg<T: for<'de> Deserialize<'de>>(stream: &mut TcpStream) -> Result<T> {
    let mut len_buf = [0u8; 4];
    stream.read_exact(&mut len_buf).await?;
    let len = u32::from_be_bytes(len_buf) as usize;
    if len > 1024 * 1024 {
        bail!("Message too large ({} bytes)", len);
    }
    let mut buf = vec![0u8; len];
    stream.read_exact(&mut buf).await?;
    Ok(serde_json::from_slice(&buf)?)
}

// ─── Client mode ──────────────────────────────────────────────────────────────

pub async fn run(args: TunnelArgs) -> Result<()> {
    match args.command {
        Some(TunnelCommand::Serve(serve_args)) => run_server(serve_args).await,
        None => {
            let local_port = args.port.context(
                "Specify a local port with --port (e.g. `mpesa-dev tunnel --port 8080`)",
            )?;
            run_client(&args.server, local_port, args.secret.as_deref()).await
        }
    }
}

async fn run_client(server: &str, local_port: u16, secret: Option<&str>) -> Result<()> {
    println!("\n{}", "mpesa-dev tunnel".bold());
    println!("  Connecting to relay server {}…", server.cyan());

    let mut ctrl = TcpStream::connect(server)
        .await
        .with_context(|| format!("Cannot connect to relay server {}", server))?;

    // Authenticate if a secret is required.
    if let Some(s) = secret {
        send_msg(&mut ctrl, &ClientMessage::Authenticate(s.to_string())).await?;
    }

    // Say hello with the desired local port (0 = let server pick).
    send_msg(&mut ctrl, &ClientMessage::Hello(local_port)).await?;

    let assigned_port = match recv_msg::<ServerMessage>(&mut ctrl).await? {
        ServerMessage::Hello(p) => p,
        ServerMessage::Error(e) => bail!("Relay server error: {}", e),
        msg => bail!("Unexpected server message: {:?}", msg),
    };

    let host = server.split(':').next().unwrap_or(server);
    println!("\n  {} Tunnel is live:\n", "✓".green().bold());
    println!(
        "    {}{}",
        "Public URL: ".bold(),
        format!("http://{}:{}", host, assigned_port).cyan().bold()
    );
    println!(
        "    {}{}",
        "Local:      ".bold(),
        format!("http://localhost:{}", local_port).cyan()
    );
    println!();
    println!(
        "  {}",
        "Set this as your M-Pesa callback URL on the Daraja portal.".dimmed()
    );
    println!("  {}", "Press Ctrl+C to stop the tunnel.\n".dimmed());

    let server_owned = server.to_string();

    loop {
        match recv_msg::<ServerMessage>(&mut ctrl).await {
            Ok(ServerMessage::Connection(id)) => {
                let server_clone = server_owned.clone();
                let secret_clone = secret.map(|s| s.to_string());
                tokio::spawn(async move {
                    if let Err(e) =
                        proxy_connection(&server_clone, local_port, id, secret_clone.as_deref())
                            .await
                    {
                        eprintln!(
                            "  {} Connection {} failed: {}",
                            "!".red(),
                            &id.to_string()[..8],
                            e
                        );
                    }
                });
            }
            Ok(ServerMessage::Heartbeat) => {}
            Ok(ServerMessage::Error(e)) => {
                bail!("Relay server error: {}", e);
            }
            Err(e) => {
                bail!("Lost connection to relay server: {}", e);
            }
            Ok(msg) => {
                eprintln!("Unexpected message: {:?}", msg);
            }
        }
    }
}

async fn proxy_connection(
    server: &str,
    local_port: u16,
    id: Uuid,
    secret: Option<&str>,
) -> Result<()> {
    let mut remote = TcpStream::connect(server)
        .await
        .with_context(|| format!("Cannot connect to relay for proxy: {}", server))?;

    if let Some(s) = secret {
        send_msg(&mut remote, &ClientMessage::Authenticate(s.to_string())).await?;
    }
    send_msg(&mut remote, &ClientMessage::Accept(id)).await?;

    let local_addr = format!("127.0.0.1:{}", local_port);
    let mut local = TcpStream::connect(&local_addr)
        .await
        .with_context(|| format!("Cannot connect to local server at {}", local_addr))?;

    // Bidirectional copy
    tokio::io::copy_bidirectional(&mut remote, &mut local).await?;
    Ok(())
}

// ─── Server mode ──────────────────────────────────────────────────────────────

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::oneshot;
use tokio::sync::Mutex;

struct RelayState {
    secret: Option<String>,
    /// pending_conns maps connection id → oneshot sender that carries the
    /// remote (public-side) TcpStream to the waiting client proxy task.
    pending: HashMap<Uuid, oneshot::Sender<TcpStream>>,
}

pub async fn run_server(args: ServeArgs) -> Result<()> {
    println!("\n{}", "mpesa-dev tunnel serve".bold());
    println!("  Relay server binding on {}…\n", args.bind.cyan());

    let listener = TcpListener::bind(&args.bind)
        .await
        .with_context(|| format!("Cannot bind to {}", args.bind))?;

    let state = Arc::new(Mutex::new(RelayState {
        secret: args.secret.clone(),
        pending: HashMap::new(),
    }));

    println!(
        "  {} Relay server ready on {}\n",
        "✓".green().bold(),
        args.bind.cyan()
    );

    loop {
        let (stream, peer) = listener.accept().await?;
        let state_clone = Arc::clone(&state);
        tokio::spawn(async move {
            if let Err(e) = handle_relay_connection(stream, peer, state_clone).await {
                eprintln!("  Connection from {} error: {}", peer, e);
            }
        });
    }
}

async fn handle_relay_connection(
    mut stream: TcpStream,
    peer: std::net::SocketAddr,
    state: Arc<Mutex<RelayState>>,
) -> Result<()> {
    let first_msg: ClientMessage = recv_msg(&mut stream).await?;

    match first_msg {
        ClientMessage::Authenticate(provided) => {
            // Verify secret then expect the next message.
            {
                let st = state.lock().await;
                if let Some(ref expected) = st.secret {
                    if &provided != expected {
                        send_msg(&mut stream, &ServerMessage::Error("Bad secret".into())).await?;
                        return Ok(());
                    }
                }
            }
            let second: ClientMessage = recv_msg(&mut stream).await?;
            handle_authenticated(stream, peer, second, state).await
        }
        msg => {
            // No auth required — handle directly.
            {
                let st = state.lock().await;
                if st.secret.is_some() {
                    send_msg(
                        &mut stream,
                        &ServerMessage::Error("Authentication required".into()),
                    )
                    .await?;
                    return Ok(());
                }
            }
            handle_authenticated(stream, peer, msg, state).await
        }
    }
}

async fn handle_authenticated(
    mut stream: TcpStream,
    peer: std::net::SocketAddr,
    msg: ClientMessage,
    state: Arc<Mutex<RelayState>>,
) -> Result<()> {
    match msg {
        ClientMessage::Hello(requested_port) => {
            // This is a control connection — assign a public port.
            let public_listener = if requested_port == 0 {
                TcpListener::bind("0.0.0.0:0").await?
            } else {
                TcpListener::bind(format!("0.0.0.0:{}", requested_port))
                    .await
                    .unwrap_or(TcpListener::bind("0.0.0.0:0").await?)
            };
            let assigned = public_listener.local_addr()?.port();

            send_msg(&mut stream, &ServerMessage::Hello(assigned)).await?;
            println!("  Client {} → public port {}", peer, assigned);

            // Accept public connections and notify the client.
            loop {
                tokio::select! {
                    accept_res = public_listener.accept() => {
                        match accept_res {
                            Ok((public_stream, _public_peer)) => {
                                let id = Uuid::new_v4();
                                let (tx, rx) = oneshot::channel::<TcpStream>();
                                {
                                    let mut st = state.lock().await;
                                    st.pending.insert(id, tx);
                                }
                                // Tell the client a new connection arrived.
                                if let Err(e) = send_msg(&mut stream, &ServerMessage::Connection(id)).await {
                                    eprintln!("Control stream error: {}", e);
                                    break;
                                }
                                // Wait for the client's proxy stream, then splice.
                                tokio::spawn(async move {
                                    if let Ok(mut client_proxy) = rx.await {
                                        let mut ps = public_stream;
                                        let _ = tokio::io::copy_bidirectional(&mut ps, &mut client_proxy).await;
                                    }
                                });
                            }
                            Err(e) => {
                                eprintln!("Public accept error: {}", e);
                                break;
                            }
                        }
                    }
                    // Heartbeat every 30 s to keep the control connection alive.
                    _ = tokio::time::sleep(std::time::Duration::from_secs(30)) => {
                        if send_msg(&mut stream, &ServerMessage::Heartbeat).await.is_err() {
                            break;
                        }
                    }
                }
            }
        }
        ClientMessage::Accept(id) => {
            // This is a proxy data connection.
            let tx = {
                let mut st = state.lock().await;
                st.pending.remove(&id)
            };
            if let Some(sender) = tx {
                let _ = sender.send(stream);
            } else {
                eprintln!("Accept for unknown id {}", id);
            }
        }
        ClientMessage::Authenticate(_) => {
            send_msg(
                &mut stream,
                &ServerMessage::Error("Unexpected authenticate".into()),
            )
            .await?;
        }
    }
    Ok(())
}
