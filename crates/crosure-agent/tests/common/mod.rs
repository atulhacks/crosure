#![allow(dead_code)]
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use crosure_agent::{
    AgentError, AgentEvent, Provider, ScriptedProvider, SessionExecutor, Sink, Transcript, Turn,
};
use crosure_recorder::Store;
use crosure_session::Workspace;

#[derive(Default)]
pub struct Collect(pub Mutex<Vec<AgentEvent>>, pub bool);

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

/// Lets a test keep a handle on a scripted provider after handing it to the loop.
pub struct Shared(pub Arc<ScriptedProvider>);

impl Provider for Shared {
    fn id(&self) -> &str {
        self.0.id()
    }
    fn model(&self) -> &str {
        self.0.model()
    }
    fn next(&self, t: &Transcript) -> Result<Turn, AgentError> {
        self.0.next(t)
    }
}

pub fn setup() -> Result<(SessionExecutor, String), Box<dyn std::error::Error>> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../crosure-engine/tests/fixtures/crackme-x64");
    let store = Store::open_in_memory()?;
    let (ws, _) = Workspace::open(&store, &path, None)?;
    let sid = ws.session.id.clone();
    Ok((
        SessionExecutor {
            store: Arc::new(Mutex::new(store)),
            workspace: Arc::new(Mutex::new(Some(ws))),
        },
        sid,
    ))
}
