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

/// A structured error reported by a provider, parsed from its error envelope so
/// the harness renders a clean message (and can branch) instead of dumping raw
/// JSON. Each `Wire` parses its own envelope into this (see `Wire::parse_error`).
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct ProviderError {
    /// The human-facing message — what to show. Falls back to the raw body when
    /// the envelope can't be parsed (surfaced, never hidden).
    pub message: String,
    /// Provider error type, for branching: `rate_limit_error`,
    /// `invalid_request_error`, …
    pub kind: Option<String>,
    /// Provider error code, when present.
    pub code: Option<String>,
    /// Provider request id (e.g. Anthropic `request_id`), for support/diagnostics.
    pub request_id: Option<String>,
}

/// A failure decoding a provider's wire format into `ModelEvent`s.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum WireError {
    #[error("decoding provider json: {0}")]
    Json(#[from] serde_json::Error),
    /// The provider sent an error payload mid-stream (after HTTP 200). Surfaced
    /// as an error, never swallowed as an empty chunk.
    #[error("provider error: {}", .0.message)]
    Provider(ProviderError),
    /// A tool call finished assembling without an id or name — the provider's
    /// tool-call stream was inconsistent.
    #[error("incomplete tool call from provider: {0}")]
    IncompleteToolCall(String),
}

/// A failure moving bytes to/from a provider over a transport (SSE today,
/// WebSocket later).
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum TransportError {
    #[error("http: {0}")]
    Http(#[from] reqwest::Error),
    #[error("event stream framing: {0}")]
    Frame(#[from] eventsource_stream::EventStreamError<reqwest::Error>),
    /// A non-success HTTP status, with the provider's error parsed from the body.
    #[error("provider returned {status}: {}", .error.message)]
    Status {
        status: StatusCode,
        error: ProviderError,
    },
}

/// Either layer of the streaming pipeline failed: the transport, or the wire
/// decoding its bytes.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum StreamError {
    #[error(transparent)]
    Transport(#[from] TransportError),
    #[error(transparent)]
    Wire(#[from] WireError),
}
