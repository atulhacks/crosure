//! Agent threads: one conversation each, kept per session and saved to
//! `~/.crosure/threads/<session>.json` so they survive restarts.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use crosure_agent::{AgentEvent, Profile, Transcript};
use serde::{Deserialize, Serialize};

/// A conversation with the agent about one session's binary.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Thread {
    pub id: String,
    pub session_id: String,
    pub title: String,
    pub created_ms: i64,
    pub profile: Profile,
    pub transcript: Transcript,
    /// Everything the UI showed, so reopening a thread looks the same.
    pub events: Vec<AgentEvent>,
}

/// What the thread list shows.
#[derive(Clone, Debug, Serialize)]
pub struct ThreadSummary {
    pub id: String,
    pub title: String,
    pub created_ms: i64,
    pub profile: Profile,
    pub tool_calls: usize,
}

impl Thread {
    /// A new, empty thread titled from its first prompt.
    pub fn new(session_id: &str, prompt: &str, profile: Profile) -> Self {
        let mut title: String = prompt.trim().chars().take(60).collect();
        if prompt.trim().chars().count() > 60 {
            title.push('…');
        }
        Self {
            id: format!("thr_{}", now_ms()),
            session_id: session_id.into(),
            title,
            created_ms: now_ms(),
            profile,
            transcript: Transcript {
                profile,
                ..Default::default()
            },
            events: Vec::new(),
        }
    }

    /// The list entry for this thread.
    pub fn summary(&self) -> ThreadSummary {
        ThreadSummary {
            id: self.id.clone(),
            title: self.title.clone(),
            created_ms: self.created_ms,
            profile: self.profile,
            tool_calls: self
                .events
                .iter()
                .filter(|e| {
                    matches!(
                        e,
                        AgentEvent::ToolCall {
                            step_id: Some(_),
                            ..
                        }
                    )
                })
                .count(),
        }
    }
}

pub(crate) fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

/// Threads of every session seen this run, with the open one per session.
#[derive(Default)]
pub struct ThreadStore {
    dir: PathBuf,
    by_session: BTreeMap<String, BTreeMap<String, Thread>>,
    pub current: Option<String>,
}

impl ThreadStore {
    /// Threads saved under `home/threads`.
    pub fn new(home: &Path) -> Self {
        Self {
            dir: home.join("threads"),
            ..Default::default()
        }
    }

    fn file(&self, session_id: &str) -> PathBuf {
        self.dir.join(format!("{session_id}.json"))
    }

    fn load(&mut self, session_id: &str) -> &mut BTreeMap<String, Thread> {
        if !self.by_session.contains_key(session_id) {
            let threads: Vec<Thread> = std::fs::read(self.file(session_id))
                .ok()
                .and_then(|b| serde_json::from_slice(&b).ok())
                .unwrap_or_default();
            self.by_session.insert(
                session_id.into(),
                threads.into_iter().map(|t| (t.id.clone(), t)).collect(),
            );
        }
        self.by_session.entry(session_id.into()).or_default()
    }

    /// Newest first.
    pub fn list(&mut self, session_id: &str) -> Vec<ThreadSummary> {
        let mut v: Vec<ThreadSummary> = self
            .load(session_id)
            .values()
            .map(Thread::summary)
            .collect();
        v.sort_by_key(|t| std::cmp::Reverse(t.created_ms));
        v
    }

    /// A copy of a thread.
    pub fn get(&mut self, session_id: &str, id: &str) -> Option<Thread> {
        self.load(session_id).get(id).cloned()
    }

    /// Inserts or replaces a thread and saves the session's threads.
    pub fn put(&mut self, thread: Thread) -> Result<(), String> {
        let sid = thread.session_id.clone();
        self.load(&sid).insert(thread.id.clone(), thread);
        let all: Vec<&Thread> = self
            .by_session
            .get(&sid)
            .map(|m| m.values().collect())
            .unwrap_or_default();
        std::fs::create_dir_all(&self.dir).map_err(|e| e.to_string())?;
        std::fs::write(
            self.file(&sid),
            serde_json::to_vec(&all).map_err(|e| e.to_string())?,
        )
        .map_err(|e| e.to_string())
    }
}
