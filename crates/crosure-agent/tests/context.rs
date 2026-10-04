//! Long runs stay inside the context window; large results can be paged.
mod common;

use std::sync::Mutex;

use common::{setup, Collect};
use crosure_agent::{
    estimate_tokens, render_result, run_agent_turn, AgentConfig, AgentError, AgentEvent, Block,
    Entry, Provider, ScriptedProvider, Transcript, Turn,
};
use serde_json::json;

/// Replays scripted turns, can fail the first call with a "too long"
/// error, and records the estimated size of every request it receives.
struct Probe {
    script: ScriptedProvider,
    fail_first: Mutex<bool>,
    sizes: Mutex<Vec<usize>>,
}

impl Provider for Probe {
    fn id(&self) -> &str {
        "probe"
    }
    fn model(&self) -> &str {
        "probe"
    }
    fn next(&self, t: &Transcript) -> Result<Turn, AgentError> {
        if let Ok(mut s) = self.sizes.lock() {
            s.push(estimate_tokens(t));
        }
        if let Ok(mut f) = self.fail_first.lock() {
            if std::mem::take(&mut *f) {
                return Err(AgentError::Api {
                    status: 400,
                    message: "prompt is too long: 250000 tokens > 200000 maximum".into(),
                });
            }
        }
        self.script.next(t)
    }
}

/// `n` calls that each return a whole function's disassembly (kilobytes).
fn listing_run(n: usize) -> ScriptedProvider {
    let mut turns: Vec<_> = (0..n)
        .map(|i| {
            ScriptedProvider::tool(
                &format!("c{i}"),
                "disassemble",
                json!({ "target": "main", "offset": null, "why": "read it" }),
                "look",
            )
        })
        .collect();
    turns.push(ScriptedProvider::done("done"));
    ScriptedProvider::new("probe", turns)
}

fn pairs_are_complete(t: &Transcript) -> bool {
    let mut open: Vec<String> = Vec::new();
    for e in &t.entries {
        match e {
            Entry::Assistant { turn, .. } => {
                open = turn
                    .blocks
                    .iter()
                    .filter_map(|b| match b {
                        Block::ToolUse { id, .. } => Some(id.clone()),
                        _ => None,
                    })
                    .collect();
            }
            Entry::Results { results, .. } => {
                let ids: Vec<String> = results.iter().map(|r| r.id.clone()).collect();
                if ids != open {
                    return false;
                }
                open.clear();
            }
            Entry::User(_) => {}
        }
    }
    open.is_empty()
}

#[test]
fn a_long_run_is_trimmed_to_its_budget() -> Result<(), Box<dyn std::error::Error>> {
    let (exec, _) = setup()?;
    let probe = Probe {
        script: listing_run(12),
        fail_first: Mutex::new(false),
        sizes: Mutex::new(Vec::new()),
    };
    let chain: Vec<Box<dyn Provider>> = vec![Box::new(probe)];
    let sink = Collect::default();
    let cfg = AgentConfig {
        context_tokens: 2_500,
        ..Default::default()
    };
    let mut t = Transcript::default();
    let report = run_agent_turn(&chain, &exec, &sink, &mut t, "Survey it.", &cfg)?;
    assert_eq!(report, "done");
    assert!(pairs_are_complete(&t), "every tool call keeps its result");
    let trims = sink
        .0
        .lock()
        .map_err(|_| "lock")?
        .iter()
        .filter(|e| matches!(e, AgentEvent::ContextTrimmed { .. }))
        .count();
    assert!(trims > 0, "trimming was reported");
    assert!(
        estimate_tokens(&t) <= 2_500 + 2_000,
        "{}",
        estimate_tokens(&t)
    );
    Ok(())
}

#[test]
fn a_too_long_prompt_is_trimmed_and_retried_once() -> Result<(), Box<dyn std::error::Error>> {
    let (exec, _) = setup()?;
    let sink = Collect::default();
    let cfg = AgentConfig::default();
    // A first run leaves six large listings in the thread.
    let mut t = Transcript::default();
    let first: Vec<Box<dyn Provider>> = vec![Box::new(listing_run(6))];
    run_agent_turn(&first, &exec, &sink, &mut t, "Survey it.", &cfg)?;
    let before = estimate_tokens(&t);

    // The follow-up is rejected as too long once, then succeeds.
    let probe = Probe {
        script: ScriptedProvider::new("probe", vec![ScriptedProvider::done("after trim")]),
        fail_first: Mutex::new(true),
        sizes: Mutex::new(Vec::new()),
    };
    let chain: Vec<Box<dyn Provider>> = vec![Box::new(probe)];
    let report = run_agent_turn(&chain, &exec, &sink, &mut t, "And now?", &cfg)?;
    assert_eq!(report, "after trim");
    assert!(
        estimate_tokens(&t) < before,
        "results were elided before the retry"
    );
    assert!(pairs_are_complete(&t));
    Ok(())
}

#[test]
fn large_results_page_with_an_exact_next_offset() {
    let lines: Vec<_> = (0..5000)
        .map(|i| json!({ "addr": 0x1000 + i * 4, "mnemonic": "nop", "operands": "" }))
        .collect();
    let r = json!({ "function": { "name": "big", "addr": 0x1000 }, "instructions": lines, "blocks": [] });
    let first = render_result("disasm", "5000 instructions", &r, 0);
    let cut = first
        .split("call again with offset ")
        .nth(1)
        .and_then(|s| s.split_whitespace().next())
        .and_then(|n| n.trim_end_matches(',').parse::<usize>().ok())
        .unwrap_or(0);
    assert!(cut > 100 && cut < 5000, "{cut}");
    let second = render_result("disasm", "5000 instructions", &r, cut);
    assert!(second.contains(&format!("[lines {cut}..5000 of 5000]")));
    assert!(second.contains(&format!("{:#x}", 0x1000 + cut * 4)));
    assert!(!second.contains(&format!("  {:#x}  ", 0x1000 + (cut - 1) * 4)));
}
