//! The transport boundary: how a built request is streamed from a provider.
//! SSE today; WebSocket later behind the same `EventStream`. Both decode to the
//! same `ModelEvent`s, so the agent loop stays transport-agnostic.

mod sse;

use std::pin::Pin;

use futures_core::Stream;

use crate::error::StreamError;
use crate::event::ModelEvent;

pub use sse::stream;

/// A streamed turn: model events as they arrive, or a failure mid-stream.
pub type EventStream = Pin<Box<dyn Stream<Item = Result<ModelEvent, StreamError>> + Send>>;
