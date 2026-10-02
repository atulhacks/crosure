use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use crosure_agent::{run_agent, AgentConfig, AgentEvent, ScriptedLlm, SessionExecutor, Sink};
use crosure_recorder::{ActorKind, StepKind, Store};
use crosure_session::Workspace;
use serde_json::json;

#[derive(Default)]
struct Collect(Mutex<Vec<AgentEvent>>, bool);

impl Sink for Collect {
    fn emit(&self, e: AgentEvent) {
        if let Ok(mut v) = self.0.lock() {
            v.push(e);
        }
    }
    fn should_stop(&self) -> bool {
        self.1
    }
}

fn setup() -> Result<(SessionExecutor, String), Box<dyn std::error::Error>> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../crosure-engine/tests/fixtures/crackme-x64");
    let store = Store::open_in_memory()?;
    let (ws, _) = Workspace::open(&store, &path, None)?;
    let sid = ws.session.id.clone();
    let exec = SessionExecutor {
        store: Arc::new(Mutex::new(store)),
        workspace: Arc::new(Mutex::new(Some(ws))),
        model: "scripted-demo".into(),
    };
    Ok((exec, sid))
}

#[test]
fn agent_reverses_the_crackme_and_every_step_is_recorded() -> Result<(), Box<dyn std::error::Error>>
{
    let (exec, sid) = setup()?;
    let llm = ScriptedLlm::crackme_demo();
    let sink = Collect::default();
    let report = run_agent(
        &llm,
        &exec,
        &sink,
        "Reverse this binary.",
        &AgentConfig::default(),
    )?;
    assert!(report.contains("## Verdict"));

    let store = exec.store.lock().map_err(|_| "lock")?;
    let steps = store.steps(&sid)?;
    let agent: Vec<_> = steps
        .iter()
        .filter(|s| s.actor.kind == ActorKind::Agent)
        .collect();
    assert_eq!(agent.len(), 10, "one recorded step per tool call");
    assert!(
        agent
            .iter()
            .all(|s| s.intent.as_ref().and_then(|i| i.note.as_ref()).is_some()),
        "every agent step carries its why"
    );
    assert!(agent
        .iter()
        .all(|s| s.actor.model.as_deref() == Some("scripted-demo")));
    assert!(steps.iter().any(|s| s.kind == StepKind::Verdict));
    assert!(store.verify(&sid)?.ok);
    let ws = exec.workspace.lock().map_err(|_| "lock")?;
    assert!(
        ws.as_ref()
            .ok_or("ws")?
            .functions()?
            .iter()
            .any(|f| f.name == "xor_decode"),
        "rename applied"
    );

    let events = sink.0.lock().map_err(|_| "lock")?;
    assert!(matches!(events.first(), Some(AgentEvent::Started { .. })));
    assert!(matches!(
        events.last(),
        Some(AgentEvent::Finished { tool_calls: 10, .. })
    ));
    Ok(())
}

#[test]
fn requests_follow_the_api_contract() -> Result<(), Box<dyn std::error::Error>> {
    let (exec, _) = setup()?;
    let llm = ScriptedLlm::crackme_demo();
    run_agent(
        &llm,
        &exec,
        &Collect::default(),
        "Reverse this binary.",
        &AgentConfig::default(),
    )?;
    let reqs = llm.requests.lock().map_err(|_| "lock")?;
    let second = &reqs[1];
    let msgs = second["messages"].as_array().ok_or("messages")?;
    // assistant turn echoed verbatim, thinking block included, then one user turn of tool results
    assert_eq!(msgs[1]["content"][0]["type"], "thinking");
    assert_eq!(msgs[2]["content"][0]["type"], "tool_result");
    assert_eq!(msgs[2]["content"][0]["tool_use_id"], "t1");
    assert_eq!(second["output_config"]["effort"], "high");
    assert!(second.get("temperature").is_none());
    // disassembly reaches the model as compact text with resolved names
    let dis = reqs[5]["messages"]
        .as_array()
        .ok_or("m")?
        .last()
        .ok_or("last")?["content"][0]["content"]
        .as_str()
        .ok_or("text")?
        .to_string();
    assert!(dis.contains("call 0x1090  ; strcmp@plt"), "{dis}");
    Ok(())
}

#[test]
fn refusal_bad_input_and_stop_are_handled() -> Result<(), Box<dyn std::error::Error>> {
    let (exec, sid) = setup()?;
    let refusal = ScriptedLlm::new(vec![
        json!({"stop_reason": "refusal", "content": [], "stop_details": {"type": "refusal", "category": "cyber", "explanation": "x"}, "usage": {}}),
    ]);
    let sink = Collect::default();
    run_agent(&refusal, &exec, &sink, "go", &AgentConfig::default())?;
    assert!(sink
        .0
        .lock()
        .map_err(|_| "lock")?
        .iter()
        .any(|e| matches!(e, AgentEvent::Refusal { category: Some(c), .. } if c == "cyber")));

    let bad = ScriptedLlm::new(vec![
        ScriptedLlm::tool("b1", "disassemble", json!({"why": "no target"}), "oops"),
        ScriptedLlm::tool(
            "b2",
            "disassemble",
            json!({"target": "nope", "why": "missing fn"}),
            "again",
        ),
        ScriptedLlm::done("done"),
    ]);
    let sink = Collect::default();
    run_agent(&bad, &exec, &sink, "go", &AgentConfig::default())?;
    let errors = sink
        .0
        .lock()
        .map_err(|_| "lock")?
        .iter()
        .filter(|e| matches!(e, AgentEvent::ToolCall { error: Some(_), .. }))
        .count();
    assert_eq!(errors, 2);
    let reqs = bad.requests.lock().map_err(|_| "lock")?;
    assert_eq!(reqs[1]["messages"][2]["content"][0]["is_error"], true);
    assert_eq!(
        exec.store.lock().map_err(|_| "lock")?.steps(&sid)?.len(),
        1,
        "failed calls record nothing"
    );

    let stopped = Collect(Mutex::new(Vec::new()), true);
    run_agent(
        &ScriptedLlm::crackme_demo(),
        &exec,
        &stopped,
        "go",
        &AgentConfig::default(),
    )?;
    assert!(matches!(
        stopped.0.lock().map_err(|_| "lock")?.last(),
        Some(AgentEvent::Stopped)
    ));
    Ok(())
}
