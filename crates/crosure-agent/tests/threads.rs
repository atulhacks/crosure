mod common;

use std::sync::{Arc, Mutex};

use common::{setup, Collect, Shared};
use crosure_agent::{
    run_agent_turn, AgentConfig, AgentEvent, ApprovalRequest, Permissions, Profile, Provider,
    ScriptedProvider, Sink, Transcript,
};
use crosure_recorder::StepKind;
use serde_json::json;

fn one(p: &Arc<ScriptedProvider>) -> Vec<Box<dyn Provider>> {
    vec![Box::new(Shared(p.clone()))]
}

#[test]
fn follow_ups_continue_the_same_thread() -> Result<(), Box<dyn std::error::Error>> {
    let (exec, _) = setup()?;
    let p = Arc::new(ScriptedProvider::new(
        "x",
        vec![
            ScriptedProvider::tool("a", "list_imports", json!({"why": "start"}), "t"),
            ScriptedProvider::done("first answer"),
            ScriptedProvider::done("second answer"),
        ],
    ));
    let mut t = Transcript {
        instructions: "Map behaviour to MITRE ATT&CK.".into(),
        ..Default::default()
    };
    let cfg = AgentConfig::default();
    assert_eq!(
        run_agent_turn(
            &one(&p),
            &exec,
            &Collect::default(),
            &mut t,
            "What does it import?",
            &cfg
        )?,
        "first answer"
    );
    assert_eq!(
        run_agent_turn(
            &one(&p),
            &exec,
            &Collect::default(),
            &mut t,
            "And why?",
            &cfg
        )?,
        "second answer"
    );
    let reqs = p.requests.lock().map_err(|_| "lock")?;
    let msgs = reqs[2]["messages"].as_array().ok_or("messages")?;
    assert_eq!(
        msgs.len(),
        5,
        "context+task, tool call, results, answer, follow-up"
    );
    assert_eq!(msgs[4]["content"], "And why?");
    assert!(
        msgs[0]["content"]
            .as_str()
            .is_some_and(|c| c.starts_with("Binary:")),
        "context only on the first message"
    );
    assert!(reqs[0]["system"]
        .as_str()
        .is_some_and(|s| s.ends_with("Map behaviour to MITRE ATT&CK.")));
    Ok(())
}

#[test]
fn profiles_limit_tools() -> Result<(), Box<dyn std::error::Error>> {
    let (exec, sid) = setup()?;
    let p = Arc::new(ScriptedProvider::new(
        "x",
        vec![
            ScriptedProvider::tool(
                "r",
                "rename_function",
                json!({"target": "decode", "new_name": "x", "why": "w"}),
                "t",
            ),
            ScriptedProvider::done("ok"),
        ],
    ));
    let mut t = Transcript {
        profile: Profile::ReadOnly,
        ..Default::default()
    };
    let sink = Collect::default();
    run_agent_turn(
        &one(&p),
        &exec,
        &sink,
        &mut t,
        "go",
        &AgentConfig::default(),
    )?;
    let reqs = p.requests.lock().map_err(|_| "lock")?;
    assert!(reqs[0]["tools"]
        .as_array()
        .ok_or("tools")?
        .iter()
        .all(|t| t["name"] != "rename_function"));
    assert!(sink.0.lock().map_err(|_| "lock")?.iter().any(
        |e| matches!(e, AgentEvent::ToolCall { error: Some(e), .. } if e.contains("not available"))
    ));
    assert!(exec
        .store
        .lock()
        .map_err(|_| "lock")?
        .steps(&sid)?
        .iter()
        .all(|s| s.kind != StepKind::Rename));

    let ask = Arc::new(ScriptedProvider::new(
        "x",
        vec![ScriptedProvider::done("answer")],
    ));
    let mut t = Transcript {
        profile: Profile::Ask,
        ..Default::default()
    };
    run_agent_turn(
        &one(&ask),
        &exec,
        &Collect::default(),
        &mut t,
        "explain",
        &AgentConfig::default(),
    )?;
    let reqs = ask.requests.lock().map_err(|_| "lock")?;
    assert!(reqs[0].get("tools").is_none() && reqs[0].get("tool_choice").is_none());
    Ok(())
}

struct Decide(Collect, bool, Mutex<Vec<String>>);

impl Sink for Decide {
    fn emit(&self, e: AgentEvent) {
        self.0.emit(e);
    }
    fn should_stop(&self) -> bool {
        false
    }
    fn approve(&self, r: &ApprovalRequest) -> bool {
        if let Ok(mut v) = self.2.lock() {
            v.push(r.command.clone());
        }
        self.1
    }
}

#[test]
fn confirm_permission_asks_and_respects_the_answer() -> Result<(), Box<dyn std::error::Error>> {
    for allow in [false, true] {
        let (exec, sid) = setup()?;
        let p = Arc::new(ScriptedProvider::new(
            "x",
            vec![
                ScriptedProvider::tool(
                    "r",
                    "rename_function",
                    json!({"target": "decode", "new_name": "xor_decode", "why": "w"}),
                    "t",
                ),
                ScriptedProvider::done("ok"),
            ],
        ));
        let sink = Decide(Collect::default(), allow, Mutex::new(Vec::new()));
        let cfg = AgentConfig {
            permissions: Permissions::confirm_changes(),
            ..Default::default()
        };
        run_agent_turn(
            &one(&p),
            &exec,
            &sink,
            &mut Transcript::default(),
            "go",
            &cfg,
        )?;
        assert_eq!(
            *sink.2.lock().map_err(|_| "lock")?,
            vec!["ren decode xor_decode".to_string()]
        );
        let renamed = exec
            .store
            .lock()
            .map_err(|_| "lock")?
            .steps(&sid)?
            .iter()
            .any(|s| s.kind == StepKind::Rename);
        assert_eq!(renamed, allow);
        let events = sink.0 .0.lock().map_err(|_| "lock")?;
        assert!(events.iter().any(
            |e| matches!(e, AgentEvent::ApprovalResolved { allowed, .. } if *allowed == allow)
        ));
    }
    Ok(())
}

#[test]
fn a_new_prompt_after_an_interrupted_run_keeps_roles_alternating() {
    let mut t = Transcript::default();
    t.push_user("task");
    t.entries.push(crosure_agent::Entry::Results {
        results: vec![],
        note: None,
    });
    t.push_user("continue");
    assert!(
        matches!(t.entries.last(), Some(crosure_agent::Entry::Results { note: Some(n), .. }) if n == "continue")
    );
}
