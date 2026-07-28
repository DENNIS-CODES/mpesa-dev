use base64::Engine;
use chrono::{DateTime, Utc};
use colored::Colorize;

use crate::config::Config;
use crate::daraja::models::StkPushRequest;
use crate::daraja::DarajaClient;
use crate::error::Result;
use mpesa_dev::banner::{self, icon};

/// Well-known Daraja sandbox test MSISDN, safe to use for STK push checks —
/// see https://developer.safaricom.co.ke/Documentation.
const SANDBOX_TEST_PHONE: &str = "254708374149";

enum Status {
    Pass,
    Warn,
    Fail,
    Skip,
}

struct CheckResult {
    name: &'static str,
    status: Status,
    detail: String,
    fix: Option<String>,
}

impl CheckResult {
    fn pass(name: &'static str, detail: impl Into<String>) -> Self {
        Self {
            name,
            status: Status::Pass,
            detail: detail.into(),
            fix: None,
        }
    }

    fn warn(name: &'static str, detail: impl Into<String>, fix: impl Into<String>) -> Self {
        Self {
            name,
            status: Status::Warn,
            detail: detail.into(),
            fix: Some(fix.into()),
        }
    }

    fn fail(name: &'static str, detail: impl Into<String>, fix: impl Into<String>) -> Self {
        Self {
            name,
            status: Status::Fail,
            detail: detail.into(),
            fix: Some(fix.into()),
        }
    }

    fn skip(name: &'static str, detail: impl Into<String>) -> Self {
        Self {
            name,
            status: Status::Skip,
            detail: detail.into(),
            fix: None,
        }
    }

    fn print(&self) {
        let icon = match self.status {
            Status::Pass => icon::ok(),
            Status::Warn => icon::warn(),
            Status::Fail => icon::fail(),
            Status::Skip => icon::skip(),
        };
        println!("{icon} {}", self.name.bold());
        println!("  {}", self.detail.dimmed());
        if let Some(fix) = &self.fix {
            println!("  {} {}", icon::arrow(), fix);
        }
    }
}

pub async fn run(config: &Config) -> Result<()> {
    banner::header("Doctor");
    println!("Checking your Daraja {} setup\n", config.environment.bold());

    let mut results = Vec::new();

    // 1. Credentials configured
    let creds = config.require_credentials();
    results.push(match &creds {
        Ok(_) => CheckResult::pass("consumer key/secret configured", "found in config/env"),
        Err(e) => CheckResult::fail(
            "consumer key/secret configured",
            e.to_string(),
            "run `cp .mpesa-dev.toml.example .mpesa-dev.toml` and fill in your Daraja app's \
             consumer key/secret, or set MPESA_CONSUMER_KEY / MPESA_CONSUMER_SECRET",
        ),
    });
    let client = creds
        .ok()
        .map(|(key, secret)| DarajaClient::new(config.base_url(), key, secret));

    // 2 & 3. Sandbox reachability + clock skew (share one plain HTTP request)
    let (reachability, skew) = check_reachability_and_clock(config.base_url()).await;
    results.push(reachability);
    results.push(skew);

    // 4. OAuth round trip
    results.push(check_oauth(client.as_ref(), config.base_url()).await);

    // 5. Passkey / STK push credentials
    results.push(check_stk_push(client.as_ref(), config).await);

    // 6 & 7. Callback URL reachability + HTTPS cert validity
    let (callback, cert) = check_callback(config.callback_url.as_deref()).await;
    results.push(callback);
    results.push(cert);

    println!();
    for result in &results {
        result.print();
        println!();
    }

    let failed = results
        .iter()
        .filter(|r| matches!(r.status, Status::Fail))
        .count();
    let passed = results
        .iter()
        .filter(|r| matches!(r.status, Status::Pass))
        .count();
    let summary = format!("{passed}/{} checks passed", results.len());
    if failed > 0 {
        println!("{} {}", icon::fail(), summary.bold());
    } else {
        println!("{} {}", icon::ok(), summary.bold());
    }

    if failed > 0 {
        std::process::exit(1);
    }
    Ok(())
}

async fn check_reachability_and_clock(base_url: &str) -> (CheckResult, CheckResult) {
    let http = reqwest::Client::new();
    let response = match http.get(base_url).send().await {
        Ok(response) => response,
        Err(e) => {
            let reachability = CheckResult::fail(
                "sandbox reachability",
                e.to_string(),
                "check your internet connection and that outbound HTTPS to Safaricom's sandbox isn't blocked by a firewall/proxy",
            );
            let skew = CheckResult::skip("clock skew", "skipped: sandbox was unreachable");
            return (reachability, skew);
        }
    };

    let reachability = CheckResult::pass(
        "sandbox reachability",
        format!("connected to {base_url} (HTTP {})", response.status()),
    );

    let skew = match response
        .headers()
        .get(reqwest::header::DATE)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| DateTime::parse_from_rfc2822(v).ok())
    {
        Some(server_time) => {
            let skew_seconds = (Utc::now() - server_time.with_timezone(&Utc)).num_seconds();
            let detail = format!("local clock is {skew_seconds}s off Safaricom's server time");
            if skew_seconds.abs() > 30 {
                CheckResult::fail(
                    "clock skew",
                    detail,
                    "sync your system clock (e.g. `sudo ntpdate -u pool.ntp.org` or enable automatic time sync); \
                     STK push timestamps more than a few minutes off will be rejected",
                )
            } else if skew_seconds.abs() > 5 {
                CheckResult::warn(
                    "clock skew",
                    detail,
                    "consider syncing your system clock to avoid intermittent timestamp validation failures",
                )
            } else {
                CheckResult::pass("clock skew", detail)
            }
        }
        None => CheckResult::warn(
            "clock skew",
            "sandbox response had no usable Date header",
            "unable to verify; if STK push timestamps get rejected, check your system clock manually",
        ),
    };

    (reachability, skew)
}

async fn check_oauth(client: Option<&DarajaClient>, base_url: &str) -> CheckResult {
    let Some(client) = client else {
        return CheckResult::skip("OAuth round trip", "skipped: no credentials configured");
    };
    match client.fetch_access_token().await {
        Ok(_) => CheckResult::pass("OAuth round trip", format!("token issued from {base_url}")),
        Err(e) => CheckResult::fail(
            "OAuth round trip",
            e.to_string(),
            "double check your consumer key/secret are copied correctly from an active Daraja app",
        ),
    }
}

async fn check_stk_push(client: Option<&DarajaClient>, config: &Config) -> CheckResult {
    let Some(client) = client else {
        return CheckResult::skip(
            "passkey / STK push credentials",
            "skipped: no credentials configured",
        );
    };
    let (Some(shortcode), Some(passkey)) = (&config.shortcode, &config.passkey) else {
        return CheckResult::skip(
            "passkey / STK push credentials",
            "skipped: shortcode/passkey not configured",
        );
    };

    let timestamp = Utc::now().format("%Y%m%d%H%M%S").to_string();
    let password = base64::engine::general_purpose::STANDARD
        .encode(format!("{shortcode}{passkey}{timestamp}"));
    let callback_url = config
        .callback_url
        .clone()
        .unwrap_or_else(|| "https://example.com/mpesa-dev-doctor-callback".to_string());

    let request = StkPushRequest {
        business_short_code: shortcode.clone(),
        password,
        timestamp,
        transaction_type: "CustomerPayBillOnline".to_string(),
        amount: 1,
        party_a: shortcode.clone(),
        party_b: shortcode.clone(),
        phone_number: SANDBOX_TEST_PHONE.to_string(),
        callback_url,
        account_reference: "mpesa-dev".to_string(),
        transaction_desc: "mpesa-dev doctor check".to_string(),
    };

    match client.stk_push(&request).await {
        Ok(response) if response.response_code == "0" => CheckResult::pass(
            "passkey / STK push credentials",
            format!(
                "sandbox accepted a test STK push (CheckoutRequestID {})",
                response.checkout_request_id
            ),
        ),
        Ok(response) => CheckResult::fail(
            "passkey / STK push credentials",
            response.response_description,
            format!("verify the passkey matches shortcode {shortcode} in the Daraja portal"),
        ),
        Err(e) => CheckResult::fail(
            "passkey / STK push credentials",
            e.to_string(),
            format!("verify the passkey matches shortcode {shortcode} in the Daraja portal"),
        ),
    }
}

async fn check_callback(callback_url: Option<&str>) -> (CheckResult, CheckResult) {
    let Some(url) = callback_url else {
        return (
            CheckResult::skip("callback URL reachability", "no callback_url configured"),
            CheckResult::skip("HTTPS cert validity", "no callback_url configured"),
        );
    };

    let is_https = url.starts_with("https://");
    let http = reqwest::Client::new();
    let response = http.get(url).send().await;

    let reachability = match &response {
        Ok(r) => CheckResult::pass(
            "callback URL reachability",
            format!("connected to {url} (HTTP {})", r.status()),
        ),
        Err(e) => CheckResult::fail(
            "callback URL reachability",
            e.to_string(),
            "make sure a server is listening at this URL and it's reachable from the public \
             internet (try `mpesa-dev inspect`, or `mpesa-dev tunnel` once available)",
        ),
    };

    let cert = if !is_https {
        CheckResult::warn(
            "HTTPS cert validity",
            "callback URL is not HTTPS",
            "Daraja requires an HTTPS callback URL; put a TLS-terminating proxy in front, or use `mpesa-dev tunnel` once available",
        )
    } else {
        match &response {
            Ok(_) => CheckResult::pass(
                "HTTPS cert validity",
                "TLS handshake and certificate chain validated",
            ),
            Err(e) => CheckResult::fail(
                "HTTPS cert validity",
                e.to_string(),
                "check that the certificate is valid, not expired, and not self-signed",
            ),
        }
    };

    (reachability, cert)
}
