//! The neutral request a turn sends to a provider.
//!
//! Provider-shaped, not session-shaped: the harness maps its session entries
//! onto these, and each provider's adapter maps these onto its own wire JSON.
//! Deliberately decoupled from `mei-session` so the providers API does not drag
//! in the transcript model.

use crate::catalog::Model;

/// A streamed chat turn: the model to call, the conversation so far, and the
/// tools the model may invoke.
///
/// `#[non_exhaustive]`: build with [`ChatRequest::new`] and set the public
/// fields you need, so optional params added later never break construction.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct ChatRequest<'a> {
    /// The model to run. Its `id` names the model on the wire; `max_output`
    /// bounds the response.
    pub model: &'a Model,
    /// The conversation so far, in order.
    pub messages: Vec<Message>,
    /// The tools the model may call (empty if none).
    pub tools: Vec<Tool>,
}

impl<'a> ChatRequest<'a> {
    /// A turn for `model` over `messages`, no tools. Set the other fields on the
    /// returned value as needed.
    pub fn new(model: &'a Model, messages: Vec<Message>) -> Self {
        ChatRequest {
            model,
            messages,
            tools: Vec::new(),
        }
    }
}

/// One message in the conversation.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct Message {
    pub role: Role,
    /// The message text. Tool calls and tool results get their own message
    /// shapes once the agent loop needs them.
    pub content: String,
}

impl Message {
    /// A system / developer-instructions message.
    pub fn system(content: impl Into<String>) -> Self {
        Message { role: Role::System, content: content.into() }
    }
    /// An end-user message.
    pub fn user(content: impl Into<String>) -> Self {
        Message { role: Role::User, content: content.into() }
    }
    /// A model message.
    pub fn assistant(content: impl Into<String>) -> Self {
        Message { role: Role::Assistant, content: content.into() }
    }
}

/// Who authored a message.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
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
