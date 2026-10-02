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
