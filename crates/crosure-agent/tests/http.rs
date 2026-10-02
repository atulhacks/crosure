//! Runs both providers' real HTTP paths against a local mock server.
#![allow(clippy::expect_used)] // the mock server thread fails the test loudly on I/O errors

use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpListener;
use std::sync::mpsc;

use crosure_agent::{AnthropicProvider, Block, Entry, OpenAiProvider, Provider, Stop, Transcript};
use serde_json::{json, Value};

/// Serves one request with `reply`, returns (request line + headers, body).
fn serve_once(reply: Value) -> (String, mpsc::Receiver<(String, Value)>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
    let addr = format!("http://{}", listener.local_addr().expect("addr"));
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        let (stream, _) = listener.accept().expect("accept");
        let mut reader = BufReader::new(stream.try_clone().expect("clone"));
        let mut head = String::new();
        let mut len = 0usize;
        loop {
            let mut line = String::new();
            reader.read_line(&mut line).expect("line");
            if line == "\r\n" || line.is_empty() {
                break;
            }
            if let Some(v) = line.to_lowercase().strip_prefix("content-length:") {
                len = v.trim().parse().unwrap_or(0);
            }
            head.push_str(&line);
        }
        let mut body = vec![0; len];
        reader.read_exact(&mut body).expect("body");
        let payload = reply.to_string();
        let mut stream = stream;
        write!(stream, "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{}", payload.len(), payload).expect("write");
        tx.send((head, serde_json::from_slice(&body).unwrap_or(Value::Null)))
            .expect("send");
    });
    (addr, rx)
}

fn transcript() -> Transcript {
    Transcript {
        entries: vec![Entry::User("Reverse it.".into())],
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
    let p = AnthropicProvider::new("anthropic", "claude-opus-5-5", &base, "sk-ant-test")?;
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
