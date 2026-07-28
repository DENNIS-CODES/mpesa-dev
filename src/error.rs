use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("config error: {0}")]
    Config(String),

    #[error("request to Daraja failed: {0}")]
    Request(#[from] reqwest::Error),

    #[error("Daraja returned an error response: {0}")]
    Api(String),

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("failed to parse .mpesa-dev.toml: {0}")]
    TomlParse(#[from] toml::de::Error),
}

pub type Result<T> = std::result::Result<T, Error>;
