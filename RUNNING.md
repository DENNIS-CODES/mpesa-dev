# Running mpesa-dev

Status: Milestone 1 (`doctor`) is implemented. `inspect`, `tunnel`, and
`replay` are wired up as subcommands but still stubs — each prints what it
will do and which milestone implements it.

## Prerequisites

- Rust (stable) and Cargo. Check with `cargo --version`.
- A [Safaricom Daraja](https://developer.safaricom.co.ke/) sandbox account
  with:
  - a consumer key and consumer secret (from a Daraja "app")
  - a test paybill/till shortcode (the sandbox default is `174379`)
  - the Lipa na M-Pesa Online passkey for that shortcode

## Configure credentials

mpesa-dev reads config from `.mpesa-dev.toml` in the current directory, then
applies any `MPESA_*` environment variables on top (env vars always win).

1. Copy the example file:

   ```sh
   cp .mpesa-dev.toml.example .mpesa-dev.toml
   ```

2. Fill in `consumer_key`, `consumer_secret`, `shortcode`, and `passkey`.
   `.mpesa-dev.toml` is gitignored — it's local only, never commit it.

   Or skip the file entirely and export env vars instead:

   ```sh
   export MPESA_CONSUMER_KEY=your-consumer-key
   export MPESA_CONSUMER_SECRET=your-consumer-secret
   export MPESA_SHORTCODE=174379
   export MPESA_PASSKEY=your-lipa-na-mpesa-passkey
   ```

   | Env var                | Overrides            |
   |-------------------------|-----------------------|
   | `MPESA_CONSUMER_KEY`    | `consumer_key`        |
   | `MPESA_CONSUMER_SECRET` | `consumer_secret`     |
   | `MPESA_SHORTCODE`       | `shortcode`           |
   | `MPESA_PASSKEY`         | `passkey`             |
   | `MPESA_CALLBACK_URL`    | `callback_url`        |
   | `MPESA_ENVIRONMENT`     | `environment`         |
   | `MPESA_INSPECT_PORT`    | `inspect_port`        |

## Build and run

```sh
cargo build
cargo run -- --help
```

Or build once and run the binary directly:

```sh
cargo build --release
./target/release/mpesa-dev --help
```

### Commands

```sh
cargo run -- doctor    # runs sandbox/config checks (Milestone 1 — implemented)
cargo run -- inspect   # will print live callbacks (Milestone 2 — stub)
cargo run -- tunnel    # will expose a public HTTPS URL (Milestone 3 — stub)
cargo run -- replay    # will resend a stored callback (Milestone 4 — stub)
```

`inspect`, `tunnel`, and `replay` currently print a short description of
what they'll do once their milestone lands — this confirms the CLI, config
loading, and subcommand wiring all work end to end.

### `doctor`

Runs seven checks, in order, and prints a colored PASS/WARN/FAIL/SKIP line
for each with a one-sentence fix on failure. Exits non-zero if any check
fails.

```sh
cargo run -- doctor
```

| Check | What it does |
|-------|---------------|
| consumer key/secret configured | Confirms `consumer_key`/`consumer_secret` are set |
| sandbox reachability | Plain HTTPS request to the Daraja base URL |
| clock skew | Compares your system clock to the `Date` header from that request; fails past ±30s, warns past ±5s |
| OAuth round trip | Fetches a real access token via `/oauth/v1/generate` |
| passkey / STK push credentials | Submits a real STK push to the sandbox test MSISDN (`254708374149`) using your `shortcode`/`passkey`; catches a wrong passkey since Daraja rejects the derived password synchronously |
| callback URL reachability | HTTP request to `callback_url`, if configured |
| HTTPS cert validity | Confirms the TLS handshake to `callback_url` succeeded (skipped/warned if not HTTPS) |

Checks that need config you haven't set (e.g. no `callback_url`, or no
`shortcode`/`passkey`) print `SKIP` instead of failing.

Example failure output (deliberately wrong credentials):

```
[FAIL] OAuth round trip
       Daraja returned an error response: HTTP 400 Bad Request: (empty response body)
       fix: double check your consumer key/secret are copied correctly from an active Daraja app
```

## Project layout

```
src/
  main.rs             entry point: parses CLI args, loads config, dispatches
  cli.rs              clap arg/subcommand definitions
  config.rs           .mpesa-dev.toml + env var loading
  error.rs            shared error type
  commands/
    doctor.rs         Milestone 1 (implemented)
    inspect.rs         Milestone 2 (stub)
    tunnel.rs          Milestone 3 (stub)
    replay.rs          Milestone 4 (stub)
  daraja/
    client.rs         OAuth token fetch + in-memory cache, STK push
    models.rs         typed Daraja request/response structs
```

## Running tests

```sh
cargo test
```

(No tests yet beyond scaffolding — add them alongside each milestone's
implementation.)
