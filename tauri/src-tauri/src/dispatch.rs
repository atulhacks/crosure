use serde::de::DeserializeOwned;
use serde_json::{to_value, Value};

use crate::core::{self, Res};
use crate::AppState;

fn arg<T: DeserializeOwned>(args: &Value, key: &str) -> Res<T> {
    serde_json::from_value(args.get(key).cloned().unwrap_or(Value::Null))
        .map_err(|e| format!("argument `{key}`: {e}"))
}

fn json<T: serde::Serialize>(r: Res<T>) -> Res<Value> {
    r.and_then(|v| to_value(v).map_err(|e| e.to_string()))
}

/// Runs a command by name with Tauri-style (camelCase) JSON arguments.
/// Used by the dev bridge; the same names as the Tauri commands.
pub fn dispatch(state: &AppState, cmd: &str, args: &Value) -> Res<Value> {
    match cmd {
        "open_binary" => json(core::open_binary(state, arg(args, "path")?)),
        "list_sessions" => json(core::list_sessions(state)),
        "resume_session" => json(core::resume_session(state, arg(args, "id")?)),
        "functions" => json(core::functions(state)),
        "run_op" => json(core::run_op(state, arg(args, "op")?, arg(args, "parent")?)),
        "run_console" => json(core::run_console(
            state,
            arg(args, "line")?,
            arg(args, "parent")?,
        )),
        "console_help" => json(Ok(core::console_help())),
        "annotate" => json(core::annotate(
            state,
            arg(args, "stepId")?,
            arg(args, "chip")?,
            arg(args, "note")?,
            arg::<Option<Vec<String>>>(args, "tags")?.unwrap_or_default(),
        )),
        "graph" => json(core::graph(state, arg(args, "upto")?)),
        "verify" => json(core::verify(state)),
        "export_session" => json(core::export_session(state)),
        "step_outcome" => json(core::step_outcome(state, arg(args, "stepId")?)),
        "agent_status" => json(Ok(core::agent_status(state))),
        "agent_settings" => json(Ok(core::agent_settings(state))),
        "agent_save_settings" => json(core::agent_save_settings(state, arg(args, "settings")?)),
        "agent_list_models" => json(core::agent_list_models(state, arg(args, "provider")?)),
        "agent_start" => json(core::agent_start(state, arg(args, "prompt")?)),
        "agent_events" => json(Ok(core::agent_events(
            state,
            arg::<Option<usize>>(args, "since")?.unwrap_or(0),
        ))),
        "agent_stop" => {
            core::agent_stop(state);
            Ok(Value::Null)
        }
        other => Err(format!("unknown command `{other}`")),
    }
}
