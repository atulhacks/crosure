//! Starting a run: pick or create the thread, attach `@` mentions, record the
//! request, then run the agent on a background thread.

use std::sync::atomic::Ordering;

use crosure_agent::{run_agent_turn, AgentConfig, AgentEvent, Profile, SessionExecutor, Sink};

use super::mention;
use super::threads::Thread;
use crate::AppState;

/// Sends `prompt` to the shown thread (or a new one), optionally switching its profile.
pub fn start(state: &AppState, prompt: String, profile: Option<Profile>) -> Result<(), String> {
    let rt = state.agent.clone();
    if rt.running.swap(true, Ordering::SeqCst) {
        return Err("the agent is already running".into());
    }
    let prepared = prepare(state, &prompt, profile);
    let (chain, mut thread, full_prompt) = match prepared {
        Ok(p) => p,
        Err(e) => {
            rt.running.store(false, Ordering::SeqCst);
            return Err(e);
        }
    };
    let settings = rt.snapshot();
    thread.transcript.instructions = settings.instructions.clone();
    if let Ok(mut events) = rt.events.lock() {
        *events = thread.events.clone();
    }
    if let Ok(mut store) = rt.threads.lock() {
        store.current = Some(thread.id.clone());
    }
    rt.stop.store(false, Ordering::SeqCst);
    let exec = SessionExecutor {
        store: state.store.clone(),
        workspace: state.workspace.clone(),
    };
    let cfg = AgentConfig {
        permissions: settings.permissions,
        ..Default::default()
    };
    std::thread::spawn(move || {
        let result = run_agent_turn(
            &chain,
            &exec,
            rt.as_ref(),
            &mut thread.transcript,
            &full_prompt,
            &cfg,
        );
        if let Some(report) = result.as_ref().ok().filter(|r| !r.is_empty()) {
            // Attribute the answer to whichever provider finished the run.
            let by = rt
                .events
                .lock()
                .ok()
                .and_then(|e| {
                    e.iter().rev().find_map(|e| match e {
                        AgentEvent::Switched { to, .. } => Some(to.clone()),
                        _ => None,
                    })
                })
                .unwrap_or_else(|| chain[0].tag());
            let saved = exec.store.lock().ok().and_then(|store| {
                let ws = exec.workspace.lock().ok()?;
                ws.as_ref()?.record_report(&store, &by, report).ok()
            });
            if saved.is_none() {
                rt.emit(AgentEvent::Failed {
                    error: "could not record the report".into(),
                });
            }
        }
        thread.events = rt.events.lock().map(|e| e.clone()).unwrap_or_default();
        if let Ok(mut store) = rt.threads.lock() {
            if let Err(e) = store.put(thread) {
                rt.emit(AgentEvent::Failed {
                    error: format!("could not save the thread: {e}"),
                });
            }
        }
        rt.running.store(false, Ordering::SeqCst);
    });
    Ok(())
}

type Prepared = (Vec<Box<dyn crosure_agent::Provider>>, Thread, String);

fn prepare(state: &AppState, prompt: &str, profile: Option<Profile>) -> Result<Prepared, String> {
    let rt = &state.agent;
    let chain = rt.chain()?;
    let store = state.store.lock().map_err(|_| "store lock poisoned")?;
    let mut ws = state
        .workspace
        .lock()
        .map_err(|_| "workspace lock poisoned")?;
    let ws = ws.as_mut().ok_or("no binary is open")?;
    let session_id = ws.session.id.clone();
    let existing = {
        let mut threads = rt.threads.lock().map_err(|_| "threads lock poisoned")?;
        threads
            .current
            .clone()
            .and_then(|id| threads.get(&session_id, &id))
    };
    let mut thread = existing.unwrap_or_else(|| {
        Thread::new(
            &session_id,
            prompt,
            profile.unwrap_or(rt.snapshot().default_profile),
        )
    });
    if let Some(p) = profile {
        thread.profile = p;
        thread.transcript.profile = p;
    }
    ws.record_task(&store, prompt).map_err(|e| e.to_string())?;
    let context = mention::attach(&store, ws, prompt);
    Ok((chain, thread, format!("{prompt}{context}")))
}
