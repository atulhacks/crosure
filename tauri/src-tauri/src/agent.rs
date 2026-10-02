//! Agent runtime for the app: settings, the background run, and the event log the UI polls.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;

use crosure_agent::{
    run_agent, AgentConfig, AgentEvent, ClaudeHttp, Llm, ScriptedLlm, SessionExecutor, Sink,
    DEFAULT_MODEL,
};
use serde::{Deserialize, Serialize};

use crate::AppState;

#[derive(Clone, Default, Serialize, Deserialize)]
struct Saved {
    api_key: Option<String>,
    model: Option<String>,
}

/// What the UI shows about agent setup (never the key itself).
#[derive(Serialize)]
pub struct AgentStatus {
    pub configured: bool,
    /// `env`, `saved`, `demo`, or `none`.
    pub source: &'static str,
    pub model: String,
    pub running: bool,
}

/// A page of events since a cursor.
#[derive(Serialize)]
pub struct EventPage {
    pub events: Vec<AgentEvent>,
    pub next: usize,
    pub running: bool,
}

/// Agent settings, run state, and the event log for the current run.
pub struct AgentRuntime {
    path: PathBuf,
    saved: Mutex<Saved>,
    events: Mutex<Vec<AgentEvent>>,
    running: AtomicBool,
    stop: AtomicBool,
}

impl Sink for AgentRuntime {
    fn emit(&self, event: AgentEvent) {
        if let Ok(mut e) = self.events.lock() {
            e.push(event);
        }
    }
    fn should_stop(&self) -> bool {
        self.stop.load(Ordering::SeqCst)
    }
}

fn demo_mode() -> bool {
    std::env::var("CROSURE_AGENT_DEMO").is_ok_and(|v| v == "1")
}

impl AgentRuntime {
    /// Loads saved settings from `home/agent.json`.
    pub fn new(home: &Path) -> Self {
        let path = home.join("agent.json");
        let saved = std::fs::read(&path)
            .ok()
            .and_then(|b| serde_json::from_slice(&b).ok())
            .unwrap_or_default();
        Self {
            path,
            saved: Mutex::new(saved),
            events: Mutex::new(Vec::new()),
            running: AtomicBool::new(false),
            stop: AtomicBool::new(false),
        }
    }

    fn key(&self) -> (Option<String>, &'static str) {
        if demo_mode() {
            return (None, "demo");
        }
        if let Ok(k) = std::env::var("ANTHROPIC_API_KEY") {
            if !k.trim().is_empty() {
                return (Some(k), "env");
            }
        }
        match self.saved.lock().ok().and_then(|s| s.api_key.clone()) {
            Some(k) if !k.trim().is_empty() => (Some(k), "saved"),
            _ => (None, "none"),
        }
    }

    fn model(&self) -> String {
        self.saved
            .lock()
            .ok()
            .and_then(|s| s.model.clone())
            .unwrap_or_else(|| DEFAULT_MODEL.to_string())
    }

    /// Setup status for the UI.
    pub fn status(&self) -> AgentStatus {
        let (key, source) = self.key();
        AgentStatus {
            configured: key.is_some() || source == "demo",
            source,
            model: if source == "demo" {
                "scripted-demo".into()
            } else {
                self.model()
            },
            running: self.running.load(Ordering::SeqCst),
        }
    }

    /// Saves the API key (and optionally a model) to `agent.json`, readable only by the user.
    pub fn save(&self, api_key: Option<String>, model: Option<String>) -> Result<(), String> {
        let mut saved = self.saved.lock().map_err(|_| "settings lock poisoned")?;
        if api_key.is_some() {
            saved.api_key = api_key.filter(|k| !k.trim().is_empty());
        }
        if let Some(m) = model.filter(|m| !m.trim().is_empty()) {
            saved.model = Some(m);
        }
        std::fs::write(
            &self.path,
            serde_json::to_vec_pretty(&*saved).map_err(|e| e.to_string())?,
        )
        .map_err(|e| e.to_string())?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = std::fs::set_permissions(&self.path, std::fs::Permissions::from_mode(0o600));
        }
        Ok(())
    }

    /// Events since `since`.
    pub fn page(&self, since: usize) -> EventPage {
        let events = self
            .events
            .lock()
            .map(|e| {
                e.get(since..)
                    .map(<[AgentEvent]>::to_vec)
                    .unwrap_or_default()
            })
            .unwrap_or_default();
        EventPage {
            next: since + events.len(),
            events,
            running: self.running.load(Ordering::SeqCst),
        }
    }

    /// Asks the current run to stop after its in-flight call.
    pub fn request_stop(&self) {
        self.stop.store(true, Ordering::SeqCst);
    }

    fn llm(&self) -> Result<Box<dyn Llm>, String> {
        match self.key() {
            (_, "demo") => Ok(Box::new(ScriptedLlm::crackme_demo())),
            (Some(k), _) => {
                let base = std::env::var("ANTHROPIC_BASE_URL").ok();
                Ok(Box::new(
                    ClaudeHttp::new(&k, &self.model(), base.as_deref())
                        .map_err(|e| e.to_string())?,
                ))
            }
            (None, _) => Err("Add an Anthropic API key to use the agent.".into()),
        }
    }
}

/// Records the request, then runs the agent on a background thread.
pub fn start(state: &AppState, prompt: String) -> Result<(), String> {
    let rt = state.agent.clone();
    if rt.running.swap(true, Ordering::SeqCst) {
        return Err("the agent is already running".into());
    }
    let begin = || -> Result<Box<dyn Llm>, String> {
        let llm = rt.llm()?;
        let store = state.store.lock().map_err(|_| "store lock poisoned")?;
        let ws = state
            .workspace
            .lock()
            .map_err(|_| "workspace lock poisoned")?;
        ws.as_ref()
            .ok_or("no binary is open")?
            .record_task(&store, &prompt)
            .map_err(|e| e.to_string())?;
        Ok(llm)
    };
    let llm = match begin() {
        Ok(l) => l,
        Err(e) => {
            rt.running.store(false, Ordering::SeqCst);
            return Err(e);
        }
    };
    if let Ok(mut e) = rt.events.lock() {
        e.clear();
    }
    rt.stop.store(false, Ordering::SeqCst);
    let exec = SessionExecutor {
        store: state.store.clone(),
        workspace: state.workspace.clone(),
        model: llm.model().to_string(),
    };
    let model = llm.model().to_string();
    std::thread::spawn(move || {
        if let Ok(report) = run_agent(
            llm.as_ref(),
            &exec,
            rt.as_ref(),
            &prompt,
            &AgentConfig::default(),
        ) {
            if !report.is_empty() {
                let saved = exec.store.lock().ok().and_then(|store| {
                    let ws = exec.workspace.lock().ok()?;
                    ws.as_ref()?.record_report(&store, &model, &report).ok()
                });
                if saved.is_none() {
                    rt.emit(AgentEvent::Failed {
                        error: "could not record the report".into(),
                    });
                }
            }
        }
        rt.running.store(false, Ordering::SeqCst);
    });
    Ok(())
}
