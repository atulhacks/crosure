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
pub fn console_help() -> String {
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

/// Whether decompilation is available.
#[tauri::command]
pub fn decompiler_status() -> crosure_engine::DecompilerStatus {
    core::decompiler_status()
}

/// Exports every verified session as a training dataset.
#[tauri::command]
pub fn export_dataset(state: State<'_, AppState>) -> Res<core::DatasetExport> {
    core::export_dataset(&state)
}

/// Revisits an earlier step's result.
#[tauri::command]
pub fn step_outcome(state: State<'_, AppState>, step_id: String) -> Res<Outcome> {
    core::step_outcome(&state, step_id)
}

/// Agent dock status.
#[tauri::command]
pub fn agent_status(state: State<'_, AppState>) -> crate::agent::AgentStatus {
    core::agent_status(&state)
}

/// Provider settings.
#[tauri::command]
pub fn agent_settings(state: State<'_, AppState>) -> crate::agent::SettingsView {
    core::agent_settings(&state)
}

/// Saves provider settings.
#[tauri::command]
pub fn agent_save_settings(
    state: State<'_, AppState>,
    settings: crosure_agent::AgentSettings,
) -> Res<crate::agent::SettingsView> {
    core::agent_save_settings(&state, settings)
}

/// Lists a provider's models.
#[tauri::command]
pub fn agent_list_models(
    state: State<'_, AppState>,
    provider: crosure_agent::ProviderConfig,
) -> Res<Vec<crosure_agent::ModelInfo>> {
    core::agent_list_models(&state, provider)
}

/// Sends a prompt to the agent.
#[tauri::command]
pub fn agent_start(
    state: State<'_, AppState>,
    prompt: String,
    profile: Option<crosure_agent::Profile>,
) -> Res<()> {
    core::agent_start(&state, prompt, profile)
}

/// Agent threads of the open session.
#[tauri::command]
pub fn agent_threads(state: State<'_, AppState>) -> Res<crate::agent::ThreadList> {
    core::agent_threads(&state)
}

/// Shows a thread (or a fresh one).
#[tauri::command]
pub fn agent_open_thread(state: State<'_, AppState>, id: Option<String>) -> Res<()> {
    core::agent_open_thread(&state, id)
}

/// Answers a pending approval.
#[tauri::command]
pub fn agent_decide(state: State<'_, AppState>, id: String, allow: bool) -> Res<()> {
    core::agent_decide(&state, id, allow)
}

/// Switches the active provider.
#[tauri::command]
pub fn agent_set_active(state: State<'_, AppState>, id: String) -> Res<crate::agent::AgentStatus> {
    core::agent_set_active(&state, id)
}

/// Agent events since a cursor.
#[tauri::command]
pub fn agent_events(state: State<'_, AppState>, since: usize) -> crate::agent::EventPage {
    core::agent_events(&state, since)
}

/// Stops the agent.
#[tauri::command]
pub fn agent_stop(state: State<'_, AppState>) {
    core::agent_stop(&state)
}
