//! The model stream contract.
//!
//! A provider turns its wire protocol (SSE today, WebSocket later) into a
//! stream of these events. The agent loop consumes them without knowing which
//! provider or transport produced them — both decode to the same `ModelEvent`.

/// One event in a model's streamed response.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum ModelEvent {
    /// A chunk of the model's reasoning ("thinking"), shown separately from the
    /// answer. Lets the user watch the model's direction and cancel the turn
    /// (drop the stream) before it commits to a wrong path.
    ReasoningDelta(String),
    /// A chunk of assistant text — the actual answer.
    TextDelta(String),
    /// The model is requesting a tool call, fully assembled.
    ToolCall {
        /// Provider-assigned id, echoed back with the tool result.
        id: String,
        /// Tool name as registered with the model.
        name: String,
        /// Call arguments as a JSON object string (parsed by the caller).
        arguments: String,
    },
    /// The turn is done; no more events follow.
    Finish {
        reason: FinishReason,
        /// Token counts, when the provider reported them. Absent when usage was
        /// never sent (`include_usage` off, a proxy dropped it) or the stream
        /// was cut before it arrived — never faked as zero.
        usage: Option<Usage>,
    },
}

/// Why the model stopped producing the turn.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum FinishReason {
    /// Stopped on its own — the message is complete.
    Stop,
    /// Stopped to hand back tool calls for the harness to run.
    ToolUse,
    /// Hit the maximum output token limit.
    Length,
    /// A reason outside the set above, preserved verbatim (DeepSeek
    /// `insufficient_system_resource`, Gemini `OTHER`, …).
    Other(String),
}

/// Token counts for a turn.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Usage {
    pub input_tokens: u32,
    pub output_tokens: u32,
}
