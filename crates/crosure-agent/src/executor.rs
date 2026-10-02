use std::sync::{Arc, Mutex};

use crosure_recorder::{Intent, Store};
use crosure_session::{Author, Origin, Workspace};

use crate::{Executed, Executor, ToolCall};

/// Executes agent tool calls on a shared store and workspace, recording each
/// as an agent step. Locks are held only for the duration of one call, so the
/// UI stays usable while the agent runs.
pub struct SessionExecutor {
    pub store: Arc<Mutex<Store>>,
    pub workspace: Arc<Mutex<Option<Workspace>>>,
    pub model: String,
}

impl Executor for SessionExecutor {
    fn execute(&self, call: &ToolCall) -> Result<Executed, String> {
        let store = self.store.lock().map_err(|_| "store lock poisoned")?;
        let mut ws = self
            .workspace
            .lock()
            .map_err(|_| "workspace lock poisoned")?;
        let ws = ws.as_mut().ok_or("no binary is open")?;
        let author = Author {
            model: Some(self.model.clone()),
            intent: (!call.why.is_empty()).then(|| Intent {
                chip: None,
                note: Some(call.why.clone()),
            }),
        };
        let out = ws
            .run_as(&store, call.op.clone(), None, Origin::Agent, author)
            .map_err(|e| e.to_string())?;
        let kind = serde_json::to_value(out.step.kind)
            .ok()
            .and_then(|v| v.as_str().map(str::to_string))
            .unwrap_or_default();
        Ok(Executed {
            step_id: out.step.id.clone(),
            kind,
            command: out.step.command.clone().unwrap_or_default(),
            summary: out.step.observation.summary.clone(),
            result: out.result,
        })
    }

    fn context(&self) -> String {
        let Ok(ws) = self.workspace.lock() else {
            return String::new();
        };
        let Some(ws) = ws.as_ref() else {
            return "No binary is open.".into();
        };
        match ws.engine.info() {
            Ok(i) => format!(
                "Binary: {} ({} {} {}-bit, entry {:#x}, {} functions{}, sha256 {}).",
                i.path,
                i.format.to_uppercase(),
                i.arch,
                i.bits,
                i.entry,
                ws.functions().map(|f| f.len()).unwrap_or(0),
                if i.stripped { ", stripped" } else { "" },
                i.sha256.trim_start_matches("sha256:")
            ),
            Err(e) => format!("Binary info unavailable: {e}"),
        }
    }
}
