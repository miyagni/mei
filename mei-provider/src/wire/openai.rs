//! The OpenAI `/chat/completions` dialect, shared by every OpenAI-compatible
//! provider (OpenAI, DeepSeek, opencode-zen, Groq, OpenRouter, …) — clones
//! differ only in base URL + key, which live in `Auth`.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use super::{Decoder, Wire, WireRequest};
use crate::auth::Auth;
use crate::error::{ProviderError, WireError};
use crate::event::{FinishReason, ModelEvent, Usage};
use crate::request::{ChatRequest, Message, Role, Tool, ToolChoice};

pub struct OpenAiCompat;

impl Wire for OpenAiCompat {
    type Decoder = OpenAiDecoder;

    fn build(&self, auth: &Auth, request: &ChatRequest<'_>) -> Result<WireRequest, WireError> {
        // Tool choice is meaningless without tools, and `auto` is the implicit
        // default — so send it only when there are tools and the choice isn't Auto.
        let tool_choice = if request.tools.is_empty() || request.tool_choice == ToolChoice::Auto {
            None
        } else {
            Some(WireToolChoice::from(&request.tool_choice))
        };
        let body = Body {
            model: request.model.id,
            messages: request.messages.iter().map(WireMessage::from).collect(),
            stream: true,
            stream_options: StreamOptions { include_usage: true },
            tools: request.tools.iter().map(WireTool::from).collect(),
            tool_choice,
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

    fn parse_error(&self, body: &str) -> ProviderError {
        match serde_json::from_str::<ErrorEnvelope>(body) {
            Ok(env) => env.error.into_provider(env.request_id),
            // Not the expected envelope — surface the raw body as the message
            // rather than hide it.
            Err(_) => ProviderError {
                message: body.to_string(),
                kind: None,
                code: None,
                request_id: None,
            },
        }
    }
}

// --- request ---

#[derive(Serialize)]
struct Body<'a> {
    model: &'a str,
    messages: Vec<WireMessage<'a>>,
    stream: bool,
    stream_options: StreamOptions,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    tools: Vec<WireTool<'a>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_choice: Option<WireToolChoice<'a>>,
}

#[derive(Serialize)]
struct StreamOptions {
    include_usage: bool,
}

#[derive(Serialize)]
struct WireTool<'a> {
    #[serde(rename = "type")]
    kind: &'static str,
    function: WireFunction<'a>,
}

#[derive(Serialize)]
struct WireFunction<'a> {
    name: &'a str,
    description: &'a str,
    parameters: &'a schemars::Schema,
}

impl<'a> From<&'a Tool> for WireTool<'a> {
    fn from(t: &'a Tool) -> Self {
        WireTool {
            kind: "function",
            function: WireFunction {
                name: &t.name,
                description: &t.description,
                parameters: &t.parameters,
            },
        }
    }
}

/// OpenAI's `tool_choice`: either a mode string (`"auto"`/`"none"`/`"required"`)
/// or `{ "type": "function", "function": { "name": … } }`.
#[derive(Serialize)]
#[serde(untagged)]
enum WireToolChoice<'a> {
    Mode(&'static str),
    Named {
        #[serde(rename = "type")]
        kind: &'static str,
        function: NamedFn<'a>,
    },
}

#[derive(Serialize)]
struct NamedFn<'a> {
    name: &'a str,
}

impl<'a> From<&'a ToolChoice> for WireToolChoice<'a> {
    fn from(tc: &'a ToolChoice) -> Self {
        match tc {
            ToolChoice::Auto => WireToolChoice::Mode("auto"),
            ToolChoice::None => WireToolChoice::Mode("none"),
            ToolChoice::Required => WireToolChoice::Mode("required"),
            ToolChoice::Function(name) => WireToolChoice::Named {
                kind: "function",
                function: NamedFn { name },
            },
        }
    }
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

/// The provider's error object, `{ message, type, code }`. Shared by the
/// non-200 HTTP body (wrapped in `ErrorEnvelope`) and the mid-stream error chunk.
#[derive(Deserialize)]
struct ApiError {
    message: String,
    #[serde(rename = "type")]
    kind: Option<String>,
    code: Option<String>,
}

impl ApiError {
    fn into_provider(self, request_id: Option<String>) -> ProviderError {
        ProviderError {
            message: self.message,
            kind: self.kind,
            code: self.code,
            request_id,
        }
    }
}

/// A non-success HTTP error body: `{ "error": { … }, "request_id"? }`. The
/// `request_id` sits beside `error` (Anthropic sends it; OpenAI doesn't).
#[derive(Deserialize)]
struct ErrorEnvelope {
    error: ApiError,
    request_id: Option<String>,
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
    #[serde(default)]
    tool_calls: Vec<ToolCallFragment>,
}

/// One streamed piece of a tool call, keyed by `index`. The first fragment for
/// an index carries `id` + `function.name`; later ones append `arguments`.
#[derive(Deserialize)]
struct ToolCallFragment {
    index: u32,
    id: Option<String>,
    function: Option<FunctionFragment>,
}

#[derive(Deserialize)]
struct FunctionFragment {
    name: Option<String>,
    #[serde(default)]
    arguments: String,
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
    /// Tool calls being assembled, keyed by `index` (order preserved). Emitted
    /// whole at stream end.
    tool_calls: BTreeMap<u32, PartialToolCall>,
}

#[derive(Default)]
struct PartialToolCall {
    id: String,
    name: String,
    arguments: String,
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
            return Err(WireError::Provider(error.into_provider(None)));
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
            for frag in choice.delta.tool_calls {
                let call = self.tool_calls.entry(frag.index).or_default();
                if let Some(id) = frag.id {
                    call.id = id;
                }
                if let Some(function) = frag.function {
                    if let Some(name) = function.name {
                        call.name = name;
                    }
                    call.arguments.push_str(&function.arguments);
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
        // No finish reason means the stream was cut/cancelled: emit nothing (not
        // even partial tool calls) — the absence of Finish is the incomplete signal.
        let Some(reason) = self.finish_reason.take() else {
            return Ok(Vec::new());
        };

        let mut events = Vec::new();
        for (_index, call) in std::mem::take(&mut self.tool_calls) {
            if call.id.is_empty() || call.name.is_empty() {
                return Err(WireError::IncompleteToolCall(format!(
                    "id={:?}, name={:?}",
                    call.id, call.name
                )));
            }
            events.push(ModelEvent::ToolCall {
                id: call.id,
                name: call.name,
                arguments: call.arguments,
            });
        }
        events.push(ModelEvent::Finish {
            reason,
            usage: self.usage.take(),
        });
        Ok(events)
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
            Err(WireError::Provider(e)) => {
                assert_eq!(e.message, "rate limit exceeded");
                assert_eq!(e.kind.as_deref(), Some("rate_limit_error"));
            }
            other => panic!("expected WireError::Provider, got {other:?}"),
        }
    }

    #[test]
    fn parse_error_extracts_the_envelope() {
        let e = OpenAiCompat.parse_error(
            r#"{"error":{"message":"Invalid API key","type":"invalid_request_error","code":"invalid_api_key"}}"#,
        );
        assert_eq!(e.message, "Invalid API key");
        assert_eq!(e.kind.as_deref(), Some("invalid_request_error"));
        assert_eq!(e.code.as_deref(), Some("invalid_api_key"));
    }

    #[test]
    fn parse_error_falls_back_to_raw_body() {
        // Not the expected envelope — the raw text is surfaced, not hidden.
        let e = OpenAiCompat.parse_error("upstream connect error or disconnect/reset");
        assert_eq!(e.message, "upstream connect error or disconnect/reset");
        assert!(e.kind.is_none());
    }

    #[test]
    fn assembles_a_tool_call_across_fragments() {
        let mut d = OpenAiDecoder::default();
        d.push(r#"{"choices":[{"delta":{"tool_calls":[{"index":0,"id":"call_1","function":{"name":"get_weather","arguments":"{\"loc"}}]},"finish_reason":null}]}"#).unwrap();
        d.push(r#"{"choices":[{"delta":{"tool_calls":[{"index":0,"function":{"arguments":"ation\":\"SP\"}"}}]},"finish_reason":null}]}"#).unwrap();
        d.push(r#"{"choices":[{"delta":{},"finish_reason":"tool_calls"}]}"#).unwrap();
        assert_eq!(
            d.end().unwrap(),
            vec![
                ModelEvent::ToolCall {
                    id: "call_1".into(),
                    name: "get_weather".into(),
                    arguments: r#"{"location":"SP"}"#.into(),
                },
                ModelEvent::Finish { reason: FinishReason::ToolUse, usage: None },
            ]
        );
    }

    #[test]
    fn assembles_parallel_tool_calls_in_index_order() {
        let mut d = OpenAiDecoder::default();
        d.push(r#"{"choices":[{"delta":{"tool_calls":[{"index":0,"id":"a","function":{"name":"f0","arguments":"{}"}},{"index":1,"id":"b","function":{"name":"f1","arguments":"{}"}}]},"finish_reason":null}]}"#).unwrap();
        d.push(r#"{"choices":[{"delta":{},"finish_reason":"tool_calls"}]}"#).unwrap();
        let events = d.end().unwrap();
        assert_eq!(
            events[0],
            ModelEvent::ToolCall { id: "a".into(), name: "f0".into(), arguments: "{}".into() }
        );
        assert_eq!(
            events[1],
            ModelEvent::ToolCall { id: "b".into(), name: "f1".into(), arguments: "{}".into() }
        );
        assert!(matches!(events[2], ModelEvent::Finish { reason: FinishReason::ToolUse, .. }));
    }

    #[test]
    fn incomplete_tool_call_breaks_loud() {
        let mut d = OpenAiDecoder::default();
        // id arrives, name never does — provider stream is inconsistent.
        d.push(r#"{"choices":[{"delta":{"tool_calls":[{"index":0,"id":"call_x","function":{"arguments":"{}"}}]},"finish_reason":null}]}"#).unwrap();
        d.push(r#"{"choices":[{"delta":{},"finish_reason":"tool_calls"}]}"#).unwrap();
        assert!(matches!(d.end(), Err(WireError::IncompleteToolCall(_))));
    }

    #[test]
    fn build_serializes_typed_tools_and_forced_choice() {
        use crate::auth::Auth;
        use crate::catalog::Model;
        use crate::request::{ChatRequest, Message, Tool, ToolChoice};

        #[derive(schemars::JsonSchema)]
        #[allow(dead_code)]
        struct Args {
            city: String,
        }

        let model = Model { provider: "test", id: "m", name: "m", context: 0, max_output: 0 };
        let auth = Auth { key: "k".into(), base_url: "https://x" };
        let mut req = ChatRequest::new(&model, vec![Message::user("hi")]);
        req.tools = vec![Tool::new("get_weather", "Get weather", schemars::schema_for!(Args))];
        req.tool_choice = ToolChoice::Function("get_weather".into());

        let body = OpenAiCompat.build(&auth, &req).unwrap().body;
        assert!(body.contains(r#""name":"get_weather""#));
        assert!(body.contains(r#""tool_choice""#));
        // the schema is embedded as a JSON object, not a re-encoded string
        assert!(body.contains(r#""properties""#));
    }

    #[test]
    fn no_finish_reason_yields_no_finish() {
        let mut d = OpenAiDecoder::default();
        d.push(r#"{"choices":[{"delta":{"content":"partial"},"finish_reason":null}]}"#).unwrap();
        assert!(d.end().unwrap().is_empty()); // stream cut before any finish_reason
    }
}
