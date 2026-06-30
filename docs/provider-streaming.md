# Provider streaming reference — decoding wire formats into `ModelEvent`

How each provider streams a turn, and how its wire format maps onto the common
event the adapter emits:

```rust
enum ModelEvent {
    ReasoningDelta(String),
    TextDelta(String),
    ToolCall { id: String, name: String, arguments: String },
    Finish { reason: FinishReason, usage: Option<Usage> },
}
```

Researched 2026-06-30 from primary docs + the Vercel AI SDK source (ground
truth for chunk shapes). Five wire formats, **three dialects**:

- **OpenAI-compatible** (`/chat/completions`, flat `choices[].delta` chunks) —
  one adapter covers OpenAI, DeepSeek, opencode-zen, Groq, OpenRouter, Together,
  Fireworks, …; clones change only the base URL + key.
- **Anthropic Messages** — typed content-block lifecycle (start/delta/stop).
- **Google Gemini** — `candidates[].parts` SSE.

---

## 1. OpenAI-compatible (`/chat/completions`)

Covers OpenAI **and** every clone (DeepSeek, opencode-zen, Groq, OpenRouter,
Together, Fireworks…). Identical path + chunk shape; only base URL + key change.

- **Endpoint:** `POST {base_url}/chat/completions`, `stream: true` → SSE
  (`text/event-stream`), one event per `data: {json}\n\n`, terminated by a
  literal `data: [DONE]` line (**not JSON** — string-match before parsing).
- **Auth:** `Authorization: Bearer <key>` + `Content-Type: application/json`.
  (Azure clone uses `api-key: <key>`.)
- **Base URLs:** OpenAI `https://api.openai.com/v1`, DeepSeek
  `https://api.deepseek.com` (no `/v1`; the docs warn `/v1` is *not* a version),
  opencode-zen `https://opencode.ai/zen/v1`, opencode-go
  `https://opencode.ai/zen/go/v1` (undocumented surface — verify live),
  Groq `https://api.groq.com/openai/v1`, OpenRouter `https://openrouter.ai/api/v1`.
- **Request:** `{ model, messages, stream: true, stream_options: { include_usage: true }, tools?, tool_choice? }`.
  Usage in a stream is **opt-in** — without `stream_options.include_usage` no
  usage is ever sent.
- **Chunk:** `chat.completion.chunk`:
  ```
  { choices: [ { index,
      delta: { role?, content?: string|null, reasoning_content?: string|null, tool_calls?: [...] },
      finish_reason: string|null } ],
    usage?: null | { prompt_tokens, completion_tokens, total_tokens, ... } }
  ```
  - `delta.content` — text fragment. `""` on the role-priming first chunk;
    null/absent on tool-only and finish chunks.
  - `delta.reasoning_content` — **non-standard extension** (DeepSeek reasoner,
    some opencode models). Absent on vanilla OpenAI. Sibling of `content`.
  - `finish_reason` — null on every intermediate chunk; set only on the last
    content chunk (whose `delta` is `{}`).
  - The trailing **usage chunk has `choices: []` (empty)** and a populated
    `usage`. It arrives in a **separate chunk after** the finish-reason chunk,
    right before `data: [DONE]`. **finish_reason and usage are never in the same
    chunk.**
- **finish_reason values:** `stop`, `length`, `tool_calls`, `content_filter`,
  `function_call` (deprecated alias of `tool_calls`); DeepSeek adds
  `insufficient_system_resource`. **Unknown values must pass through, not fail.**
- **Tool calls:** keyed by `delta.tool_calls[].index` (**not by id**). The first
  delta for an index carries `{index, id, type:"function", function:{name, arguments:""}}`;
  later deltas carry only `{index, function:{arguments:"<fragment>"}}`.
  Concatenate `function.arguments` fragments in arrival order; complete at
  `finish_reason == "tool_calls"`. Parallel calls = multiple indices.
- **Reasoning:** vanilla OpenAI chat-completions does **not** stream reasoning
  text (it's internal, counted only as `usage.completion_tokens_details.reasoning_tokens`).
  Reasoning text exists only via the extension field `reasoning_content`
  (DeepSeek reasoner streams the full CoT first, then switches to `content`).
  **`reasoning_content` must NOT be echoed back in later request messages →
  HTTP 400.**

### Decode mapping

| Source | `ModelEvent` |
|---|---|
| `delta.content` (non-null) | `TextDelta` (skip the `""` priming chunk) |
| `delta.reasoning_content` (extension) | `ReasoningDelta` |
| `delta.tool_calls[index]` | buffer per `index`; emit one `ToolCall{id,name,arguments}` when complete |
| `finish_reason` + the `usage` (any chunk where `usage != null`) | `Finish{reason, usage}`, emitted at stream end |
| chunk with top-level `error` field | terminal `Err` (not a normal chunk) |

Map reasons: `stop→Stop, length→Length, tool_calls|function_call→ToolUse,
unknown→Other(s)`. Read usage from **any** chunk where `usage != null` (some
clones attach it to the finish chunk). `usage` may be **absent** entirely
(include_usage off, or stream cut) → `Finish.usage` is `Option`, never 0.

**Per-provider notes (this dialect):**
- **DeepSeek:** model ids `deepseek-chat`/`deepseek-reasoner` (legacy, deprecate
  2026-07-24, route to `deepseek-v4-flash` modes) or `deepseek-v4-flash`/
  `deepseek-v4-pro`. Don't hardcode `deepseek-chat == V3`. Reasoner rejects
  sampling params (temperature, top_p, …).
- **opencode-zen / go:** model id is the **bare** id (`gpt-5.5`, not
  `opencode/gpt-5.5`). `include_usage` honored end-to-end is **unverified** (it's
  a proxy); authoritative usage is Zen's own billing. Reasoning passthrough is
  undocumented — read `reasoning_content` defensively.

---

## 2. Anthropic Messages (future adapter)

Bespoke typed-block dialect — fundamentally different from OpenAI.

- **Endpoint:** `POST https://api.anthropic.com/v1/messages`, `stream: true` in
  the body.
- **Auth:** `x-api-key: <key>` (NOT Bearer) + **required**
  `anthropic-version: 2023-06-01` + `content-type: application/json`.
- **Request:** `{ model, max_tokens (REQUIRED), messages, stream: true, system?, tools?, thinking? }`.
  **Usage is always streamed — no `include_usage` flag.**
- **Events:** named SSE (`event: <name>` + `data: <json>`), three nesting
  levels: `message_start` → per block (`content_block_start` → N×`content_block_delta`
  → `content_block_stop`) → `message_delta`(s) → `message_stop`. Plus `ping`,
  `error`. Each block addressed by `index`. **No `[DONE]` — end is `message_stop`.**
- **Deltas** (in `content_block_delta.delta`, by `delta.type`): `text_delta.text`
  → `TextDelta`; `thinking_delta.thinking` → `ReasoningDelta`; `input_json_delta.partial_json`
  → tool args (concatenate per index); a single `signature_delta` before block
  stop (the thinking-block integrity token).
- **finish:** `message_delta.delta.stop_reason ∈ {end_turn, max_tokens,
  stop_sequence, tool_use, pause_turn, refusal, model_context_window_exceeded,
  compaction}`. Map: `end_turn/stop_sequence/pause_turn→Stop, tool_use→ToolUse,
  max_tokens/model_context_window_exceeded→Length, refusal→content-filter,
  compaction→Other`.
- **usage:** split — `message_start.usage` (input) ⊕ last `message_delta.usage`
  (output, **cumulative**). Merge both.

### Strain on the abstraction (handle when building this adapter)
1. **`signature_delta`** carries the thinking-block signature (not text). No
   `ModelEvent` slot; dropping it breaks multi-turn replay of thinking → needs a
   provider-metadata side channel.
2. **Block lifecycle + `index`** → the adapter MUST keep per-index buffering
   state (emit `ToolCall` at `content_block_stop`). Stateful, not a 1:1 chunk
   translator.
3. **In-stream `error` events** arrive after HTTP 200 → the stream's `Err` arm
   handles this (we surface errors as `Result::Err`, not a `ModelEvent`).
4. `pause_turn` (resumable) and `refusal` (policy) both collapse into a finish
   reason — nuance lost.

---

## 3. Google Gemini (future adapter)

- **Endpoint:** `POST https://generativelanguage.googleapis.com/v1beta/models/{model}:streamGenerateContent?alt=sse`.
  `?alt=sse` is **required** for SSE; without it returns one JSON array. Model
  path-encoded `models/{id}`.
- **Auth:** `x-goog-api-key: <key>` (or `?key=`). No Bearer/version header.
- **Request:** `{ contents, systemInstruction?, tools?, generationConfig? }`.
  No streaming/usage flags — SSE via the query param; `usageMetadata` always
  returned.
- **Chunk:** `{ candidates?: [{ content?: { parts?: Part[] }, finishReason? }], usageMetadata? }`.
  Only `candidates[0]` is read; the array can be **empty** (skip such chunks).
  `Part` is a union; each may carry `thoughtSignature?`.
  - text part, `thought !== true` → `TextDelta(text)` (already incremental).
  - text part, `thought === true` → `ReasoningDelta(text)`.
  - `functionCall` part → `ToolCall`.
- **finish:** `candidates[0].finishReason` (only on terminal chunk): `STOP`
  (→ `tool-calls` if any functionCall was emitted, else `stop`), `MAX_TOKENS→length`,
  `SAFETY/RECITATION/…→content-filter`, `MALFORMED_FUNCTION_CALL→error`,
  else `other`. **No `[DONE]`/`message_stop`** — end = the chunk with
  `finishReason` + SSE close.
- **usage:** `usageMetadata` throughout (cumulative), always on the final chunk.
  Output total = `candidatesTokenCount + thoughtsTokenCount` (candidates
  **excludes** thinking).

### Strain on the abstraction (handle when building this adapter)
1. **`functionCall` has NO id** → synthesize a `ToolCall.id` (match the response
   back by name/order).
2. **`functionCall.args` is a JSON object, not a string** → `to_string` it for
   `ToolCall.arguments`.
3. Tool call delivered **whole in one chunk** (classic) — single `ToolCall`, no
   accumulation. (Gemini-3 `partialArgs`/`willContinue` mode needs accumulation;
   different shape.)
4. `STOP` is ambiguous (returned even with a function call) — infer
   stop-vs-tool-calls from whether any tool call was seen.
5. Reasoning is only a `thought: true` boolean on a text part; `thoughtSignature`
   is opaque metadata, not reasoning text.

---

## Consequences for `ModelEvent` (the contract)

The real wire forces two honest changes, both = "don't fake / break loud":

1. **`Finish.usage: Option<Usage>`** — usage can genuinely be absent
   (`include_usage` off, stream cancelled, proxy didn't forward it). Never
   default to 0.
2. **`FinishReason::Other(String)`** — providers send reasons outside our set
   (DeepSeek `insufficient_system_resource`, Gemini `OTHER`, etc.). Preserve the
   value; don't error, don't fake `Stop`.

Holds for the OpenAI-compatible happy path now. Anthropic and Gemini will, when
built, additionally need: a place to stash the thinking signature
(provider-metadata), per-block/per-index buffering state, a synthesized
tool-call id + stringified args for Gemini, and they already fit our
"errors are the stream's `Err` arm" design (no `ModelEvent::Error` needed).
