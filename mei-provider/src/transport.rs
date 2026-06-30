//! The transport boundary: how a turn is streamed from a provider.
//!
//! A provider implements [`Transport`] over one wire (SSE today, WebSocket
//! later); both decode to the same [`ModelEvent`] stream, so the agent loop
//! stays transport-agnostic.

use std::future::Future;
use std::pin::Pin;

use futures_core::Stream;
use thiserror::Error;

use crate::auth::Auth;
use crate::event::ModelEvent;
use crate::request::ChatRequest;

/// A streamed turn: model events as they arrive, or a failure mid-stream.
pub type EventStream = Pin<Box<dyn Stream<Item = Result<ModelEvent, TransportError>> + Send>>;

/// Streams a turn from a provider, implemented once per wire protocol.
pub trait Transport {
    /// Send `request` and stream the model's reply. The returned future
    /// resolves once the stream is open; events then arrive over the
    /// [`EventStream`].
    fn stream(
        &self,
        auth: &Auth,
        request: ChatRequest<'_>,
    ) -> impl Future<Output = Result<EventStream, TransportError>> + Send;
}

/// A failure talking to a provider, at connect time or mid-stream.
#[derive(Debug, Error)]
pub enum TransportError {
    /// The provider answered with a non-success HTTP status.
    #[error("provider returned status {code}: {body}")]
    Status { code: u16, body: String },
    /// The connection failed or the stream broke (network, timeout, reset).
    #[error("connection failed: {0}")]
    Connection(String),
    /// A streamed payload could not be decoded.
    #[error("could not decode the stream: {0}")]
    Decode(String),
}
