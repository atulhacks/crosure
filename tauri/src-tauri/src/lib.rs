//! Crosure desktop app: Tauri commands over the session, recorder and graph crates.

mod agent;
mod commands;
pub mod core;
mod dispatch;
mod state;

pub use dispatch::dispatch;
pub use state::{crosure_home, AppState};

/// Starts the desktop app.
pub fn run() {
    let state = match AppState::open(crosure_home()) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("crosure: cannot open store: {e}");
            std::process::exit(1);
        }
    };
    let result = tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(state)
        .invoke_handler(tauri::generate_handler![
            commands::open_binary,
            commands::list_sessions,
            commands::resume_session,
            commands::functions,
            commands::run_op,
            commands::run_console,
            commands::console_help,
            commands::annotate,
            commands::graph,
            commands::verify,
            commands::export_session,
            commands::step_outcome,
            commands::agent_status,
            commands::agent_settings,
            commands::agent_save_settings,
            commands::agent_list_models,
            commands::agent_start,
            commands::agent_threads,
            commands::agent_open_thread,
            commands::agent_decide,
            commands::agent_set_active,
            commands::agent_events,
            commands::agent_stop,
        ])
        .run(tauri::generate_context!());
    if let Err(e) = result {
        eprintln!("crosure: {e}");
        std::process::exit(1);
    }
}
