use std::io::Write;
use std::path::Path;

use serde::{Deserialize, Serialize};
use tempfile::NamedTempFile;

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

    /// Writes the session to `path` as canonical JSON.
    ///
    /// Atomic: writes a sibling temp file and renames it over the target, so a
    /// crash mid-write cannot truncate an existing session — `load` always sees
    /// either the old session or the fully written new one.
    pub fn save(&self, path: impl AsRef<Path>) -> Result<(), SessionError> {
        let path = path.as_ref();
        let dir = match path.parent() {
            Some(parent) if !parent.as_os_str().is_empty() => parent,
            _ => Path::new("."),
        };
        let mut tmp = NamedTempFile::new_in(dir)?;
        tmp.write_all(self.to_json()?.as_bytes())?;
        tmp.persist(path).map_err(|e| e.error)?;
        Ok(())
    }

    /// Reads a session from `path`, validating invariants.
    pub fn load(path: impl AsRef<Path>) -> Result<Self, SessionError> {
        let json = std::fs::read_to_string(path)?;
        Session::from_json(&json)
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
