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

**The missing local development toolkit for M-Pesa Daraja.**

Every Daraja tutorial tells you to bolt on ngrok for callback testing and Google `ResultCode` values at 2am. `mpesa-dev` fixes both. One static binary — no Node, no `npm install`, no version drift between your machine and the tutorial you copied from.

## What it does

- **`doctor`** — checks your whole Daraja sandbox setup (credentials, OAuth, a real test payment, your callback URL) in one shot and tells you exactly what's broken and how to fix it — no more guessing which of five things is misconfigured
- **`inspect`** — a local server that shows every incoming M-Pesa callback live in your terminal, decoded into plain English (`ResultCode 1032` → "Cancelled — the customer pressed cancel"), instead of a wall of raw JSON
- **`tunnel`** — a public HTTPS URL that forwards straight to your machine, so Safaricom can actually reach `localhost` — no separate ngrok install
- **`replay`** — resend a callback you've already captured, on demand, with delay/duplicate/corrupt options — test your error handling without waiting on Safaricom's sandbox every time

## Quick start

```sh
mpesa-dev
```

That alone gets you the banner and an arrow-key menu to pick what to run — no flags to memorize on day one.

```
Welcome to mpesa-dev.

███╗   ███╗
████╗ ████║
██╔████╔██║
██║╚██╔╝██║
██║ ╚═╝ ██║
╚═╝     ╚═╝

M-Pesa Developer Toolkit v0.1.0
Environment: sandbox

? Choose a command (↑/↓, Enter) ›
❯ doctor   Run sandbox connectivity and config checks
  inspect  Start a local server that prints incoming Daraja callbacks live
  tunnel   Expose a local port to the internet so Daraja can reach your callback URL
  replay   Resend a previously captured callback to a local endpoint
```

## Install

```sh
# curl install script (Linux/macOS, downloads a prebuilt binary — fastest)
curl -fsSL https://raw.githubusercontent.com/DENNIS-CODES/mpesa-dev/main/scripts/install.sh | sh

# cargo install (published on crates.io — needs Rust, works on any platform Rust supports)
cargo install mpesa-dev

# Homebrew (once the tap is published — see packaging/homebrew/)
brew tap DENNIS-CODES/tap && brew install mpesa-dev
```

New to Rust or unsure which to pick? Use the curl script — it just downloads a ready-to-run binary, nothing to compile. `cargo install mpesa-dev` pulls the [crates.io package](https://crates.io/crates/mpesa-dev) and builds both `mpesa-dev` and `mpesa-relay` locally. See [RUNNING.md](RUNNING.md#packaging--installing) for details on each path.

## Using it

**1. Get your Daraja sandbox credentials.** Register a test app at the [Safaricom Daraja portal](https://developer.safaricom.co.ke/) — you'll get a consumer key/secret, a test shortcode, and a passkey.

**2. Configure them:**

```sh
cp .mpesa-dev.toml.example .mpesa-dev.toml
# edit the file: paste in your consumer_key, consumer_secret, shortcode, passkey
```

(Environment variables work too — see [RUNNING.md](RUNNING.md) for the full list.)

**3. Check everything works:**

```sh
mpesa-dev doctor
```

This is the command to run first, always. It confirms your credentials are valid, does a real OAuth round trip, submits an actual test payment to the sandbox, and checks your callback URL — with a one-sentence fix for anything that's wrong.

**4. Watch callbacks land, live:**

```sh
mpesa-dev inspect
```

Leave this running while you build. Every callback that arrives gets pretty-printed and decoded — amount, receipt number, and what the result actually means, not just a raw status code.

**5. Get a public URL for Safaricom to call back to:**

Safaricom's sandbox can't reach `localhost` directly, so you need a public URL pointing at `inspect`. Either use your own tunnel (ngrok, etc.) or `mpesa-dev tunnel`, which needs an `mpesa-relay` deployed somewhere first (see [RUNNING.md](RUNNING.md) for a from-scratch VPS setup).

**6. Replay a callback instead of triggering a new one every time:**

```sh
mpesa-dev replay
```

Lists everything `inspect` has captured. Pick one to resend it — handy for iterating on your own callback handler without paying Safaricom's sandbox another round trip each time.

## Full documentation

[RUNNING.md](RUNNING.md) has the complete reference: every config option and environment variable, what each `doctor` check actually does, deploying `mpesa-relay`, and the project's internal layout for anyone contributing.

## License

MIT
