//! mei-provider: the providers API. Adapts an external protocol (HTTP/SSE/WS)
//! into a common ModelEvent, and handles auth and the model catalog.

mod catalog;
mod credential;
mod error;
mod store;

pub use catalog::{Model, Provider};
pub use credential::{Credential, OAuthToken};
pub use error::AuthError;
pub use store::AuthStore;
