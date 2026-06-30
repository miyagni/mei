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

pub use auth::Auth;
pub use catalog::{Model, Provider};
pub use credential::{Credential, OAuthToken};
pub use error::AuthError;
pub use event::{FinishReason, ModelEvent, Usage};
pub use request::{ChatRequest, Message, Role, Tool};
pub use store::AuthStore;
pub use transport::{EventStream, Transport, TransportError};
