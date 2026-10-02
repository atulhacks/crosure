use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use crosure_agent::SessionExecutor;
use crosure_mcp::Server;
use crosure_recorder::{ActorKind, Store};
use crosure_session::Workspace;
use serde_json::json;

#[test]
fn an_mcp_client_session_is_recorded() -> Result<(), Box<dyn std::error::Error>> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../crosure-engine/tests/fixtures/crackme-x64");
    let store = Store::open_in_memory()?;
    let (ws, _) = Workspace::open(&store, &path, None)?;
    let sid = ws.session.id.clone();
    let store = Arc::new(Mutex::new(store));
    let exec = SessionExecutor {
        store: store.clone(),
        workspace: Arc::new(Mutex::new(Some(ws))),
    };
    let mut s = Server::new(exec);

    let init = s.handle(&json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18","clientInfo":{"name":"claude-code","version":"2"}}})).ok_or("no reply")?;
    assert_eq!(init["result"]["serverInfo"]["name"], "crosure");
    assert!(init["result"]["instructions"]
        .as_str()
        .is_some_and(|i| i.contains("x86_64")));
    assert!(s
        .handle(&json!({"jsonrpc":"2.0","method":"notifications/initialized"}))
        .is_none());

    let list = s
        .handle(&json!({"jsonrpc":"2.0","id":2,"method":"tools/list"}))
        .ok_or("no reply")?;
    let tools = list["result"]["tools"].as_array().ok_or("tools")?;
    assert!(tools
        .iter()
        .any(|t| t["name"] == "disassemble" && t["inputSchema"]["type"] == "object"));

    let call = s.handle(&json!({"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"disassemble","arguments":{"target":"check_password","why":"read the checker"}}})).ok_or("no reply")?;
    assert_eq!(call["result"]["isError"], false);
    assert!(call["result"]["content"][0]["text"]
        .as_str()
        .is_some_and(|t| t.contains("strcmp@plt")));

    let bad = s.handle(&json!({"jsonrpc":"2.0","id":4,"method":"tools/call","params":{"name":"disassemble","arguments":{}}})).ok_or("no reply")?;
    assert_eq!(bad["result"]["isError"], true);
    let unknown = s
        .handle(&json!({"jsonrpc":"2.0","id":5,"method":"resources/list"}))
        .ok_or("no reply")?;
    assert_eq!(unknown["error"]["code"], -32601);

    // the call is on the graph as an agent step tagged with the client, with its why
    let steps = store.lock().map_err(|_| "lock")?.steps(&sid)?;
    let agent: Vec<_> = steps
        .iter()
        .filter(|s| s.actor.kind == ActorKind::Agent)
        .collect();
    assert_eq!(agent.len(), 1, "only the valid call is recorded");
    assert_eq!(agent[0].actor.model.as_deref(), Some("mcp:claude-code"));
    assert_eq!(
        agent[0].intent.as_ref().and_then(|i| i.note.as_deref()),
        Some("read the checker")
    );
    assert!(store.lock().map_err(|_| "lock")?.verify(&sid)?.ok);
    Ok(())
}
