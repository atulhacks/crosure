//! Agent runtime for the app: provider settings, the background run, and the
//! event log the UI polls.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;

use crosure_agent::{
    build_chain, list_models, presets, run_agent, AgentConfig, AgentEvent, AgentSettings,
    KeySource, Provider, ProviderConfig, ProviderView, ScriptedProvider, SessionExecutor, Sink,
};
use serde::Serialize;

use crate::AppState;

/// What the agent dock shows (never a key).
#[derive(Serialize)]
pub struct AgentStatus {
    pub configured: bool,
    pub demo: bool,
    /// Active provider id and label.
    pub provider: String,
    pub label: String,
    pub model: String,
    pub key_source: Option<KeySource>,
    /// How many providers are ready to take over after a decline.
    pub fallbacks: usize,
    pub running: bool,
}

/// Settings as the settings dialog sees them.
#[derive(Serialize)]
pub struct SettingsView {
    pub providers: Vec<ProviderView>,
    pub active: String,
    pub auto_fallback: bool,
    /// Templates for "Add provider" (no keys).
    pub presets: Vec<ProviderConfig>,
}

/// A page of events since a cursor.
#[derive(Serialize)]
pub struct EventPage {
    pub events: Vec<AgentEvent>,
    pub next: usize,
    pub running: bool,
}

/// Settings, run state and the event log for the current run.
pub struct AgentRuntime {
    path: PathBuf,
    settings: Mutex<AgentSettings>,
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
    /// Loads `home/agent.json` (migrating the first format).
    pub fn new(home: &Path) -> Self {
        let path = home.join("agent.json");
        let settings = std::fs::read(&path)
            .map(|b| AgentSettings::from_json(&b))
            .unwrap_or_default();
        Self {
            path,
            settings: Mutex::new(settings),
            events: Mutex::new(Vec::new()),
            running: AtomicBool::new(false),
            stop: AtomicBool::new(false),
        }
    }

    fn snapshot(&self) -> AgentSettings {
        self.settings.lock().map(|s| s.clone()).unwrap_or_default()
    }

    /// Dock status.
    pub fn status(&self) -> AgentStatus {
        let running = self.running.load(Ordering::SeqCst);
        if demo_mode() {
            return AgentStatus {
                configured: true,
                demo: true,
                provider: "demo".into(),
                label: "Scripted demo".into(),
                model: "scripted-demo".into(),
                key_source: None,
                fallbacks: 0,
                running,
            };
        }
        let s = self.snapshot();
        let active = s.providers.iter().find(|p| p.id == s.active);
        let fallbacks = if s.auto_fallback {
            s.providers
                .iter()
                .filter(|p| p.id != s.active && p.ready())
                .count()
        } else {
            0
        };
        AgentStatus {
            configured: active.is_some_and(ProviderConfig::ready),
            demo: false,
            provider: s.active.clone(),
            label: active.map(|p| p.label.clone()).unwrap_or_default(),
            model: active.map(|p| p.model.clone()).unwrap_or_default(),
            key_source: active.map(|p| p.key().1),
            fallbacks,
            running,
        }
    }

    /// Settings for the dialog.
    pub fn settings_view(&self) -> SettingsView {
        let s = self.snapshot();
        SettingsView {
            providers: s.views(),
            active: s.active,
            auto_fallback: s.auto_fallback,
            presets: presets(),
        }
    }

    fn persist(&self, s: &AgentSettings) -> Result<(), String> {
        std::fs::write(
            &self.path,
            serde_json::to_vec_pretty(s).map_err(|e| e.to_string())?,
        )
        .map_err(|e| e.to_string())?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = std::fs::set_permissions(&self.path, std::fs::Permissions::from_mode(0o600));
        }
        Ok(())
    }

    /// Saves settings from the dialog (keys left out are kept; "" clears).
    pub fn save(&self, incoming: AgentSettings) -> Result<SettingsView, String> {
        {
            let mut s = self.settings.lock().map_err(|_| "settings lock poisoned")?;
            s.merge(incoming);
            if !s.providers.iter().any(|p| p.id == s.active) {
                s.active = s
                    .providers
                    .first()
                    .map(|p| p.id.clone())
                    .unwrap_or_default();
            }
            self.persist(&s)?;
        }
        Ok(self.settings_view())
    }

    /// Model ids a provider offers (also tests the connection). Uses the saved key when none is given.
    pub fn models(&self, mut cfg: ProviderConfig) -> Result<Vec<String>, String> {
        if cfg.api_key.as_deref().is_none_or(str::is_empty) {
            cfg.api_key = self
                .snapshot()
                .providers
                .into_iter()
                .find(|p| p.id == cfg.id)
                .and_then(|p| p.api_key);
        }
        list_models(&cfg).map_err(|e| e.to_string())
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

    fn chain(&self) -> Result<Vec<Box<dyn Provider>>, String> {
        if demo_mode() {
            return Ok(vec![Box::new(ScriptedProvider::crackme_demo())]);
        }
        build_chain(&self.snapshot())
            .map_err(|_| "No AI provider is ready. Open agent settings to add one.".to_string())
    }
}

/// Records the request, then runs the agent on a background thread.
pub fn start(state: &AppState, prompt: String) -> Result<(), String> {
    let rt = state.agent.clone();
    if rt.running.swap(true, Ordering::SeqCst) {
        return Err("the agent is already running".into());
    }
    let begin = || -> Result<Vec<Box<dyn Provider>>, String> {
        let chain = rt.chain()?;
        let store = state.store.lock().map_err(|_| "store lock poisoned")?;
        let ws = state
            .workspace
            .lock()
            .map_err(|_| "workspace lock poisoned")?;
        ws.as_ref()
            .ok_or("no binary is open")?
            .record_task(&store, &prompt)
            .map_err(|e| e.to_string())?;
        Ok(chain)
    };
    let chain = match begin() {
        Ok(c) => c,
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
    };
    std::thread::spawn(move || {
        if let Ok(report) = run_agent(&chain, &exec, rt.as_ref(), &prompt, &AgentConfig::default())
        {
            if !report.is_empty() {
                // Attribute the report to whichever provider finished the run.
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
                    ws.as_ref()?.record_report(&store, &by, &report).ok()
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
