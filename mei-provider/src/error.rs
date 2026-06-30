use thiserror::Error;

/// Errors from the providers auth API: resolving auth for a request, and the
/// credential store (config dir resolution, file io, corrupt persisted data).
#[derive(Debug, Error)]
pub enum AuthError {
    /// The requested model is currently disabled. The message is the reason,
    /// shown to the user verbatim.
    #[error("{0}")]
    ModelDisabled(&'static str),

    #[error("config dir: {0}")]
    Config(#[from] mei_config::ConfigError),

    #[error("auth store io: {0}")]
    Io(#[from] std::io::Error),

    #[error("invalid auth store json: {0}")]
    Json(#[from] serde_json::Error),
}
