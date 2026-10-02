//! `crosure-reverse <binary> [task]`: let the AI reverse a file from the
//! terminal. Every step is recorded to the same store the app uses, so the
//! session opens in Crosure afterwards (graph, replay, verification).
//!
//! Key: `ANTHROPIC_API_KEY`. Model: `CROSURE_MODEL` (default claude-opus-5-5).
//! `--demo` runs the scripted model instead (no key; for the bundled crackme).

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use crosure_agent::{
    run_agent, AgentConfig, AgentEvent, ClaudeHttp, Llm, ScriptedLlm, SessionExecutor, Sink,
    DEFAULT_MODEL,
};
use crosure_recorder::Store;
use crosure_session::Workspace;

struct Print;

impl Sink for Print {
    fn emit(&self, e: AgentEvent) {
        match e {
            AgentEvent::Started { model, .. } => eprintln!("· model {model}"),
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
                category,
                explanation,
            } => eprintln!(
                "! declined ({}): {}",
                category.unwrap_or_default(),
                explanation.unwrap_or_default()
            ),
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
    let llm: Box<dyn Llm> = if demo {
        Box::new(ScriptedLlm::crackme_demo())
    } else {
        let key = std::env::var("ANTHROPIC_API_KEY")
            .map_err(|_| "set ANTHROPIC_API_KEY (or use --demo)")?;
        let model = std::env::var("CROSURE_MODEL").unwrap_or_else(|_| DEFAULT_MODEL.into());
        Box::new(ClaudeHttp::new(
            &key,
            &model,
            std::env::var("ANTHROPIC_BASE_URL").ok().as_deref(),
        )?)
    };
    let dir = home();
    std::fs::create_dir_all(&dir)?;
    let store = Store::open(&dir.join("crosure.db"))?;
    let (ws, _) = Workspace::open(&store, std::path::Path::new(binary.as_str()), None)?;
    ws.record_task(&store, task)?;
    let sid = ws.session.id.clone();
    let exec = SessionExecutor {
        store: Arc::new(Mutex::new(store)),
        workspace: Arc::new(Mutex::new(Some(ws))),
        model: llm.model().into(),
    };
    let report = run_agent(llm.as_ref(), &exec, &Print, task, &AgentConfig::default())?;
    let store = exec.store.lock().map_err(|_| "lock")?;
    if !report.is_empty() {
        if let Some(ws) = exec.workspace.lock().map_err(|_| "lock")?.as_ref() {
            ws.record_report(&store, llm.model(), &report)?;
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
