//! mei-provider: the providers API. Adapts an external protocol (HTTP/SSE/WS)
//! into a common ModelEvent, and handles auth and the model catalog.

mod auth;
mod catalog;
mod credential;
mod error;
mod event;
mod request;
mod store;
mod transport;
mod wire;

pub use auth::Auth;
pub use catalog::{Model, Provider};
pub use credential::{Credential, OAuthToken};
pub use error::{AuthError, ProviderError, StreamError, TransportError, WireError};
pub use event::{FinishReason, ModelEvent, Usage};
pub use request::{ChatRequest, Message, Role, Tool, ToolChoice};
pub use store::AuthStore;
pub use transport::{stream, EventStream};
pub use wire::{Decoder, OpenAiCompat, Wire, WireRequest};
