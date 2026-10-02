mod common;

use std::sync::{Arc, Mutex};

use common::{setup, Collect, Shared};
use crosure_agent::{run_agent, AgentConfig, AgentEvent, Provider, ScriptedProvider};
use crosure_recorder::{ActorKind, StepKind};
use serde_json::json;

fn chain(p: ScriptedProvider) -> (Arc<ScriptedProvider>, Vec<Box<dyn Provider>>) {
    let p = Arc::new(p);
    (p.clone(), vec![Box::new(Shared(p))])
}

#[test]
fn agent_reverses_the_crackme_and_every_step_is_recorded() -> Result<(), Box<dyn std::error::Error>>
{
    let (exec, sid) = setup()?;
    let (_, providers) = chain(ScriptedProvider::crackme_demo());
    let sink = Collect::default();
    let report = run_agent(
        &providers,
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
    assert!(
        agent
            .iter()
            .all(|s| s.actor.model.as_deref() == Some("demo:scripted-demo")),
        "provider:model recorded"
    );
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
fn anthropic_requests_follow_the_api_contract() -> Result<(), Box<dyn std::error::Error>> {
    let (exec, _) = setup()?;
    let (p, providers) = chain(ScriptedProvider::crackme_demo());
    run_agent(
        &providers,
        &exec,
        &Collect::default(),
        "Reverse this binary.",
        &AgentConfig::default(),
    )?;
    let reqs = p.requests.lock().map_err(|_| "lock")?;
    let msgs = reqs[1]["messages"].as_array().ok_or("messages")?;
    assert_eq!(
        msgs[1]["content"][0]["type"], "thinking",
        "own turns echoed verbatim"
    );
    assert_eq!(msgs[2]["content"][0]["type"], "tool_result");
    assert_eq!(msgs[2]["content"][0]["tool_use_id"], "t1");
    assert_eq!(reqs[1]["output_config"]["effort"], "high");
    assert!(reqs[1].get("temperature").is_none());
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
fn a_refusal_moves_the_run_to_the_next_provider() -> Result<(), Box<dyn std::error::Error>> {
    let (exec, sid) = setup()?;
    let a = Arc::new(ScriptedProvider::new(
        "first",
        vec![
            ScriptedProvider::tool("a1", "list_imports", json!({"why": "start"}), "thinking A"),
            ScriptedProvider::refusal("cyber"),
        ],
    ));
    let b = Arc::new(ScriptedProvider::new(
        "second",
        vec![
            ScriptedProvider::tool(
                "b1",
                "binary_info",
                json!({"why": "continue"}),
                "thinking B",
            ),
            ScriptedProvider::done("report from B"),
        ],
    ));
    let providers: Vec<Box<dyn Provider>> =
        vec![Box::new(Shared(a.clone())), Box::new(Shared(b.clone()))];
    let sink = Collect::default();
    let report = run_agent(&providers, &exec, &sink, "go", &AgentConfig::default())?;
    assert_eq!(report, "report from B");
    let events = sink.0.lock().map_err(|_| "lock")?;
    assert!(events.iter().any(|e| matches!(e, AgentEvent::Refusal { provider, category: Some(c), .. } if provider == "first" && c == "cyber")));
    assert!(events.iter().any(|e| matches!(e, AgentEvent::Switched { from, to } if from == "first:scripted-demo" && to == "second:scripted-demo")));
    // B sees A's turn rebuilt from its tool call (A's thinking is not forwarded).
    let breq = b.requests.lock().map_err(|_| "lock")?;
    let a_turn = &breq[0]["messages"][1]["content"];
    assert_eq!(a_turn[0]["type"], "tool_use");
    assert!(a_turn
        .as_array()
        .ok_or("arr")?
        .iter()
        .all(|b| b["type"] != "thinking"));
    let store = exec.store.lock().map_err(|_| "lock")?;
    let by: Vec<String> = store
        .steps(&sid)?
        .iter()
        .filter_map(|s| s.actor.model.clone())
        .collect();
    assert_eq!(by, vec!["first:scripted-demo", "second:scripted-demo"]);
    Ok(())
}

#[test]
fn bad_input_and_stop_are_handled() -> Result<(), Box<dyn std::error::Error>> {
    let (exec, sid) = setup()?;
    let (p, providers) = chain(ScriptedProvider::new(
        "x",
        vec![
            ScriptedProvider::tool("b1", "disassemble", json!({"why": "no target"}), "oops"),
            ScriptedProvider::tool(
                "b2",
                "disassemble",
                json!({"target": "nope", "why": "missing fn"}),
                "again",
            ),
            ScriptedProvider::done("done"),
        ],
    ));
    let sink = Collect::default();
    run_agent(&providers, &exec, &sink, "go", &AgentConfig::default())?;
    let errors = sink
        .0
        .lock()
        .map_err(|_| "lock")?
        .iter()
        .filter(|e| matches!(e, AgentEvent::ToolCall { error: Some(_), .. }))
        .count();
    assert_eq!(errors, 2);
    assert_eq!(
        p.requests.lock().map_err(|_| "lock")?[1]["messages"][2]["content"][0]["is_error"],
        true
    );
    assert_eq!(
        exec.store.lock().map_err(|_| "lock")?.steps(&sid)?.len(),
        1,
        "failed calls record nothing"
    );

    let (_, providers) = chain(ScriptedProvider::crackme_demo());
    let stopped = Collect(Mutex::new(Vec::new()), true);
    run_agent(&providers, &exec, &stopped, "go", &AgentConfig::default())?;
    assert!(matches!(
        stopped.0.lock().map_err(|_| "lock")?.last(),
        Some(AgentEvent::Stopped)
    ));
    Ok(())
}
