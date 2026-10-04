//! Streamed replies become the same turns as whole ones; Stop ends a
//! request at once; a cut-off stream is retried; a server that cannot
//! stream is called without streaming.
mod common;

use std::sync::Mutex;
use std::time::{Duration, Instant};

use common::mock::{serve_seq, Reply};
use crosure_agent::{
    AgentError, AgentEvent, AnthropicProvider, Block, Entry, Live, OpenAiProvider, Provider, Sink,
    Stop, Transcript,
};
use serde_json::json;

fn transcript() -> Transcript {
    Transcript {
        entries: vec![Entry::User("Reverse it.".into())],
        ..Default::default()
    }
}

/// Records live updates; can press Stop after the first one.
#[derive(Default)]
struct Watch {
    lives: Mutex<Vec<Live>>,
    stop_on_live: bool,
}

impl Sink for Watch {
    fn emit(&self, _: AgentEvent) {}
    fn should_stop(&self) -> bool {
        self.stop_on_live && self.lives.lock().is_ok_and(|l| !l.is_empty())
    }
    fn live(&self, live: &Live) {
        if let Ok(mut l) = self.lives.lock() {
            l.push(live.clone());
        }
    }
}

const ANTHROPIC_EVENTS: &[&str] = &[
    r#"{"type":"message_start","message":{"id":"m1","type":"message","role":"assistant","content":[],"usage":{"input_tokens":120,"cache_read_input_tokens":1000,"output_tokens":1}}}"#,
    r#"{"type":"content_block_start","index":0,"content_block":{"type":"thinking","thinking":"","signature":""}}"#,
    r#"{"type":"content_block_delta","index":0,"delta":{"type":"thinking_delta","thinking":"Check the "}}"#,
    r#"{"type":"content_block_delta","index":0,"delta":{"type":"thinking_delta","thinking":"checker."}}"#,
    r#"{"type":"content_block_delta","index":0,"delta":{"type":"signature_delta","signature":"sig=="}}"#,
    r#"{"type":"content_block_stop","index":0}"#,
    r#"{"type":"ping"}"#,
    r#"{"type":"content_block_start","index":1,"content_block":{"type":"text","text":""}}"#,
    r#"{"type":"content_block_delta","index":1,"delta":{"type":"text_delta","text":"Reading it."}}"#,
    r#"{"type":"content_block_stop","index":1}"#,
    r#"{"type":"content_block_start","index":2,"content_block":{"type":"tool_use","id":"t1","name":"disassemble","input":{}}}"#,
    r#"{"type":"content_block_delta","index":2,"delta":{"type":"input_json_delta","partial_json":"{\"target\":\"check_"}}"#,
    r#"{"type":"content_block_delta","index":2,"delta":{"type":"input_json_delta","partial_json":"password\",\"why\":\"w\"}"}}"#,
    r#"{"type":"content_block_stop","index":2}"#,
    r#"{"type":"message_delta","delta":{"stop_reason":"tool_use","stop_sequence":null},"usage":{"output_tokens":42}}"#,
    r#"{"type":"message_stop"}"#,
];

#[test]
fn an_anthropic_stream_is_the_same_turn_as_a_whole_reply() -> Result<(), Box<dyn std::error::Error>>
{
    let whole = json!({
        "id": "m1", "type": "message", "role": "assistant", "stop_reason": "tool_use",
        "content": [
            { "type": "thinking", "thinking": "Check the checker.", "signature": "sig==" },
            { "type": "text", "text": "Reading it." },
            { "type": "tool_use", "id": "t1", "name": "disassemble", "input": { "target": "check_password", "why": "w" } }
        ],
        "usage": { "input_tokens": 120, "cache_read_input_tokens": 1000, "output_tokens": 42 }
    });
    let (base, rx) = serve_seq(vec![
        Reply::sse(ANTHROPIC_EVENTS),
        Reply::json("200 OK", &whole),
    ]);
    let p = AnthropicProvider::new("anthropic", "claude-x", &base, "k")?;
    let sink = Watch::default();
    let streamed = p.next_live(&transcript(), &sink)?;
    assert_eq!(rx.recv()?["stream"], true);
    let plain = p.next(&transcript())?;
    assert_eq!(rx.recv()?.get("stream"), None);

    assert_eq!(
        streamed.raw, plain.raw,
        "echoed back verbatim, signature included"
    );
    assert_eq!(streamed.stop, Stop::ToolUse);
    assert_eq!(
        (
            streamed.input_tokens,
            streamed.output_tokens,
            streamed.cache_read_tokens
        ),
        (
            plain.input_tokens,
            plain.output_tokens,
            plain.cache_read_tokens
        )
    );
    assert_eq!(streamed.blocks.len(), plain.blocks.len());
    let last = sink
        .lives
        .lock()
        .map_err(|_| "lock")?
        .last()
        .cloned()
        .ok_or("no live")?;
    assert_eq!(last.thinking, "Check the checker.");
    assert_eq!(last.text, "Reading it.");
    assert_eq!(last.tool.as_deref(), Some("disassemble"));
    Ok(())
}

#[test]
fn an_openai_stream_rebuilds_calls_and_reasoning() -> Result<(), Box<dyn std::error::Error>> {
    let (base, rx) = serve_seq(vec![Reply::sse(&[
        r#"{"choices":[{"index":0,"delta":{"role":"assistant","reasoning_content":"Strings first."}}]}"#,
        r#"{"choices":[{"index":0,"delta":{"tool_calls":[{"index":0,"id":"c1","type":"function","function":{"name":"search_strings","arguments":""}}]}}]}"#,
        r#"{"choices":[{"index":0,"delta":{"tool_calls":[{"index":0,"function":{"arguments":"{\"filter\":\"pass\","}}]}}]}"#,
        r#"{"choices":[{"index":0,"delta":{"tool_calls":[{"index":0,"function":{"arguments":"\"why\":\"w\"}"}}]}}]}"#,
        r#"{"choices":[{"index":0,"delta":{},"finish_reason":"tool_calls"}]}"#,
        r#"{"choices":[],"usage":{"prompt_tokens":900,"completion_tokens":20,"prompt_tokens_details":{"cached_tokens":512}}}"#,
        "[DONE]",
    ])]);
    let p = OpenAiProvider::new("deepseek", "deepseek-x", &base, Some("k"), false)?;
    let turn = p.next_live(&transcript(), &Watch::default())?;
    let body = rx.recv()?;
    assert_eq!(body["stream"], true);
    assert_eq!(body["stream_options"]["include_usage"], true);
    assert_eq!(turn.stop, Stop::ToolUse);
    assert_eq!((turn.input_tokens, turn.cache_read_tokens), (900, 512));
    assert_eq!(
        turn.raw["reasoning_content"], "Strings first.",
        "echoed back to DeepSeek"
    );
    assert!(turn.blocks.iter().any(|b| matches!(b,
        Block::ToolUse { name, input, .. } if name == "search_strings" && input["filter"] == "pass")));
    Ok(())
}

#[test]
fn a_cut_off_stream_is_retried_not_kept() -> Result<(), Box<dyn std::error::Error>> {
    let (base, _rx) = serve_seq(vec![Reply::sse(&ANTHROPIC_EVENTS[..9])]);
    let p = AnthropicProvider::new("anthropic", "claude-x", &base, "k")?;
    let e = p
        .next_live(&transcript(), &Watch::default())
        .err()
        .ok_or("accepted")?;
    assert!(e.is_transient(), "{e}");
    Ok(())
}

#[test]
fn stop_ends_a_streaming_request_at_once() -> Result<(), Box<dyn std::error::Error>> {
    let mut reply = Reply::sse(ANTHROPIC_EVENTS);
    // Up to the first thinking delta, then the server goes quiet for 5s.
    reply.hang_after = Some(reply.body.find("checker").ok_or("event")?);
    let (base, _rx) = serve_seq(vec![reply]);
    let p = AnthropicProvider::new("anthropic", "claude-x", &base, "k")?;
    let sink = Watch {
        stop_on_live: true,
        ..Default::default()
    };
    let started = Instant::now();
    let e = p.next_live(&transcript(), &sink).err().ok_or("finished")?;
    assert!(matches!(e, AgentError::Stopped), "{e}");
    assert!(
        started.elapsed() < Duration::from_secs(2),
        "{:?}",
        started.elapsed()
    );
    Ok(())
}

#[test]
fn a_server_that_cannot_stream_is_called_whole() -> Result<(), Box<dyn std::error::Error>> {
    let refused = json!({ "error": { "message": "stream mode is not supported" } });
    let whole = json!({ "choices": [{ "message": { "content": "ok" }, "finish_reason": "stop" }] });
    let (base, rx) = serve_seq(vec![
        Reply::json("400 Bad Request", &refused),
        Reply::json("200 OK", &whole),
        Reply::json("200 OK", &whole),
    ]);
    let p = OpenAiProvider::new("local", "m", &base, None, false)?;
    assert_eq!(p.next_live(&transcript(), &Watch::default())?.text(), "ok");
    assert_eq!(rx.recv()?["stream"], true);
    assert_eq!(rx.recv()?.get("stream"), None, "retried whole");
    p.next_live(&transcript(), &Watch::default())?;
    assert_eq!(rx.recv()?.get("stream"), None, "remembered");
    Ok(())
}
