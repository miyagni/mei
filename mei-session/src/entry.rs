use serde::{Deserialize, Serialize};

/// A position in the transcript. Each variant is a kind of session message.
#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "role", content = "content", rename_all = "snake_case")]
pub enum Entry {
    /// User message.
    User(String),
    /// Model message.
    Assistant(String),
    /// Summary of what came before. Visible-context boundary: the model only sees
    /// from this point onward; the user still sees the whole session.
    Compaction(String),
}

impl Entry {
    /// A user message.
    pub fn user(text: impl Into<String>) -> Self {
        Entry::User(text.into())
    }

    /// A model message.
    pub fn assistant(text: impl Into<String>) -> Self {
        Entry::Assistant(text.into())
    }

    /// A compaction boundary carrying the summary of what came before.
    pub fn compaction(summary: impl Into<String>) -> Self {
        Entry::Compaction(summary.into())
    }

    /// `true` if this entry is a compaction boundary.
    pub fn is_compaction(&self) -> bool {
        matches!(self, Entry::Compaction(_))
    }
}
