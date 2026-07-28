use anyhow::{Context, Result};
use base64::{engine::general_purpose::STANDARD as B64, Engine as _};
use chrono::Utc;
use clap::Args;
use colored::Colorize;
use reqwest::Client;
use serde::Deserialize;

use crate::output::{print_check, print_header, print_hint};

/// Arguments for the `doctor` subcommand.
#[derive(Debug, Args)]
pub struct DoctorArgs {
    /// Daraja consumer key (falls back to MPESA_CONSUMER_KEY env var)
    #[arg(long, env = "MPESA_CONSUMER_KEY")]
    pub consumer_key: Option<String>,

    /// Daraja consumer secret (falls back to MPESA_CONSUMER_SECRET env var)
    #[arg(long, env = "MPESA_CONSUMER_SECRET")]
    pub consumer_secret: Option<String>,

    /// Callback URL to check for reachability
    #[arg(long, env = "MPESA_CALLBACK_URL")]
    pub callback_url: Option<String>,

    /// Use the production Daraja API instead of sandbox
    #[arg(long, default_value_t = false)]
    pub production: bool,
}

#[derive(Debug, Deserialize)]
struct OAuthResponse {
    access_token: Option<String>,
    #[allow(dead_code)]
    error: Option<String>,
}

/// Run all doctor checks.
pub async fn run(args: DoctorArgs) -> Result<()> {
    println!("{}", "\nmpesa-dev doctor\n".bold());

    let base_url = if args.production {
        "https://api.safaricom.co.ke"
    } else {
        "https://sandbox.safaricom.co.ke"
    };

    let environment = if args.production {
        "production"
    } else {
        "sandbox"
    };

    let mut all_ok = true;

    // --- Credentials check ---
    print_header("Credentials");
    let key = args.consumer_key.clone().unwrap_or_default();
    let secret = args.consumer_secret.clone().unwrap_or_default();

    let key_ok = !key.is_empty();
    let secret_ok = !secret.is_empty();

    all_ok &= key_ok;
    all_ok &= secret_ok;

    print_check(
        "MPESA_CONSUMER_KEY is set",
        key_ok,
        if key_ok {
            ""
        } else {
            "Set --consumer-key or export MPESA_CONSUMER_KEY=..."
        },
    );
    if !key_ok {
        print_hint("export MPESA_CONSUMER_KEY=<your key from Daraja portal>");
    }

    print_check(
        "MPESA_CONSUMER_SECRET is set",
        secret_ok,
        if secret_ok {
            ""
        } else {
            "Set --consumer-secret or export MPESA_CONSUMER_SECRET=..."
        },
    );
    if !secret_ok {
        print_hint("export MPESA_CONSUMER_SECRET=<your secret from Daraja portal>");
    }

    // --- Sandbox/Production reachability ---
    print_header(&format!("Daraja {} reachability", environment));

    let client = Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .context("Failed to build HTTP client")?;

    let sandbox_reachable = client.get(base_url).send().await.is_ok();
    all_ok &= sandbox_reachable;
    print_check(
        &format!("Daraja {} reachable ({})", environment, base_url),
        sandbox_reachable,
        if sandbox_reachable {
            ""
        } else {
            "Check your network or VPN settings"
        },
    );
    if !sandbox_reachable {
        print_hint("Ensure you can reach sandbox.safaricom.co.ke on port 443");
    }

    // --- OAuth round trip ---
    print_header("OAuth");

    if key_ok && secret_ok {
        let token_url = format!(
            "{}/oauth/v1/generate?grant_type=client_credentials",
            base_url
        );
        let credentials = format!("{}:{}", key, secret);
        let encoded = B64.encode(credentials.as_bytes());

        let resp = client
            .get(&token_url)
            .header("Authorization", format!("Basic {}", encoded))
            .send()
            .await;

        match resp {
            Ok(r) => {
                let status = r.status();
                if status.is_success() {
                    let body: OAuthResponse = r.json().await.unwrap_or(OAuthResponse {
                        access_token: None,
                        error: Some("Failed to parse response".into()),
                    });
                    let token_ok = body.access_token.is_some();
                    all_ok &= token_ok;
                    print_check(
                        "OAuth token obtained",
                        token_ok,
                        if token_ok {
                            "Credentials are valid"
                        } else {
                            "Unexpected response format"
                        },
                    );
                } else {
                    all_ok = false;
                    let detail = format!("HTTP {}", status);
                    print_check("OAuth token obtained", false, &detail);
                    print_hint(
                        "Verify consumer key and secret on https://developer.safaricom.co.ke",
                    );
                }
            }
            Err(e) => {
                all_ok = false;
                print_check("OAuth token obtained", false, &e.to_string());
                print_hint("Check network connectivity to Daraja");
            }
        }
    } else {
        print_check(
            "OAuth token obtained",
            false,
            "Skipped — credentials not provided",
        );
    }

    // --- Callback reachability ---
    print_header("Callback URL");

    if let Some(ref url) = args.callback_url {
        let scheme_ok = url.starts_with("https://");
        all_ok &= scheme_ok;
        print_check(
            "Callback URL uses HTTPS",
            scheme_ok,
            if scheme_ok {
                url.as_str()
            } else {
                "Daraja requires HTTPS callbacks"
            },
        );
        if !scheme_ok {
            print_hint("Use `mpesa-dev tunnel` to get a free HTTPS tunnel URL");
        }

        let reachable = client
            .head(url.as_str())
            .timeout(std::time::Duration::from_secs(8))
            .send()
            .await
            .map(|r| r.status().as_u16() < 500)
            .unwrap_or(false);

        all_ok &= reachable;
        print_check(
            "Callback URL is reachable",
            reachable,
            if reachable {
                ""
            } else {
                "Could not reach the callback URL"
            },
        );
        if !reachable {
            print_hint("Run `mpesa-dev tunnel --port <local-port>` to expose your local server");
        }
    } else {
        print_check("Callback URL configured", false, "Not provided");
        print_hint("Pass --callback-url or set MPESA_CALLBACK_URL");
    }

    // --- Clock skew ---
    print_header("System clock");

    let clock_ok = check_clock_skew(&client).await;
    all_ok &= clock_ok;
    print_check(
        "System clock within 5 minutes of network time",
        clock_ok,
        if clock_ok {
            ""
        } else {
            "Daraja rejects requests with large clock skew"
        },
    );
    if !clock_ok {
        print_hint("Sync your system clock: `sudo timedatectl set-ntp true` (Linux) or check Date & Time settings");
    }

    // --- Summary ---
    println!();
    if all_ok {
        println!("{}", "All checks passed.".green().bold());
    } else {
        println!(
            "{}",
            "Some checks failed. Fix the items marked FAIL above."
                .red()
                .bold()
        );
    }
    println!();

    Ok(())
}

/// Check that the local system clock is within 5 minutes of an NTP response.
///
/// We use the `Date` header from a TLS request to a known server as a rough
/// proxy for network time.  This avoids a hard dependency on an NTP library.
async fn check_clock_skew(client: &Client) -> bool {
    // Fetch the Date header from a reliable HTTPS server.
    let resp = match client
        .head("https://www.google.com")
        .timeout(std::time::Duration::from_secs(5))
        .send()
        .await
    {
        Ok(r) => r,
        Err(_) => return true, // Can't reach internet — assume clock is fine
    };

    if let Some(date_hdr) = resp.headers().get("date") {
        if let Ok(date_str) = date_hdr.to_str() {
            // Parse RFC 2822 date
            if let Ok(server_time) = chrono::DateTime::parse_from_rfc2822(date_str) {
                let local_now = Utc::now();
                let skew = (local_now - server_time.with_timezone(&Utc))
                    .num_seconds()
                    .abs();
                return skew < 300; // 5 minutes
            }
        }
    }

    true // If we can't parse the date, assume clock is fine
}
