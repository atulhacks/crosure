//! Command logic shared by the Tauri app and the dev bridge.

use std::path::Path;

use crosure_engine::{BinaryInfo, FunctionInfo};
use crosure_graph::InvestigationGraph;
use crosure_recorder::{Intent, ParentRef, Session, Step, VerifyReport};
use crosure_session::{parse_command, Op, Origin, Outcome, Workspace, CONSOLE_HELP};
use serde::Serialize;
use serde_json::json;

use crate::AppState;

pub type Res<T> = Result<T, String>;

fn err(e: impl std::fmt::Display) -> String {
    e.to_string()
}

/// What the UI needs right after opening or resuming a session.
#[derive(Serialize)]
pub struct Opened {
    pub session: Session,
    pub info: BinaryInfo,
    pub functions: Vec<FunctionInfo>,
}

fn opened(ws: &Workspace) -> Res<Opened> {
    Ok(Opened {
        session: ws.session.clone(),
        info: ws.engine.info().map_err(err)?,
        functions: ws.functions().map_err(err)?,
    })
}

/// Opens a binary and starts a new recorded session.
pub fn open_binary(state: &AppState, path: String) -> Res<Opened> {
    let store = state.store.lock().map_err(err)?;
    let (ws, _) = Workspace::open(&store, Path::new(&path), None).map_err(err)?;
    let out = opened(&ws)?;
    *state.workspace.lock().map_err(err)? = Some(ws);
    Ok(out)
}

/// Every recorded session, newest first.
pub fn list_sessions(state: &AppState) -> Res<Vec<Session>> {
    state.store.lock().map_err(err)?.sessions().map_err(err)
}

/// Reopens a recorded session (its binary must still exist).
pub fn resume_session(state: &AppState, id: String) -> Res<Opened> {
    let store = state.store.lock().map_err(err)?;
    let ws = Workspace::resume(&store, &id).map_err(err)?;
    let out = opened(&ws)?;
    *state.workspace.lock().map_err(err)? = Some(ws);
    Ok(out)
}

/// Functions of the open binary, with renames applied.
pub fn functions(state: &AppState) -> Res<Vec<FunctionInfo>> {
    let (_store, ws) = state.both()?;
    ws.as_ref()
        .ok_or("no binary is open")?
        .functions()
        .map_err(err)
}

/// Runs an op from the UI and records it.
pub fn run_op(state: &AppState, op: Op, parent: Option<ParentRef>) -> Res<Outcome> {
    let (store, mut ws) = state.both()?;
    ws.as_mut()
        .ok_or("no binary is open")?
        .run(&store, op, parent, Origin::Ui)
        .map_err(err)
}

/// Parses and runs a console line and records it.
pub fn run_console(state: &AppState, line: String, parent: Option<ParentRef>) -> Res<Outcome> {
    let op = parse_command(&line).map_err(err)?;
    let (store, mut ws) = state.both()?;
    ws.as_mut()
        .ok_or("no binary is open")?
        .run(&store, op, parent, Origin::Console)
        .map_err(err)
}

/// Console help text.
pub fn console_help() -> &'static str {
    CONSOLE_HELP
}

/// Adds intent/tags to an earlier step (recorded as a new step).
pub fn annotate(
    state: &AppState,
    step_id: String,
    chip: Option<String>,
    note: Option<String>,
    tags: Vec<String>,
) -> Res<Step> {
    let (store, ws) = state.both()?;
    let intent = (chip.is_some() || note.is_some()).then_some(Intent { chip, note });
    ws.as_ref()
        .ok_or("no binary is open")?
        .annotate(&store, &step_id, intent, tags)
        .map_err(err)
}

/// The current session's investigation graph, optionally as of step `upto`.
pub fn graph(state: &AppState, upto: Option<u64>) -> Res<InvestigationGraph> {
    let (store, ws) = state.both()?;
    let steps = store
        .steps(&ws.as_ref().ok_or("no binary is open")?.session.id)
        .map_err(err)?;
    Ok(match upto {
        Some(n) => crosure_graph::replay(&steps, n),
        None => crosure_graph::build(&steps),
    })
}

/// Re-derives the current session's hash chain.
pub fn verify(state: &AppState) -> Res<VerifyReport> {
    let (store, ws) = state.both()?;
    store
        .verify(&ws.as_ref().ok_or("no binary is open")?.session.id)
        .map_err(err)
}

/// Writes the session (steps, graph, verification) to a JSON file and returns its path.
pub fn export_session(state: &AppState) -> Res<String> {
    let (store, ws) = state.both()?;
    let session = ws.as_ref().ok_or("no binary is open")?.session.clone();
    let steps = store.steps(&session.id).map_err(err)?;
    let doc = json!({
        "format": "crosure.session.v1",
        "session": session,
        "verify": store.verify(&session.id).map_err(err)?,
        "graph": crosure_graph::build(&steps),
        "steps": steps,
    });
    let dir = state.home.join("exports");
    std::fs::create_dir_all(&dir).map_err(err)?;
    let path = dir.join(format!("{}-{}.json", session.name, session.id));
    std::fs::write(&path, serde_json::to_vec_pretty(&doc).map_err(err)?).map_err(err)?;
    Ok(path.display().to_string())
}

/// Whether rizin + rz-ghidra are available for decompilation.
pub fn decompiler_status() -> crosure_engine::DecompilerStatus {
    crosure_engine::decompiler_status()
}

/// Where a dataset export went and what it holds.
#[derive(Serialize)]
pub struct DatasetExport {
    pub dir: String,
    pub manifest: crosure_dataset::Manifest,
}

/// Writes every verified session as a training dataset (trajectories, SFT,
/// DPO) to `home/datasets/<time>/`.
pub fn export_dataset(state: &AppState) -> Res<DatasetExport> {
    let store = state.store.lock().map_err(|_| "store lock poisoned")?;
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(err)?
        .as_secs();
    let dir = state.home.join("datasets").join(stamp.to_string());
    let manifest = crosure_dataset::export(&store, &dir, &Default::default()).map_err(err)?;
    Ok(DatasetExport {
        dir: dir.display().to_string(),
        manifest,
    })
}

/// An earlier step with its stored result, for revisiting it (records nothing).
pub fn step_outcome(state: &AppState, step_id: String) -> Res<Outcome> {
    let (store, ws) = state.both()?;
    let session = &ws.as_ref().ok_or("no binary is open")?.session;
    let step = store
        .steps(&session.id)
        .map_err(err)?
        .into_iter()
        .find(|s| s.id == step_id)
        .ok_or_else(|| format!("unknown step {step_id}"))?;
    let result = match &step.observation.blob {
        Some(key) => match store.get_blob(key).map_err(err)? {
            Some(bytes) => serde_json::from_slice(&bytes).map_err(err)?,
            None => serde_json::Value::Null,
        },
        None => serde_json::Value::Null,
    };
    Ok(Outcome { step, result })
}

/// Agent dock status (never a key).
pub fn agent_status(state: &AppState) -> crate::agent::AgentStatus {
    state.agent.status()
}

/// Provider settings for the settings dialog (keys redacted).
pub fn agent_settings(state: &AppState) -> crate::agent::SettingsView {
    state.agent.settings_view()
}

/// Saves provider settings.
pub fn agent_save_settings(
    state: &AppState,
    settings: crosure_agent::AgentSettings,
) -> Res<crate::agent::SettingsView> {
    state.agent.save(settings)
}

/// Lists a provider's models (a connection test too).
pub fn agent_list_models(
    state: &AppState,
    provider: crosure_agent::ProviderConfig,
) -> Res<Vec<String>> {
    state.agent.models(provider)
}

/// Sends a prompt to the shown thread (or a new one).
pub fn agent_start(
    state: &AppState,
    prompt: String,
    profile: Option<crosure_agent::Profile>,
) -> Res<()> {
    let prompt = prompt.trim().to_string();
    if prompt.is_empty() {
        return Err("Tell the agent what to do.".into());
    }
    crate::agent::start(state, prompt, profile)
}

fn session_id(state: &AppState) -> Res<String> {
    let ws = state.workspace.lock().map_err(err)?;
    Ok(ws.as_ref().ok_or("no binary is open")?.session.id.clone())
}

/// Agent threads of the open session.
pub fn agent_threads(state: &AppState) -> Res<crate::agent::ThreadList> {
    Ok(state.agent.threads(&session_id(state)?))
}

/// Shows a saved thread, or a fresh one when `id` is `None`.
pub fn agent_open_thread(state: &AppState, id: Option<String>) -> Res<()> {
    state.agent.open_thread(&session_id(state)?, id.as_deref())
}

/// Answers a pending tool-call approval.
pub fn agent_decide(state: &AppState, id: String, allow: bool) -> Res<()> {
    state.agent.decide(&id, allow)
}

/// Switches the provider runs start on.
pub fn agent_set_active(state: &AppState, id: String) -> Res<crate::agent::AgentStatus> {
    state.agent.set_active(&id)
}

/// Agent events since `since`.
pub fn agent_events(state: &AppState, since: usize) -> crate::agent::EventPage {
    state.agent.page(since)
}

/// Stops the running agent after its current call.
pub fn agent_stop(state: &AppState) {
    state.agent.request_stop();
}
