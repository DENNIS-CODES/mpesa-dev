// Wired into the `doctor` (Milestone 1) and `inspect` (Milestone 2) commands;
// not yet called from the CLI, hence the otherwise-unused warnings.
#![allow(dead_code)]

use std::sync::Mutex;

use chrono::{DateTime, Duration, Utc};
use reqwest::StatusCode;

use crate::error::{Error, Result};

use super::models::{DarajaErrorResponse, OAuthTokenResponse, StkPushRequest, StkPushResponse};

/// How much earlier than the server's stated expiry we consider a cached
/// token stale, so a request in flight never races an actual expiry.
const TOKEN_EXPIRY_SKEW: Duration = Duration::seconds(30);

#[derive(Debug, Clone)]
struct CachedToken {
    access_token: String,
    expires_at: DateTime<Utc>,
}

/// Thin wrapper around the Daraja HTTP API: handles OAuth token caching so
/// every other subcommand can just call an endpoint method without thinking
/// about auth.
pub struct DarajaClient {
    http: reqwest::Client,
    base_url: String,
    consumer_key: String,
    consumer_secret: String,
    token_cache: Mutex<Option<CachedToken>>,
}

impl DarajaClient {
    pub fn new(
        base_url: impl Into<String>,
        consumer_key: impl Into<String>,
        consumer_secret: impl Into<String>,
    ) -> Self {
        Self {
            http: reqwest::Client::new(),
            base_url: base_url.into(),
            consumer_key: consumer_key.into(),
            consumer_secret: consumer_secret.into(),
            token_cache: Mutex::new(None),
        }
    }

    /// Returns a valid OAuth access token, reusing the cached one if it has
    /// not expired yet, and fetching a fresh one otherwise.
    pub async fn access_token(&self) -> Result<String> {
        if let Some(token) = self.cached_token() {
            return Ok(token);
        }
        self.fetch_access_token().await
    }

    fn cached_token(&self) -> Option<String> {
        let cache = self.token_cache.lock().expect("token cache lock poisoned");
        cache.as_ref().and_then(|cached| {
            if cached.expires_at - TOKEN_EXPIRY_SKEW > Utc::now() {
                Some(cached.access_token.clone())
            } else {
                None
            }
        })
    }

    /// Performs the OAuth round trip against `/oauth/v1/generate` and caches
    /// the result. Exposed separately from [`access_token`] so `doctor` can
    /// call it directly to verify the round trip without relying on cache
    /// state left over from an earlier check.
    pub async fn fetch_access_token(&self) -> Result<String> {
        let url = format!(
            "{}/oauth/v1/generate?grant_type=client_credentials",
            self.base_url
        );
        let response = self
            .http
            .get(&url)
            .basic_auth(&self.consumer_key, Some(&self.consumer_secret))
            .send()
            .await?;

        let response = Self::ensure_success(response).await?;
        let body: OAuthTokenResponse = response.json().await?;

        let expires_in: i64 = body
            .expires_in
            .parse()
            .map_err(|_| Error::Api(format!("unexpected expires_in value: {}", body.expires_in)))?;
        let expires_at = Utc::now() + Duration::seconds(expires_in);

        *self.token_cache.lock().expect("token cache lock poisoned") = Some(CachedToken {
            access_token: body.access_token.clone(),
            expires_at,
        });

        Ok(body.access_token)
    }

    /// Submits an STK push (Lipa na M-Pesa Online) request.
    pub async fn stk_push(&self, request: &StkPushRequest) -> Result<StkPushResponse> {
        let token = self.access_token().await?;
        let url = format!("{}/mpesa/stkpush/v1/processrequest", self.base_url);

        let response = self
            .http
            .post(&url)
            .bearer_auth(token)
            .json(request)
            .send()
            .await?;

        let response = Self::ensure_success(response).await?;
        Ok(response.json().await?)
    }

    /// Returns the `Date` response header from a lightweight authenticated
    /// call, used by `doctor` to measure clock skew against Safaricom's
    /// server time.
    pub async fn server_date_header(&self) -> Result<String> {
        let url = format!(
            "{}/oauth/v1/generate?grant_type=client_credentials",
            self.base_url
        );
        let response = self
            .http
            .get(&url)
            .basic_auth(&self.consumer_key, Some(&self.consumer_secret))
            .send()
            .await?;

        response
            .headers()
            .get(reqwest::header::DATE)
            .and_then(|value| value.to_str().ok())
            .map(str::to_string)
            .ok_or_else(|| Error::Api("Daraja response had no Date header".into()))
    }

    async fn ensure_success(response: reqwest::Response) -> Result<reqwest::Response> {
        if response.status() == StatusCode::OK {
            return Ok(response);
        }
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        let message = serde_json::from_str::<DarajaErrorResponse>(&body)
            .map(|err| err.description())
            .unwrap_or(body);
        Err(Error::Api(format!("HTTP {status}: {message}")))
    }
}
