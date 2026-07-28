# mpesa-dev

A single-binary CLI for M-Pesa Daraja local development. Diagnose config, inspect live callbacks, tunnel to localhost, and replay payloads. No Node, no ngrok.

## Status

Milestone 0 (foundations): CLI skeleton, config loading, and the Daraja
OAuth client are scaffolded. `doctor`, `inspect`, `tunnel`, and `replay` are
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

| Command   | Milestone | Purpose |
|-----------|-----------|---------|
| `doctor`  | 1         | Sequential sandbox/config checks (OAuth round trip, callback reachability, clock skew, ...) with pass/fail and suggested fixes |
| `inspect` | 2         | Local server that prints incoming Daraja callbacks live, decoding ResultCode into plain English |
| `tunnel`  | 3         | Public HTTPS URL for your local callback endpoint, no ngrok required |
| `replay`  | 4         | Resend a stored callback, with delay/duplicate/corrupt options |
