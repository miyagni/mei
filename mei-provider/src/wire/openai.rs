//! The OpenAI `/chat/completions` dialect, shared by every OpenAI-compatible
//! provider (OpenAI, DeepSeek, opencode-zen, Groq, OpenRouter, …) — clones
//! differ only in base URL + key, which live in `Auth`.

use serde::{Deserialize, Serialize};

use super::{Decoder, Wire, WireRequest};
use crate::auth::Auth;
use crate::error::WireError;
use crate::event::{FinishReason, ModelEvent, Usage};
use crate::request::{ChatRequest, Message, Role};

pub struct OpenAiCompat;

impl Wire for OpenAiCompat {
    type Decoder = OpenAiDecoder;

    fn build(&self, auth: &Auth, request: &ChatRequest<'_>) -> Result<WireRequest, WireError> {
        let body = Body {
            model: request.model.id,
            messages: request.messages.iter().map(WireMessage::from).collect(),
            stream: true,
            stream_options: StreamOptions { include_usage: true },
        };
        Ok(WireRequest {
            path: "/chat/completions",
            headers: vec![
                ("Authorization", format!("Bearer {}", auth.key)),
                ("Content-Type", "application/json".to_string()),
                ("Accept", "text/event-stream".to_string()),
            ],
            body: serde_json::to_string(&body)?,
        })
    }

    fn decoder(&self) -> OpenAiDecoder {
        OpenAiDecoder::default()
    }
}

// --- request ---

#[derive(Serialize)]
struct Body<'a> {
    model: &'a str,
    messages: Vec<WireMessage<'a>>,
    stream: bool,
    stream_options: StreamOptions,
}

#[derive(Serialize)]
struct StreamOptions {
    include_usage: bool,
}

#[derive(Serialize)]
struct WireMessage<'a> {
    role: &'static str,
    content: &'a str,
}

impl<'a> From<&'a Message> for WireMessage<'a> {
    fn from(m: &'a Message) -> Self {
        WireMessage {
            role: match m.role {
                Role::System => "system",
                Role::User => "user",
                Role::Assistant => "assistant",
            },
            content: &m.content,
        }
    }
}

// --- response ---

#[derive(Deserialize)]
struct Chunk {
    #[serde(default)]
    choices: Vec<Choice>,
    usage: Option<WireUsage>,
    /// An error the provider streamed after HTTP 200 (rate limit hit mid-turn,
    /// backend failure). Its presence turns the chunk into a hard error.
    error: Option<ApiError>,
}

#[derive(Deserialize)]
struct ApiError {
    message: String,
}

#[derive(Deserialize)]
struct Choice {
    #[serde(default)]
    delta: Delta,
    finish_reason: Option<String>,
}

#[derive(Deserialize, Default)]
struct Delta {
    content: Option<String>,
    reasoning_content: Option<String>,
}

#[derive(Deserialize)]
struct WireUsage {
    prompt_tokens: u32,
    completion_tokens: u32,
}

/// Decodes the OpenAI-compatible chunk stream. Stateful: the finish reason and
/// the usage arrive in separate chunks (usage in a trailing chunk with empty
/// `choices`, after the finish-reason chunk), and `Finish` is emitted once, at
/// stream end.
#[derive(Default)]
pub struct OpenAiDecoder {
    finish_reason: Option<FinishReason>,
    usage: Option<Usage>,
}

impl Decoder for OpenAiDecoder {
    fn push(&mut self, payload: &str) -> Result<Vec<ModelEvent>, WireError> {
        // The stream terminates with a literal `[DONE]` line — not JSON.
        if payload.trim() == "[DONE]" {
            return Ok(Vec::new());
        }
        let chunk: Chunk = serde_json::from_str(payload)?;

        // A provider error streamed after HTTP 200 must break loud, not vanish
        // into an empty chunk.
        if let Some(error) = chunk.error {
            return Err(WireError::Provider(error.message));
        }

        let mut events = Vec::new();
        if let Some(choice) = chunk.choices.into_iter().next() {
            if let Some(reasoning) = choice.delta.reasoning_content {
                events.push(ModelEvent::ReasoningDelta(reasoning));
            }
            if let Some(text) = choice.delta.content {
                // Skip the empty role-priming first chunk.
                if !text.is_empty() {
                    events.push(ModelEvent::TextDelta(text));
                }
            }
            if let Some(reason) = choice.finish_reason {
                self.finish_reason = Some(map_reason(reason));
            }
        }
        if let Some(usage) = chunk.usage {
            self.usage = Some(Usage {
                input_tokens: usage.prompt_tokens,
                output_tokens: usage.completion_tokens,
            });
        }
        Ok(events)
    }

    fn end(&mut self) -> Result<Vec<ModelEvent>, WireError> {
        match self.finish_reason.take() {
            Some(reason) => Ok(vec![ModelEvent::Finish {
                reason,
                usage: self.usage.take(),
            }]),
            None => Ok(Vec::new()),
        }
    }
}

/// Map a `finish_reason`, preserving unknown values instead of faking/dropping.
fn map_reason(reason: String) -> FinishReason {
    match reason.as_str() {
        "stop" => FinishReason::Stop,
        "length" => FinishReason::Length,
        "tool_calls" | "function_call" => FinishReason::ToolUse,
        _ => FinishReason::Other(reason),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_text_then_finishes_with_usage() {
        let mut d = OpenAiDecoder::default();
        assert!(d
            .push(r#"{"choices":[{"delta":{"role":"assistant","content":""},"finish_reason":null}]}"#)
            .unwrap()
            .is_empty());
        assert_eq!(
            d.push(r#"{"choices":[{"delta":{"content":"Hi"},"finish_reason":null}]}"#).unwrap(),
            vec![ModelEvent::TextDelta("Hi".into())]
        );
        assert!(d.push(r#"{"choices":[{"delta":{},"finish_reason":"stop"}]}"#).unwrap().is_empty());
        assert!(d
            .push(r#"{"choices":[],"usage":{"prompt_tokens":3,"completion_tokens":1}}"#)
            .unwrap()
            .is_empty());
        assert!(d.push("[DONE]").unwrap().is_empty());
        assert_eq!(
            d.end().unwrap(),
            vec![ModelEvent::Finish {
                reason: FinishReason::Stop,
                usage: Some(Usage { input_tokens: 3, output_tokens: 1 }),
            }]
        );
    }

    #[test]
    fn reasoning_maps_and_unknown_reason_passes_through() {
        let mut d = OpenAiDecoder::default();
        assert_eq!(
            d.push(r#"{"choices":[{"delta":{"reasoning_content":"think"},"finish_reason":null}]}"#)
                .unwrap(),
            vec![ModelEvent::ReasoningDelta("think".into())]
        );
        d.push(r#"{"choices":[{"delta":{},"finish_reason":"insufficient_system_resource"}]}"#)
            .unwrap();
        assert_eq!(
            d.end().unwrap(),
            vec![ModelEvent::Finish {
                reason: FinishReason::Other("insufficient_system_resource".into()),
                usage: None,
            }]
        );
    }

    #[test]
    fn provider_error_chunk_breaks_loud() {
        let mut d = OpenAiDecoder::default();
        let result =
            d.push(r#"{"error":{"message":"rate limit exceeded","type":"rate_limit_error"}}"#);
        match result {
            Err(WireError::Provider(msg)) => assert_eq!(msg, "rate limit exceeded"),
            other => panic!("expected WireError::Provider, got {other:?}"),
        }
    }

    #[test]
    fn no_finish_reason_yields_no_finish() {
        let mut d = OpenAiDecoder::default();
        d.push(r#"{"choices":[{"delta":{"content":"partial"},"finish_reason":null}]}"#).unwrap();
        assert!(d.end().unwrap().is_empty()); // stream cut before any finish_reason
    }
}
