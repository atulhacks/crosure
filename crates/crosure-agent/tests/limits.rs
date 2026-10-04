//! Model limits reach the wire, usage is read in full, and a server that
//! silently drops the start of a long prompt is detected.
mod common;

use common::mock::serve_once;
use common::{setup, Collect};
use crosure_agent::{
    run_agent_turn, AgentConfig, AgentError, AgentEvent, AnthropicProvider, Entry, ModelLimits,
    OpenAiProvider, Provider, ScriptedProvider, Transcript, Turn,
};
use serde_json::json;

fn transcript() -> Transcript {
    Transcript {
        entries: vec![Entry::User("Reverse it.".into())],
        ..Default::default()
    }
}

const LIMITS: ModelLimits = ModelLimits {
    context_window: Some(64_000),
    max_output: Some(4_000),
};

#[test]
fn anthropic_output_limit_and_cache_usage() -> Result<(), Box<dyn std::error::Error>> {
    let (base, rx) = serve_once(json!({
        "content": [{ "type": "text", "text": "ok" }], "stop_reason": "end_turn",
        "usage": { "input_tokens": 100, "cache_read_input_tokens": 9000,
                   "cache_creation_input_tokens": 500, "output_tokens": 20 }
    }));
    let p = AnthropicProvider::new("anthropic", "claude-x", &base, "k")?.with_limits(LIMITS);
    let turn = p.next(&transcript())?;
    let (_, body) = rx.recv()?;
    assert_eq!(body["max_tokens"], 4000);
    assert_eq!(turn.input_tokens, 9600, "the whole prompt");
    assert_eq!(
        (turn.cache_read_tokens, turn.cache_write_tokens),
        (9000, 500)
    );
    assert_eq!(p.limits(), LIMITS);
    Ok(())
}

#[test]
fn openai_output_limit_and_cached_tokens() -> Result<(), Box<dyn std::error::Error>> {
    let (base, rx) = serve_once(json!({
        "choices": [{ "message": { "content": "ok" }, "finish_reason": "stop" }],
        "usage": { "prompt_tokens": 5000, "completion_tokens": 10,
                   "prompt_tokens_details": { "cached_tokens": 4096 } }
    }));
    let p = OpenAiProvider::new("openai", "gpt-x", &base, Some("k"), false)?.with_limits(LIMITS);
    let turn = p.next(&transcript())?;
    let (_, body) = rx.recv()?;
    assert_eq!(body["max_tokens"], 4000);
    assert_eq!((turn.input_tokens, turn.cache_read_tokens), (5000, 4096));
    Ok(())
}

#[test]
fn listed_limits_are_read() -> Result<(), Box<dyn std::error::Error>> {
    let (base, rx) = serve_once(json!({ "data": [
        { "id": "llama", "meta": { "n_ctx": 8192, "n_ctx_train": 131072 } }
    ] }));
    let mut cfg = crosure_agent::presets()
        .into_iter()
        .find(|p| p.id == "custom")
        .ok_or("no custom preset")?;
    cfg.base_url = base;
    let models = crosure_agent::list_model_info(&cfg)?;
    assert!(rx.recv()?.0.starts_with("GET /models"));
    assert_eq!(
        models[0].limits.context_window,
        Some(8192),
        "the loaded size, not the trained one"
    );
    Ok(())
}

/// A server whose context holds 2500 tokens: it reports a prompt no larger
/// than that, whatever was sent, as Ollama does when it truncates.
struct Truncating {
    script: ScriptedProvider,
}

impl Provider for Truncating {
    fn id(&self) -> &str {
        "local"
    }
    fn model(&self) -> &str {
        "small"
    }
    fn next(&self, t: &Transcript) -> Result<Turn, AgentError> {
        let mut turn = self.script.next(t)?;
        turn.input_tokens = crosure_agent::estimate_tokens(t).min(2_500) as u64;
        Ok(turn)
    }
}

#[test]
fn a_truncating_server_is_detected_and_the_prompt_fitted() -> Result<(), Box<dyn std::error::Error>>
{
    let (exec, _) = setup()?;
    let call = |i: usize| {
        ScriptedProvider::tool(
            &format!("c{i}"),
            "disassemble",
            json!({ "target": "main", "offset": null, "why": "read it" }),
            "look",
        )
    };
    let mut turns: Vec<_> = (0..30).map(call).collect();
    turns.push(ScriptedProvider::done("done"));
    let chain: Vec<Box<dyn Provider>> = vec![Box::new(Truncating {
        script: ScriptedProvider::new("local", turns),
    })];
    let sink = Collect::default();
    let mut t = Transcript::default();
    run_agent_turn(
        &chain,
        &exec,
        &sink,
        &mut t,
        "Survey it.",
        &AgentConfig::default(),
    )?;
    let events = sink.0.lock().map_err(|_| "lock")?;
    let warned = events
        .iter()
        .filter(|e| matches!(e, AgentEvent::PromptTruncated { .. }))
        .count();
    assert_eq!(warned, 1, "warned once");
    assert!(events
        .iter()
        .any(|e| matches!(e, AgentEvent::ContextTrimmed { .. })));
    // After the warning, every request fits what the server keeps.
    let last = events.iter().rev().find_map(|e| match e {
        AgentEvent::Usage { context_limit, .. } => Some(*context_limit),
        _ => None,
    });
    assert!(last.is_some_and(|l| l <= 2_500), "{last:?}");
    Ok(())
}
