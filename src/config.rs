use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use crate::error::{Error, Result};

/// Configuration loaded from `.mpesa-dev.toml`, then overridden by environment
/// variables. Env vars always win so CI and local `.env`-style setups can
/// override a checked-in config file without editing it.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Config {
    pub consumer_key: Option<String>,
    pub consumer_secret: Option<String>,
    pub shortcode: Option<String>,
    pub passkey: Option<String>,
    pub callback_url: Option<String>,
    #[serde(default = "default_environment")]
    pub environment: String,
    #[serde(default = "default_inspect_port")]
    pub inspect_port: u16,
}

fn default_environment() -> String {
    "sandbox".to_string()
}

fn default_inspect_port() -> u16 {
    4321
}

impl Config {
    pub fn load() -> Result<Self> {
        Self::load_from(&Self::default_path())
    }

    pub fn load_from(path: &std::path::Path) -> Result<Self> {
        let mut config = if path.exists() {
            let contents = std::fs::read_to_string(path)?;
            toml::from_str(&contents)?
        } else {
            Config {
                environment: default_environment(),
                inspect_port: default_inspect_port(),
                ..Default::default()
            }
        };
        config.apply_env_overrides();
        Ok(config)
    }

    fn default_path() -> PathBuf {
        PathBuf::from(".mpesa-dev.toml")
    }

    fn apply_env_overrides(&mut self) {
        if let Ok(v) = std::env::var("MPESA_CONSUMER_KEY") {
            self.consumer_key = Some(v);
        }
        if let Ok(v) = std::env::var("MPESA_CONSUMER_SECRET") {
            self.consumer_secret = Some(v);
        }
        if let Ok(v) = std::env::var("MPESA_SHORTCODE") {
            self.shortcode = Some(v);
        }
        if let Ok(v) = std::env::var("MPESA_PASSKEY") {
            self.passkey = Some(v);
        }
        if let Ok(v) = std::env::var("MPESA_CALLBACK_URL") {
            self.callback_url = Some(v);
        }
        if let Ok(v) = std::env::var("MPESA_ENVIRONMENT") {
            self.environment = v;
        }
        if let Ok(v) = std::env::var("MPESA_INSPECT_PORT") {
            if let Ok(port) = v.parse() {
                self.inspect_port = port;
            }
        }
    }

    /// Base URL for the Daraja API, chosen by `environment` ("sandbox" or
    /// "production"). Anything other than "production" is treated as sandbox.
    pub fn base_url(&self) -> &'static str {
        match self.environment.as_str() {
            "production" => "https://api.safaricom.co.ke",
            _ => "https://sandbox.safaricom.co.ke",
        }
    }

    pub fn require_credentials(&self) -> Result<(String, String)> {
        match (&self.consumer_key, &self.consumer_secret) {
            (Some(key), Some(secret)) => Ok((key.clone(), secret.clone())),
            _ => Err(Error::Config(
                "missing consumer key/secret; set MPESA_CONSUMER_KEY and MPESA_CONSUMER_SECRET, \
                 or add consumer_key / consumer_secret to .mpesa-dev.toml"
                    .into(),
            )),
        }
    }
}
