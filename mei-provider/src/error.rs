use thiserror::Error;

/// Errors from the credential store. Only appears at a real boundary: file io
/// and corrupt persisted data.
#[derive(Debug, Error)]
pub enum AuthError {
    #[error("auth store io: {0}")]
    Io(#[from] std::io::Error),

    #[error("invalid auth store json: {0}")]
    Json(#[from] serde_json::Error),
}
