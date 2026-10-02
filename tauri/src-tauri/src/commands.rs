use crosure_engine::FunctionInfo;
use crosure_graph::InvestigationGraph;
use crosure_recorder::{ParentRef, Session, Step, VerifyReport};
use crosure_session::{Op, Outcome};
use tauri::State;

use crate::core::{self, Opened, Res};
use crate::AppState;

/// Opens a binary and starts a new recorded session.
#[tauri::command]
pub fn open_binary(state: State<'_, AppState>, path: String) -> Res<Opened> {
    core::open_binary(&state, path)
}

/// Every recorded session, newest first.
#[tauri::command]
pub fn list_sessions(state: State<'_, AppState>) -> Res<Vec<Session>> {
    core::list_sessions(&state)
}

/// Reopens a recorded session.
#[tauri::command]
pub fn resume_session(state: State<'_, AppState>, id: String) -> Res<Opened> {
    core::resume_session(&state, id)
}

/// Functions of the open binary.
#[tauri::command]
pub fn functions(state: State<'_, AppState>) -> Res<Vec<FunctionInfo>> {
    core::functions(&state)
}

/// Runs and records a UI op.
#[tauri::command]
pub fn run_op(state: State<'_, AppState>, op: Op, parent: Option<ParentRef>) -> Res<Outcome> {
    core::run_op(&state, op, parent)
}

/// Runs and records a console line.
#[tauri::command]
pub fn run_console(
    state: State<'_, AppState>,
    line: String,
    parent: Option<ParentRef>,
) -> Res<Outcome> {
    core::run_console(&state, line, parent)
}

/// Console help text.
#[tauri::command]
pub fn console_help() -> &'static str {
    core::console_help()
}

/// Adds intent/tags to a step.
#[tauri::command]
pub fn annotate(
    state: State<'_, AppState>,
    step_id: String,
    chip: Option<String>,
    note: Option<String>,
    tags: Vec<String>,
) -> Res<Step> {
    core::annotate(&state, step_id, chip, note, tags)
}

/// The investigation graph.
#[tauri::command]
pub fn graph(state: State<'_, AppState>, upto: Option<u64>) -> Res<InvestigationGraph> {
    core::graph(&state, upto)
}

/// Verifies the hash chain.
#[tauri::command]
pub fn verify(state: State<'_, AppState>) -> Res<VerifyReport> {
    core::verify(&state)
}

/// Exports the session as JSON.
#[tauri::command]
pub fn export_session(state: State<'_, AppState>) -> Res<String> {
    core::export_session(&state)
}

/// Revisits an earlier step's result.
#[tauri::command]
pub fn step_outcome(state: State<'_, AppState>, step_id: String) -> Res<Outcome> {
    core::step_outcome(&state, step_id)
}
