# mpesa-dev

```
███╗   ███╗
████╗ ████║
██╔████╔██║
██║╚██╔╝██║
██║ ╚═╝ ██║
╚═╝     ╚═╝

M-Pesa Developer Toolkit
```

A single-binary CLI for M-Pesa Daraja local development. Diagnose config, inspect live callbacks, tunnel to localhost, and replay payloads. No Node, no ngrok.

## Status

All four milestones are implemented: `doctor`, `inspect`, `tunnel` (+ `mpesa-relay`), and `replay`.

## Quick start

```sh
cp .mpesa-dev.toml.example .mpesa-dev.toml   # then fill in your sandbox creds
cargo build
cargo run
```

Running `mpesa-dev` with no arguments shows the banner and jumps straight into `inspect` — the fastest way to see something happen. See [RUNNING.md](RUNNING.md) for full setup, configuration, and command details.

## Install

```sh
# curl install script (Linux/macOS, downloads a prebuilt binary)
curl -fsSL https://raw.githubusercontent.com/DENNIS-CODES/mpesa-dev/main/scripts/install.sh | sh

# cargo install (from source, any platform Rust supports)
git clone https://github.com/DENNIS-CODES/mpesa-dev && cd mpesa-dev
cargo install --path . --bin mpesa-dev

# Homebrew (once the tap is published — see packaging/homebrew/)
brew tap DENNIS-CODES/tap && brew install mpesa-dev
```

See [RUNNING.md](RUNNING.md#packaging--installing) for details on each path, including what's actually been verified vs. what's a documented-but-untested template.

## Commands

| Command   | Milestone | Status | Purpose |
|-----------|-----------|--------|---------|
| `doctor`  | 1         | done   | Sequential sandbox/config checks (credentials, OAuth round trip, passkey/STK push, callback reachability, HTTPS cert, clock skew) with pass/fail/warn and suggested fixes |
| `inspect` | 2         | done   | Local Axum server that prints incoming Daraja callbacks live, decoding ResultCode into plain English, and persists them for `replay` |
| `tunnel`  | 3         | done   | Public HTTPS URL for your local callback endpoint via `mpesa-relay`, no ngrok required |
| `replay`  | 4         | done   | Resend a stored callback, with `--delay`/`--duplicate`/`--corrupt` options |
