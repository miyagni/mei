//! SSE transport: POST the built request, frame the `text/event-stream` body,
//! and run each event payload through the wire's decoder.

use std::collections::VecDeque;

use eventsource_stream::Eventsource;
use futures_util::{stream::unfold, StreamExt};

use super::EventStream;
use crate::auth::Auth;
use crate::error::{StreamError, TransportError};
use crate::event::ModelEvent;
use crate::request::ChatRequest;
use crate::wire::{Decoder, Wire};

/// Open an SSE stream for `request`, using `wire` to build and decode it.
/// `client` is shared and injected — one per process, not one per call.
pub async fn stream<W: Wire>(
    client: &reqwest::Client,
    wire: &W,
    auth: &Auth,
    request: ChatRequest<'_>,
) -> Result<EventStream, StreamError> {
    let wire_request = wire.build(auth, &request)?;

    let mut http = client
        .post(format!("{}{}", auth.base_url, wire_request.path))
        .body(wire_request.body);
    for (name, value) in wire_request.headers {
        http = http.header(name, value);
    }

    let response = http.send().await.map_err(TransportError::from)?;
    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.map_err(TransportError::from)?;
        return Err(TransportError::Status { status, body }.into());
    }

    let state = State {
        events: response.bytes_stream().eventsource(),
        decoder: wire.decoder(),
        pending: VecDeque::new(),
        done: false,
    };

    let stream = unfold(state, |mut st| async move {
        loop {
            if let Some(event) = st.pending.pop_front() {
                return Some((Ok(event), st));
            }
            if st.done {
                return None;
            }
            match st.events.next().await {
                Some(Ok(sse)) => match st.decoder.push(&sse.data) {
                    Ok(events) => st.pending.extend(events),
                    Err(err) => {
                        st.done = true;
                        return Some((Err(StreamError::from(err)), st));
                    }
                },
                Some(Err(err)) => {
                    st.done = true;
                    return Some((Err(StreamError::from(TransportError::from(err))), st));
                }
                None => {
                    st.done = true;
                    match st.decoder.end() {
                        Ok(events) => st.pending.extend(events),
                        Err(err) => return Some((Err(StreamError::from(err)), st)),
                    }
                }
            }
        }
    });

    Ok(Box::pin(stream))
}

struct State<S, D> {
    events: S,
    decoder: D,
    pending: VecDeque<ModelEvent>,
    done: bool,
}

#[cfg(test)]
mod real_stream {
    use super::*;
    use crate::catalog::Model;
    use crate::request::{Message, Role};
    use crate::wire::OpenAiCompat;

    /// Hits a real OpenAI-compatible provider. Opt-in: run with
    /// `cargo test -- --ignored` and `MEI_TEST_BASE_URL`/`_API_KEY`/`_MODEL` set.
    #[tokio::test]
    #[ignore = "hits a real provider; set MEI_TEST_BASE_URL / MEI_TEST_API_KEY / MEI_TEST_MODEL"]
    async fn streams_text_and_finishes() {
        let (Ok(base), Ok(key), Ok(model_id)) = (
            std::env::var("MEI_TEST_BASE_URL"),
            std::env::var("MEI_TEST_API_KEY"),
            std::env::var("MEI_TEST_MODEL"),
        ) else {
            eprintln!("skipping: set MEI_TEST_BASE_URL, MEI_TEST_API_KEY, MEI_TEST_MODEL");
            return;
        };

        let base_url: &'static str = Box::leak(base.into_boxed_str());
        let id: &'static str = Box::leak(model_id.into_boxed_str());
        let model = Model { provider: "test", id, name: "test", context: 0, max_output: 0 };
        let auth = Auth { key, base_url };
        let request = ChatRequest {
            model: &model,
            messages: vec![Message { role: Role::User, content: "Say hello in one word.".into() }],
            tools: Vec::new(),
        };

        let client = reqwest::Client::new();
        let mut events = stream(&client, &OpenAiCompat, &auth, request)
            .await
            .expect("stream opens");

        let mut text = String::new();
        let mut finished = false;
        while let Some(event) = events.next().await {
            match event.expect("event decodes") {
                ModelEvent::TextDelta(t) => text.push_str(&t),
                ModelEvent::ReasoningDelta(_) | ModelEvent::ToolCall { .. } => {}
                ModelEvent::Finish { reason, usage } => {
                    finished = true;
                    eprintln!("finish: {reason:?}, usage: {usage:?}");
                }
            }
        }
        assert!(!text.is_empty(), "expected some assistant text");
        assert!(finished, "expected a Finish event");
    }
}
