//! `crosure-reverse [--demo] <binary> [task]`: let an AI reverse a file from
//! the terminal. Every step is recorded to the same store the app uses, so the
//! session opens in Crosure afterwards (graph, replay, verification).
//!
//! Providers come from `~/.crosure/agent.json` (set up in the app). With no
//! file, Claude is used with `ANTHROPIC_API_KEY`. `--demo` runs the scripted
//! model instead (no key; for the bundled crackme).

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use crosure_agent::{
    build_chain, run_agent, AgentConfig, AgentEvent, AgentSettings, Provider, ScriptedProvider,
    SessionExecutor, Sink,
};
use crosure_recorder::Store;
use crosure_session::Workspace;

struct Print;

impl Sink for Print {
    fn emit(&self, e: AgentEvent) {
        match e {
            AgentEvent::Started {
                model, provider, ..
            } => eprintln!("· {provider} / {model}"),
            AgentEvent::Thinking { text } => eprintln!("  … {}", text.lines().next().unwrap_or("")),
            AgentEvent::Message { text } => eprintln!("  {text}"),
            AgentEvent::ToolCall {
                command,
                why,
                summary,
                error,
                ..
            } => {
                eprintln!("▸ {command}");
                if !why.is_empty() {
                    eprintln!("    why: {why}");
                }
                if let Some(s) = summary {
                    eprintln!("    {s}");
                }
                if let Some(err) = error {
                    eprintln!("    error: {err}");
                }
            }
            AgentEvent::Refusal {
                provider,
                category,
                explanation,
            } => eprintln!(
                "! {provider} declined ({}): {}",
                category.unwrap_or_default(),
                explanation.unwrap_or_default()
            ),
            AgentEvent::Switched { from, to } => eprintln!("· {from} declined; continuing on {to}"),
            AgentEvent::Finished {
                tool_calls,
                input_tokens,
                output_tokens,
                ..
            } => {
                eprintln!("· done: {tool_calls} recorded steps, {input_tokens} in / {output_tokens} out tokens")
            }
            AgentEvent::Failed { error } => eprintln!("! {error}"),
            AgentEvent::Stopped => eprintln!("· stopped"),
            AgentEvent::Usage { .. }
            | AgentEvent::ApprovalRequested { .. }
            | AgentEvent::ApprovalResolved { .. } => {}
        }
    }
    fn should_stop(&self) -> bool {
        false
    }
}

fn home() -> PathBuf {
    std::env::var_os("CROSURE_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".crosure")))
        .unwrap_or_else(|| PathBuf::from(".crosure"))
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let demo = args.iter().any(|a| a == "--demo");
    let rest: Vec<&String> = args.iter().filter(|a| *a != "--demo").collect();
    let Some(binary) = rest.first() else {
        eprintln!("usage: crosure-reverse [--demo] <binary> [task]");
        std::process::exit(2);
    };
    let task = rest
        .get(1)
        .map(|s| s.as_str())
        .unwrap_or("Reverse this binary and explain what it does.");
    let dir = home();
    std::fs::create_dir_all(&dir)?;
    let chain: Vec<Box<dyn Provider>> = if demo {
        vec![Box::new(ScriptedProvider::crackme_demo())]
    } else {
        let settings = std::fs::read(dir.join("agent.json"))
            .map(|b| AgentSettings::from_json(&b))
            .unwrap_or_default();
        build_chain(&settings).map_err(|_| {
            "no provider is ready: set ANTHROPIC_API_KEY, configure one in the app, or use --demo"
        })?
    };
    let store = Store::open(&dir.join("crosure.db"))?;
    let (ws, _) = Workspace::open(&store, std::path::Path::new(binary.as_str()), None)?;
    ws.record_task(&store, task)?;
    let sid = ws.session.id.clone();
    let exec = SessionExecutor {
        store: Arc::new(Mutex::new(store)),
        workspace: Arc::new(Mutex::new(Some(ws))),
    };
    let report = run_agent(&chain, &exec, &Print, task, &AgentConfig::default())?;
    let store = exec.store.lock().map_err(|_| "lock")?;
    if !report.is_empty() {
        if let Some(ws) = exec.workspace.lock().map_err(|_| "lock")?.as_ref() {
            ws.record_report(&store, &chain[0].tag(), &report)?;
        }
        println!("{report}");
    }
    let v = store.verify(&sid)?;
    eprintln!(
        "· session {sid}: {} steps, chain {}",
        v.checked,
        if v.ok { "verified" } else { "BROKEN" }
    );
    Ok(())
}
