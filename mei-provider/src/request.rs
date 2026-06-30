//! The neutral request a turn sends to a provider.
//!
//! Provider-shaped, not session-shaped: the harness maps its session entries
//! onto these, and each provider's adapter maps these onto its own wire JSON.
//! Deliberately decoupled from `mei-session` so the providers API does not drag
//! in the transcript model.

use crate::catalog::Model;

/// A streamed chat turn: the model to call, the conversation so far, and the
/// tools the model may invoke.
#[derive(Debug, Clone)]
pub struct ChatRequest<'a> {
    /// The model to run. Its `id` names the model on the wire; `max_output`
    /// bounds the response.
    pub model: &'a Model,
    /// The conversation so far, in order.
    pub messages: Vec<Message>,
    /// The tools the model may call (empty if none).
    pub tools: Vec<Tool>,
}

/// One message in the conversation.
#[derive(Debug, Clone)]
pub struct Message {
    pub role: Role,
    /// The message text. Tool calls and tool results get their own message
    /// shapes once the agent loop needs them.
    pub content: String,
}

/// Who authored a message.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    /// System / developer instructions.
    System,
    /// The end user.
    User,
    /// The model.
    Assistant,
}

/// A tool the model may call.
#[derive(Debug, Clone)]
pub struct Tool {
    pub name: String,
    pub description: String,
    /// JSON Schema for the parameters, as a JSON object string.
    pub parameters: String,
}
