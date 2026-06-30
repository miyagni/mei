use thiserror::Error;

/// Errors from the credential store. Only appears at a real boundary: config
/// dir resolution, file io, and corrupt persisted data.
#[derive(Debug, Error)]
pub enum AuthError {
    #[error("config dir: {0}")]
    Config(#[from] mei_config::ConfigError),

    #[error("auth store io: {0}")]
    Io(#[from] std::io::Error),

    #[error("invalid auth store json: {0}")]
    Json(#[from] serde_json::Error),
}
