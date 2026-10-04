//! Runs both providers' real HTTP paths against a local mock server.
mod common;

use common::mock::{serve_once, serve_status};
use crosure_agent::{AnthropicProvider, Block, Entry, OpenAiProvider, Provider, Stop, Transcript};
use serde_json::json;

fn transcript() -> Transcript {
    Transcript {
        entries: vec![Entry::User("Reverse it.".into())],
        ..Default::default()
    }
}

#[test]
fn openai_compatible_round_trip() -> Result<(), Box<dyn std::error::Error>> {
    let (base, rx) = serve_once(json!({
        "choices": [{ "finish_reason": "tool_calls", "message": {
            "role": "assistant", "content": null, "reasoning_content": "check strings first",
            "tool_calls": [{ "id": "call_1", "type": "function", "function": { "name": "search_strings", "arguments": "{\"filter\":\"http\",\"why\":\"look for URLs\"}" } }]
        }}],
        "usage": { "prompt_tokens": 900, "completion_tokens": 40 }
    }));
    let p = OpenAiProvider::new(
        "openrouter",
        "some/model",
        &format!("{base}/v1"),
        Some("sk-test"),
        false,
    )?;
    let turn = p.next(&transcript())?;
    let (head, body) = rx.recv()?;
    assert!(head.starts_with("POST /v1/chat/completions"), "{head}");
    assert!(head
        .to_lowercase()
        .contains("authorization: bearer sk-test"));
    assert_eq!(body["model"], "some/model");
    assert_eq!(body["tool_choice"], "auto");
    assert_eq!(turn.stop, Stop::ToolUse);
    assert_eq!(turn.input_tokens, 900);
    assert!(matches!(&turn.blocks[0], Block::Thinking(t) if t == "check strings first"));
    assert!(
        matches!(&turn.blocks[1], Block::ToolUse { name, input, .. } if name == "search_strings" && input["filter"] == "http")
    );
    Ok(())
}

#[test]
fn local_server_without_key_and_object_arguments() -> Result<(), Box<dyn std::error::Error>> {
    // Ollama returns arguments as an object and needs no key.
    let (base, rx) = serve_once(json!({
        "choices": [{ "finish_reason": "stop", "message": { "role": "assistant", "content": "done",
            "tool_calls": [{ "function": { "name": "binary_info", "arguments": { "why": "start" } } }] } }]
    }));
    let p = OpenAiProvider::new(
        "ollama",
        "qwen2.5-coder:14b",
        &format!("{base}/v1"),
        None,
        false,
    )?;
    let turn = p.next(&transcript())?;
    let (head, _) = rx.recv()?;
    assert!(!head.to_lowercase().contains("authorization"));
    assert_eq!(
        turn.stop,
        Stop::ToolUse,
        "tool calls win over finish_reason stop"
    );
    assert!(
        matches!(&turn.blocks[1], Block::ToolUse { id, input, .. } if id == "call_0" && input["why"] == "start")
    );
    Ok(())
}

#[test]
fn openai_refusal_and_content_filter() -> Result<(), Box<dyn std::error::Error>> {
    let (base, _rx) = serve_once(
        json!({ "choices": [{ "finish_reason": "stop", "message": { "role": "assistant", "content": null, "refusal": "I can't help with that." } }] }),
    );
    let turn = OpenAiProvider::new("openai", "m", &base, Some("k"), true)?.next(&transcript())?;
    assert!(
        matches!(turn.stop, Stop::Refusal { explanation: Some(ref e), .. } if e.contains("can't"))
    );
    let (base, _rx) = serve_once(
        json!({ "choices": [{ "finish_reason": "content_filter", "message": { "role": "assistant", "content": "" } }] }),
    );
    let turn = OpenAiProvider::new("gemini", "m", &base, Some("k"), false)?.next(&transcript())?;
    assert!(
        matches!(turn.stop, Stop::Refusal { category: Some(ref c), .. } if c == "content_filter")
    );
    Ok(())
}

#[test]
fn anthropic_round_trip_headers_and_refusal() -> Result<(), Box<dyn std::error::Error>> {
    let (base, rx) = serve_once(json!({
        "stop_reason": "refusal", "content": [],
        "stop_details": { "type": "refusal", "category": "cyber", "explanation": "declined" },
        "usage": { "input_tokens": 10, "cache_read_input_tokens": 90, "output_tokens": 0 }
    }));
    let p = AnthropicProvider::new("anthropic", "claude-opus-5-5", &base, "sk-ant-test")?
        .with_claude_api(true);
    let turn = p.next(&transcript())?;
    let (head, body) = rx.recv()?;
    let h = head.to_lowercase();
    assert!(head.starts_with("POST /v1/messages"));
    assert!(h.contains("x-api-key: sk-ant-test"));
    assert!(h.contains("anthropic-version: 2023-06-01"));
    assert!(h.contains("anthropic-beta: server-side-fallback-2026-07-01"));
    assert_eq!(body["fallbacks"], "default");
    assert_eq!(body["thinking"]["type"], "adaptive");
    assert_eq!(turn.input_tokens, 100, "cache reads count as input");
    assert!(matches!(turn.stop, Stop::Refusal { category: Some(ref c), .. } if c == "cyber"));
    Ok(())
}

#[test]
fn anthropic_compatible_server_gets_a_plain_request() -> Result<(), Box<dyn std::error::Error>> {
    let (base, rx) = serve_once(json!({
        "stop_reason": "end_turn", "content": [{ "type": "text", "text": "ok" }],
        "usage": { "input_tokens": 3, "output_tokens": 1 }
    }));
    let p = AnthropicProvider::new(
        "custom-anthropic",
        "glm-4.6",
        &format!("{base}/api/anthropic"),
        "k",
    )?;
    let turn = p.next(&transcript())?;
    let (head, body) = rx.recv()?;
    assert!(head.starts_with("POST /api/anthropic/v1/messages"));
    assert!(!head.to_lowercase().contains("anthropic-beta"));
    for k in ["thinking", "output_config", "fallbacks", "cache_control"] {
        assert!(body.get(k).is_none(), "{k} sent to a compatible server");
    }
    assert_eq!(body["model"], "glm-4.6");
    assert_eq!(turn.text(), "ok");
    Ok(())
}

#[test]
fn model_list_strips_gemini_prefix() -> Result<(), Box<dyn std::error::Error>> {
    let (base, rx) = serve_once(json!({ "data": [
        { "id": "models/gemini-2.5-pro" }, { "id": "kimi-k2" }
    ] }));
    let mut cfg = crosure_agent::presets()
        .into_iter()
        .find(|p| p.id == "custom")
        .ok_or("no custom preset")?;
    cfg.base_url = base;
    let ids = crosure_agent::list_models(&cfg)?;
    assert!(rx.recv()?.0.starts_with("GET /models"));
    assert_eq!(ids, vec!["gemini-2.5-pro", "kimi-k2"]);
    Ok(())
}

#[test]
fn openai_extras_reach_the_wire() -> Result<(), Box<dyn std::error::Error>> {
    let (base, rx) = serve_once(json!({
        "choices": [{ "message": { "content": "ok" }, "finish_reason": "stop" }]
    }));
    let extras = crosure_agent::Extras {
        headers: vec![
            ("X-Gateway-Tag".into(), "crosure".into()),
            ("Authorization".into(), "Bearer hijack".into()),
        ],
        reasoning_effort: Some("high".into()),
        max_completion_tokens: true,
    };
    let p =
        OpenAiProvider::new("openai", "gpt-5", &base, Some("sk-real"), true)?.with_extras(extras);
    p.next(&transcript())?;
    let (head, body) = rx.recv()?;
    let h = head.to_lowercase();
    assert!(h.contains("x-gateway-tag: crosure"));
    assert!(h.contains("authorization: bearer sk-real"));
    assert!(
        !h.contains("hijack"),
        "managed headers cannot be overridden"
    );
    assert_eq!(body["max_completion_tokens"], 8192);
    assert!(body.get("max_tokens").is_none());
    assert_eq!(body["reasoning_effort"], "high");
    Ok(())
}

#[test]
fn claude_api_effort_none_turns_thinking_off() -> Result<(), Box<dyn std::error::Error>> {
    let (base, rx) = serve_once(json!({
        "stop_reason": "end_turn", "content": [{ "type": "text", "text": "ok" }],
        "usage": { "input_tokens": 1, "output_tokens": 1 }
    }));
    let extras = crosure_agent::Extras {
        reasoning_effort: Some("none".into()),
        ..Default::default()
    };
    let p = AnthropicProvider::new("anthropic", "claude-opus-5-5", &base, "k")?
        .with_claude_api(true)
        .with_extras(extras);
    p.next(&transcript())?;
    let (_, body) = rx.recv()?;
    assert!(body.get("thinking").is_none() && body.get("output_config").is_none());
    assert_eq!(body["fallbacks"], "default");
    Ok(())
}

#[test]
fn status_codes_are_classified_for_retry() -> Result<(), Box<dyn std::error::Error>> {
    let cases: [(&'static str, &'static str, bool); 4] = [
        ("529 Overloaded", "", true),
        ("429 Too Many Requests", "retry-after: 7\r\n", true),
        ("400 Bad Request", "", false),
        ("501 Not Implemented", "", false),
    ];
    for (status, headers, transient) in cases {
        let (base, _rx) = serve_status(json!({ "error": { "message": "nope" } }), status, headers);
        let p = OpenAiProvider::new("p", "m", &base, None, false)?;
        let err = p.next(&transcript()).err().ok_or("expected an error")?;
        assert_eq!(err.is_transient(), transient, "{status}: {err}");
        if headers.contains("retry-after") {
            assert_eq!(err.retry_after(), Some(7));
        }
    }
    Ok(())
}
