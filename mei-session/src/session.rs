use serde::{Deserialize, Serialize};

use crate::entry::Entry;
use crate::error::SessionError;
use crate::ids::SessionId;
use crate::linear::LinearSession;
use crate::tree::TreeSession;

/// Canonical form of a session on disk / export / import. A session is either
/// linear OR a tree; the `kind` field discriminates.
#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Session {
    Linear(LinearSession),
    Tree(TreeSession),
}

impl Session {
    /// Serializes to the canonical form: compact JSON (it is data, not a doc).
    pub fn to_json(&self) -> Result<String, SessionError> {
        Ok(serde_json::to_string(self)?)
    }

    /// Reads from the canonical form. Validates invariants; corrupt data becomes an error.
    pub fn from_json(s: &str) -> Result<Self, SessionError> {
        let session: Session = serde_json::from_str(s)?;
        session.validate()?;
        Ok(session)
    }

    /// The session's id, regardless of kind.
    pub fn id(&self) -> &SessionId {
        match self {
            Session::Linear(s) => s.id(),
            Session::Tree(s) => s.id(),
        }
    }

    /// What the model sees, regardless of kind: the context cut at the last compaction.
    pub fn model_context(&self) -> Vec<&Entry> {
        match self {
            Session::Linear(s) => s.model_context(),
            Session::Tree(s) => s.model_context(),
        }
    }

    fn validate(&self) -> Result<(), SessionError> {
        match self {
            Session::Linear(s) => s.validate(),
            Session::Tree(s) => s.validate(),
        }
    }
}
