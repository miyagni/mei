//! The wire boundary: a provider dialect builds a request body and decodes its
//! streamed chunks into `ModelEvent`s — independent of how the bytes are framed
//! (SSE, WebSocket). One impl per dialect; `OpenAiCompat` covers OpenAI and its
//! many clones.

pub mod openai;

use crate::auth::Auth;
use crate::error::{ProviderError, WireError};
use crate::event::ModelEvent;
use crate::request::ChatRequest;

pub use openai::OpenAiCompat;

/// A wire dialect: serialize a turn, and decode its streamed reply.
pub trait Wire {
    /// `Send + 'static` because the decoder is owned and driven inside the
    /// boxed, `'static` `EventStream`.
    type Decoder: Decoder + Send + 'static;

    /// Serialize a turn into an HTTP request: path, headers, JSON body. The body
    /// is identical regardless of transport — SSE vs WebSocket change only the
    /// endpoint and the stream implementation, never the request shape.
    fn build(&self, auth: &Auth, request: &ChatRequest<'_>) -> Result<WireRequest, WireError>;

    /// A fresh decoder for one response stream.
    fn decoder(&self) -> Self::Decoder;

    /// Parse this provider's error envelope — from a non-success HTTP body or a
    /// mid-stream error payload — into a structured [`ProviderError`]. Falls back
    /// to the raw `body` as the message when it doesn't match the envelope, so
    /// the error is surfaced, never hidden.
    fn parse_error(&self, body: &str) -> ProviderError;
}

/// Decodes successive stream payloads into `ModelEvent`s, holding the cross-chunk
/// state the dialect needs (a finish reason and usage arrive in different chunks).
pub trait Decoder {
    /// Decode one payload (the `data` of one SSE event). Yields zero, one, or
    /// many events.
    fn push(&mut self, payload: &str) -> Result<Vec<ModelEvent>, WireError>;

    /// Flush at end of stream. Emits the trailing `Finish` only if a finish
    /// reason was seen; a stream that ended without one (cancelled, cut) yields
    /// nothing — the absence of `Finish` is itself the "incomplete" signal.
    fn end(&mut self) -> Result<Vec<ModelEvent>, WireError>;
}

/// A transport-agnostic HTTP request.
pub struct WireRequest {
    /// Appended to the provider base URL, e.g. `/chat/completions`.
    pub path: &'static str,
    pub headers: Vec<(&'static str, String)>,
    /// JSON request body.
    pub body: String,
}
