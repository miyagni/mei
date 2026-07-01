use reqwest::StatusCode;
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

/// A failure decoding a provider's wire format into `ModelEvent`s.
#[derive(Debug, Error)]
pub enum WireError {
    #[error("decoding provider json: {0}")]
    Json(#[from] serde_json::Error),
}

/// A failure moving bytes to/from a provider over a transport (SSE today,
/// WebSocket later).
#[derive(Debug, Error)]
pub enum TransportError {
    #[error("http: {0}")]
    Http(#[from] reqwest::Error),
    #[error("event stream framing: {0}")]
    Frame(#[from] eventsource_stream::EventStreamError<reqwest::Error>),
    #[error("provider returned {status}: {body}")]
    Status { status: StatusCode, body: String },
}

/// Either layer of the streaming pipeline failed: the transport, or the wire
/// decoding its bytes.
#[derive(Debug, Error)]
pub enum StreamError {
    #[error(transparent)]
    Transport(#[from] TransportError),
    #[error(transparent)]
    Wire(#[from] WireError),
}
