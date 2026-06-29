//! mei-session: Mei's session API.
//!
//! A session keeps the transcript as the single source of truth. Each position
//! in the transcript is an [`Entry`].

mod entry;
mod error;
mod ids;

pub use entry::Entry;
pub use error::SessionError;
pub use ids::{NodeId, SessionId};
