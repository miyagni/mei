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

    // Canonical, provider-neutral form of tools; translating to/from each
    // provider's wire lives in mei-provider. Not wired into a tool loop yet —
    // that comes with mei-agent/the harness.
    /// Tool call emitted by the model.
    ToolCall(ToolCall),
    /// Tool result, matched to the call by `call_id`.
    ToolResult(ToolResult),
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

/// A tool call in canonical, provider-neutral form. mei-provider translates it
/// to/from each provider's wire (OpenAI sends `arguments` as a JSON string,
/// Anthropic sends `input` as an object — `Value` is the middle ground).
#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct ToolCall {
    /// Call id; the `ToolResult` matches on this id. Opaque token: we keep the
    /// provider's original id and reuse it.
    pub id: String,
    /// Tool name.
    pub name: String,
    /// Arguments as structured JSON, provider-neutral.
    pub arguments: serde_json::Value,
}

/// A tool result in canonical form. Matched to the call by `call_id`.
#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct ToolResult {
    /// References `ToolCall.id`.
    pub call_id: String,
    // TODO: text only for now. Structured/multimodal result (Anthropic allows
    // blocks) comes when the tool loop exists.
    pub output: String,
}
