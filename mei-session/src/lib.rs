//! mei-session: Mei's session API.
//!
//! A session keeps the transcript as the single source of truth. Each position
//! in the transcript is an [`Entry`]. [`LinearSession`] is a single editable
//! path; [`TreeSession`] is an append-only graph that branches on divergence.
//! [`Session`] is the canonical form on disk / export / import.

mod entry;
mod error;
mod ids;
mod linear;
mod session;
mod tree;

pub use entry::{Entry, ToolCall, ToolResult};
pub use error::SessionError;
pub use ids::{NodeId, SessionId};
pub use linear::LinearSession;
pub use session::Session;
pub use tree::{Node, TreeSession};
