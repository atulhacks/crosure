//! Agent runtime for the app: provider settings, threads, the background
//! run, approvals, and the event log the UI polls.

mod approval;
mod mention;
mod run;
mod threads;

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;

use crosure_agent::{
    build_chain, list_model_info, presets, AgentEvent, AgentSettings, ApprovalRequest, KeySource,
    ModelInfo, Provider, ProviderConfig, ProviderView, ScriptedProvider, Sink,
};
use serde::Serialize;

pub use run::start;
use threads::ThreadStore;
pub use threads::ThreadSummary;

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
    pub instructions: String,
    pub permissions: crosure_agent::Permissions,
    pub default_profile: crosure_agent::Profile,
}

/// A page of events since a cursor.
#[derive(Serialize)]
pub struct EventPage {
    pub events: Vec<AgentEvent>,
    pub next: usize,
    pub running: bool,
}

/// Threads of the open session and which one is shown.
#[derive(Serialize)]
pub struct ThreadList {
    pub threads: Vec<ThreadSummary>,
    pub current: Option<String>,
}

/// Settings, threads, run state and the live event log.
pub struct AgentRuntime {
    path: PathBuf,
    settings: Mutex<AgentSettings>,
    threads: Mutex<ThreadStore>,
    /// Events of the thread being shown (live while it runs).
    events: Mutex<Vec<AgentEvent>>,
    running: AtomicBool,
    stop: AtomicBool,
    gate: approval::Gate,
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
    fn approve(&self, request: &ApprovalRequest) -> bool {
        self.gate
            .wait(&request.id, || self.stop.load(Ordering::SeqCst))
    }
}

fn demo_mode() -> bool {
    std::env::var("CROSURE_AGENT_DEMO").is_ok_and(|v| v == "1")
}

impl AgentRuntime {
    /// Loads `home/agent.json` (migrating the first format) and saved threads lazily.
    pub fn new(home: &Path) -> Self {
        let path = home.join("agent.json");
        let settings = std::fs::read(&path)
            .map(|b| AgentSettings::from_json(&b))
            .unwrap_or_default();
        Self {
            path,
            settings: Mutex::new(settings),
            threads: Mutex::new(ThreadStore::new(home)),
            events: Mutex::new(Vec::new()),
            running: AtomicBool::new(false),
            stop: AtomicBool::new(false),
            gate: approval::Gate::default(),
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
            instructions: s.instructions,
            permissions: s.permissions,
            default_profile: s.default_profile,
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

    /// Models a provider offers, with the limits its server reports (also
    /// tests the connection). Uses the saved key when none is given.
    pub fn models(&self, mut cfg: ProviderConfig) -> Result<Vec<ModelInfo>, String> {
        if cfg.api_key.as_deref().is_none_or(str::is_empty) {
            cfg.api_key = self
                .snapshot()
                .providers
                .into_iter()
                .find(|p| p.id == cfg.id)
                .and_then(|p| p.api_key);
        }
        list_model_info(&cfg).map_err(|e| e.to_string())
    }

    /// Makes `id` the provider runs start on.
    pub fn set_active(&self, id: &str) -> Result<AgentStatus, String> {
        {
            let mut s = self.settings.lock().map_err(|_| "settings lock poisoned")?;
            if !s.providers.iter().any(|p| p.id == id) {
                return Err(format!("unknown provider `{id}`"));
            }
            s.active = id.into();
            self.persist(&s)?;
        }
        Ok(self.status())
    }

    /// Events of the shown thread since `since`.
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

    /// Asks the current run to stop after its in-flight call (a pending approval is denied).
    pub fn request_stop(&self) {
        self.stop.store(true, Ordering::SeqCst);
    }

    /// The analyst's answer to a pending tool call.
    pub fn decide(&self, id: &str, allow: bool) -> Result<(), String> {
        if self.gate.decide(id, allow) {
            Ok(())
        } else {
            Err("nothing is waiting for that decision".into())
        }
    }

    /// Threads of `session_id`, newest first.
    pub fn threads(&self, session_id: &str) -> ThreadList {
        let Ok(mut t) = self.threads.lock() else {
            return ThreadList {
                threads: vec![],
                current: None,
            };
        };
        let threads = t.list(session_id);
        let current = t
            .current
            .clone()
            .filter(|c| threads.iter().any(|x| &x.id == c));
        ThreadList { threads, current }
    }

    /// Shows a saved thread (or a fresh one with `None`). Refused while a run is going.
    pub fn open_thread(&self, session_id: &str, id: Option<&str>) -> Result<(), String> {
        if self.running.load(Ordering::SeqCst) {
            return Err("wait for the agent to finish (or stop it) first".into());
        }
        let mut store = self.threads.lock().map_err(|_| "threads lock poisoned")?;
        let events = match id {
            Some(id) => store.get(session_id, id).ok_or("unknown thread")?.events,
            None => Vec::new(),
        };
        store.current = id.map(str::to_string);
        *self.events.lock().map_err(|_| "events lock poisoned")? = events;
        Ok(())
    }

    fn chain(&self) -> Result<Vec<Box<dyn Provider>>, String> {
        if demo_mode() {
            return Ok(vec![Box::new(ScriptedProvider::crackme_demo())]);
        }
        build_chain(&self.snapshot())
            .map_err(|_| "No AI provider is ready. Open agent settings to add one.".to_string())
    }
}
