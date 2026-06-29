use thiserror::Error;

use crate::ids::NodeId;

/// Session API errors. Only appears at a real boundary: file io, corrupt
/// persisted data, and navigation to a node that does not exist.
#[derive(Debug, Error)]
pub enum SessionError {
    #[error("session io: {0}")]
    Io(#[from] std::io::Error),

    #[error("invalid session json: {0}")]
    Json(#[from] serde_json::Error),

    #[error("node {0:?} does not exist in this session")]
    UnknownNode(NodeId),

    #[error("corrupt session: {0}")]
    Corrupt(&'static str),
}
