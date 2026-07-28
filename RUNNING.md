# Running mpesa-dev

Status: Milestone 0 (foundations) scaffolding. `doctor`, `inspect`, `tunnel`,
and `replay` are wired up as subcommands but not yet implemented — each
prints what it will do and which milestone implements it.

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

### Commands (current scaffolding)

```sh
cargo run -- doctor    # will run sandbox/config checks (Milestone 1)
cargo run -- inspect   # will print live callbacks (Milestone 2)
cargo run -- tunnel    # will expose a public HTTPS URL (Milestone 3)
cargo run -- replay    # will resend a stored callback (Milestone 4)
```

Each currently prints a short description of what it will do once its
milestone lands — this confirms the CLI, config loading, and subcommand
wiring all work end to end.

### Verifying the Daraja client manually

The OAuth client (`src/daraja/client.rs`) isn't wired into a command yet —
that happens in Milestone 1's `doctor`. To sanity-check it against your own
sandbox credentials before then, add a temporary call in `main.rs`, e.g.:

```rust
let client = daraja::DarajaClient::new(config.base_url(), consumer_key, consumer_secret);
println!("{}", client.fetch_access_token().await?);
```

A successful run prints a bearer token string. Remove the snippet afterward
— `doctor` will make this a proper, permanent check.

## Project layout

```
src/
  main.rs             entry point: parses CLI args, loads config, dispatches
  cli.rs              clap arg/subcommand definitions
  config.rs           .mpesa-dev.toml + env var loading
  error.rs            shared error type
  commands/
    doctor.rs         Milestone 1 (stub)
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
