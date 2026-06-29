//! mei-provider: the providers API. Adapts an external protocol (HTTP/SSE/WS)
//! into a common ModelEvent, and handles auth and the model catalog.

mod credential;
mod error;

pub use credential::{Credential, OAuthToken};
pub use error::AuthError;
