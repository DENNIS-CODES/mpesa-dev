# mpesa-dev

A single-binary CLI for M-Pesa Daraja local development. Diagnose config, inspect live callbacks, tunnel to localhost, and replay payloads. No Node, no ngrok.

## Status

Milestone 1 (`doctor`) is implemented. `inspect`, `tunnel`, and `replay` are
wired up as subcommands but not yet implemented.

## Quick start

```sh
cp .mpesa-dev.toml.example .mpesa-dev.toml   # then fill in your sandbox creds
cargo build
cargo run -- --help
```

See [RUNNING.md](RUNNING.md) for full setup, configuration, and command
details.

## Commands

| Command   | Milestone | Status | Purpose |
|-----------|-----------|--------|---------|
| `doctor`  | 1         | done   | Sequential sandbox/config checks (credentials, OAuth round trip, passkey/STK push, callback reachability, HTTPS cert, clock skew) with pass/fail/warn and suggested fixes |
| `inspect` | 2         | stub   | Local server that prints incoming Daraja callbacks live, decoding ResultCode into plain English |
| `tunnel`  | 3         | stub   | Public HTTPS URL for your local callback endpoint, no ngrok required |
| `replay`  | 4         | stub   | Resend a stored callback, with delay/duplicate/corrupt options |
