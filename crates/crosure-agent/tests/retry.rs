//! Temporary provider failures are retried visibly; Stop always wins.
mod common;

use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::time::{Duration, Instant};

use common::{setup, Collect};
use crosure_agent::{
    run_agent_turn, AgentConfig, AgentError, AgentEvent, Entry, Provider, ScriptedProvider, Sink,
    Transcript, Turn,
};
use serde_json::json;

/// Fails the first `failures` calls with `error`, then follows the script.
struct Flaky {
    failures: AtomicU32,
    error: fn() -> AgentError,
    script: ScriptedProvider,
}

impl Provider for Flaky {
    fn id(&self) -> &str {
        "flaky"
    }
    fn model(&self) -> &str {
        "m"
    }
    fn next(&self, t: &Transcript) -> Result<Turn, AgentError> {
        if self.failures.load(Ordering::SeqCst) > 0 {
            self.failures.fetch_sub(1, Ordering::SeqCst);
            return Err((self.error)());
        }
        self.script.next(t)
    }
}

fn overloaded() -> AgentError {
    AgentError::Transient {
        status: 529,
        message: "Overloaded".into(),
        retry_after: None,
    }
}

fn fast() -> AgentConfig {
    AgentConfig {
        retry_base_ms: 1,
        ..Default::default()
    }
}

fn flaky(failures: u32, error: fn() -> AgentError) -> Vec<Box<dyn Provider>> {
    vec![Box::new(Flaky {
        failures: AtomicU32::new(failures),
        error,
        script: ScriptedProvider::new("flaky", vec![ScriptedProvider::done("ok")]),
    })]
}

#[test]
fn temporary_failures_are_retried_and_shown() -> Result<(), Box<dyn std::error::Error>> {
    let (exec, _) = setup()?;
    let sink = Collect::default();
    let mut t = Transcript::default();
    let report = run_agent_turn(&flaky(2, overloaded), &exec, &sink, &mut t, "go", &fast())?;
    assert_eq!(report, "ok");
    let events = sink.0.lock().map_err(|_| "lock")?;
    let delays: Vec<u64> = events
        .iter()
        .filter_map(|e| match e {
            AgentEvent::Retrying { delay_ms, .. } => Some(*delay_ms),
            _ => None,
        })
        .collect();
    assert_eq!(delays.len(), 2);
    Ok(())
}

#[test]
fn final_errors_are_not_retried() -> Result<(), Box<dyn std::error::Error>> {
    let (exec, _) = setup()?;
    let sink = Collect::default();
    let mut t = Transcript::default();
    let bad_key = || AgentError::Auth("invalid x-api-key".into());
    let r = run_agent_turn(&flaky(1, bad_key), &exec, &sink, &mut t, "go", &fast());
    assert!(matches!(r, Err(AgentError::Auth(_))));
    let events = sink.0.lock().map_err(|_| "lock")?;
    assert!(!events
        .iter()
        .any(|e| matches!(e, AgentEvent::Retrying { .. })));
    Ok(())
}

/// Presses Stop as soon as a given event is emitted.
struct StopOn(Collect, fn(&AgentEvent) -> bool, AtomicBool);

impl Sink for StopOn {
    fn emit(&self, e: AgentEvent) {
        if (self.1)(&e) {
            self.2.store(true, Ordering::SeqCst);
        }
        self.0.emit(e);
    }
    fn should_stop(&self) -> bool {
        self.2.load(Ordering::SeqCst)
    }
}

#[test]
fn stop_during_a_retry_wait_returns_promptly() -> Result<(), Box<dyn std::error::Error>> {
    let (exec, _) = setup()?;
    let sink = StopOn(
        Collect::default(),
        |e| matches!(e, AgentEvent::Retrying { .. }),
        AtomicBool::new(false),
    );
    let cfg = AgentConfig {
        retry_base_ms: 30_000,
        ..Default::default()
    };
    let mut t = Transcript::default();
    let started = Instant::now();
    let report = run_agent_turn(&flaky(1, overloaded), &exec, &sink, &mut t, "go", &cfg)?;
    assert!(
        started.elapsed() < Duration::from_secs(2),
        "did not wait 30s"
    );
    assert_eq!(report, "");
    let events = sink.0 .0.lock().map_err(|_| "lock")?;
    assert!(matches!(events.last(), Some(AgentEvent::Stopped)));
    Ok(())
}

#[test]
fn stop_mid_batch_still_answers_every_call() -> Result<(), Box<dyn std::error::Error>> {
    let (exec, sid) = setup()?;
    let two_calls = json!({
        "stop_reason": "tool_use",
        "content": [
            { "type": "tool_use", "id": "a", "name": "list_imports", "input": { "why": "1" } },
            { "type": "tool_use", "id": "b", "name": "binary_info", "input": { "why": "2" } }
        ],
        "usage": { "input_tokens": 1, "output_tokens": 1 }
    });
    let chain: Vec<Box<dyn Provider>> = vec![Box::new(ScriptedProvider::new("s", vec![two_calls]))];
    let sink = StopOn(
        Collect::default(),
        |e| matches!(e, AgentEvent::ToolCall { .. }),
        AtomicBool::new(false),
    );
    let mut t = Transcript::default();
    let report = run_agent_turn(&chain, &exec, &sink, &mut t, "go", &AgentConfig::default())?;
    assert_eq!(report, "");
    let Some(Entry::Results { results, .. }) = t.entries.last() else {
        return Err("results missing".into());
    };
    let ids: Vec<&str> = results.iter().map(|r| r.id.as_str()).collect();
    assert_eq!(ids, ["a", "b"], "both calls answered");
    assert!(results[1].is_error && results[1].content.contains("stopped"));
    // Only the first call ran and was recorded (load + 1 step).
    let store = exec.store.lock().map_err(|_| "lock")?;
    assert_eq!(store.steps(&sid)?.len(), 2);
    Ok(())
}
