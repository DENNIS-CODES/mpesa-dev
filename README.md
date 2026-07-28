# mpesa-dev

The missing local development toolkit for M-Pesa Daraja. One static Rust binary — no Node runtime, no npm install, no ngrok.

## Install

```bash
cargo install mpesa-dev
```

Or build from source:

```bash
git clone https://github.com/DENNIS-CODES/mpesa-dev
cd mpesa-dev
cargo build --release
```

## Commands

### `doctor` — health-check your Daraja config

Runs a pass/fail checklist across credentials, OAuth round-trip, callback
reachability, HTTPS validity, sandbox connectivity, and clock skew, then tells
you exactly how to fix whatever is broken.

```bash
export MPESA_CONSUMER_KEY=your-key
export MPESA_CONSUMER_SECRET=your-secret
export MPESA_CALLBACK_URL=https://example.com/callback

mpesa-dev doctor
# or pass flags directly
mpesa-dev doctor --consumer-key KEY --consumer-secret SECRET --callback-url https://...
# use production instead of sandbox
mpesa-dev doctor --production
```

### `inspect` — live callback viewer

Spins up a local HTTP server, shows every incoming M-Pesa callback live in
your terminal with pretty-printed JSON, and decodes every ResultCode into
plain English (1032 = user cancelled, 1037 = timeout, etc.).

```bash
mpesa-dev inspect              # listens on :9090
mpesa-dev inspect --port 8080  # custom port
mpesa-dev inspect --save callbacks.ndjson  # also save to file
```

Point your Daraja callback URL to `http://localhost:9090/callback` (or use
`mpesa-dev tunnel` to get a public HTTPS URL).

### `tunnel` — public HTTPS tunnel to localhost

Exposes a public URL that forwards straight to your local server.
Defaults to [bore.pub](https://github.com/ekzhang/bore) (free, open-source relay).

```bash
mpesa-dev tunnel --port 8080
# → Tunnel is live: http://bore.pub:XXXXX

# Use your own relay server
mpesa-dev tunnel --port 8080 --server myrelay.example.com:7835
mpesa-dev tunnel --port 8080 --server myrelay.example.com:7835 --secret mysecret

# Run your own relay server (deploy on a VPS with a public IP)
mpesa-dev tunnel serve
mpesa-dev tunnel serve --bind 0.0.0.0:7835 --secret mysecret
```

### `replay` — resend stored callbacks

Resend a stored callback with delay, duplicate, or corrupt-payload flags
to test your retry logic without waiting on Safaricom.

```bash
# Save callbacks with inspect first
mpesa-dev inspect --save callbacks.ndjson

# Replay against your local server
mpesa-dev replay callbacks.ndjson --target http://localhost:8080/callback

# Options
mpesa-dev replay callbacks.ndjson --target http://... --delay 500ms
mpesa-dev replay callbacks.ndjson --target http://... --duplicate 3     # send each payload 3 times
mpesa-dev replay callbacks.ndjson --target http://... --corrupt         # randomly corrupt fields
mpesa-dev replay callbacks.ndjson --target http://... --verbose         # print payloads before sending
```

## ResultCodes decoded

`inspect` and `replay` automatically decode M-Pesa ResultCodes:

| Code | Meaning |
|------|---------|
| 0    | Success |
| 1    | Insufficient funds |
| 1032 | Request cancelled by user |
| 1037 | DS timeout — user cannot be reached |
| 1003 | PIN retries exceeded |
| 1019 | Transaction expired |
| …    | (all standard Daraja codes supported) |

## Environment variables

| Variable | Flag | Description |
|----------|------|-------------|
| `MPESA_CONSUMER_KEY` | `--consumer-key` | Daraja consumer key |
| `MPESA_CONSUMER_SECRET` | `--consumer-secret` | Daraja consumer secret |
| `MPESA_CALLBACK_URL` | `--callback-url` / `--target` | Callback / replay target URL |
| `MPESA_INSPECT_PORT` | `--port` | Inspect server port (default 9090) |
| `MPESA_TUNNEL_PORT` | `--port` | Tunnel local port |
| `MPESA_TUNNEL_SERVER` | `--server` | Relay server address (default bore.pub:7835) |
| `MPESA_TUNNEL_SECRET` | `--secret` | Relay server secret |

## Built with

- [Axum](https://github.com/tokio-rs/axum) — async HTTP server for `inspect`
- [reqwest](https://github.com/seanmonstar/reqwest) — HTTP client for `doctor` and `replay`
- [Tokio](https://tokio.rs) — async runtime
- [clap](https://clap.rs) — CLI argument parsing
