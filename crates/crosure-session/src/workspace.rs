use std::collections::BTreeMap;
use std::path::Path;

use crosure_engine::{Engine, FunctionInfo, NativeEngine};
use crosure_recorder::{
    Intent, NewStep, Observation, Relation, Session, Step, StepKind, Store, Target,
};
use serde_json::json;

use crate::SessionError;

/// An open binary plus the session recording work on it.
pub struct Workspace {
    pub engine: Box<dyn Engine>,
    pub session: Session,
    /// Analyst renames, replayed from `rename` steps.
    pub(crate) renames: BTreeMap<u64, String>,
    /// Analyst comments, replayed from `comment` steps.
    pub(crate) comments: BTreeMap<u64, String>,
    /// Functions per structural hash in this binary (computed on first recall).
    pub(crate) copies: std::cell::OnceCell<BTreeMap<String, usize>>,
}

/// Parses `0x1234` (or plain hex digits) as an address.
pub(crate) fn parse_addr(s: &str) -> Option<u64> {
    let t = s.trim();
    let hex = t.strip_prefix("0x").or_else(|| t.strip_prefix("0X"))?;
    u64::from_str_radix(hex, 16).ok()
}

impl Workspace {
    /// Opens a binary, starts a new session, and records the `load` step.
    pub fn open(
        store: &Store,
        path: &Path,
        name: Option<&str>,
    ) -> Result<(Self, Step), SessionError> {
        let engine = NativeEngine::open(path)?;
        let info = engine.info()?;
        let file_name = path.file_name().map(|n| n.to_string_lossy().into_owned());
        let session_name = name
            .map(str::to_string)
            .or(file_name)
            .unwrap_or_else(|| "binary".into());
        let session = store.create_session(&session_name, &info.path, &info.sha256)?;
        let functions = engine.functions()?.len();
        let mut load = NewStep::human(StepKind::Load, "crosure");
        load.command = Some(format!("open {}", info.path));
        load.action = json!({ "op": "load", "path": info.path, "backend": engine.backend() });
        load.observation = Observation {
            summary: format!(
                "{} {} {}-bit, {} functions{}",
                info.format.to_uppercase(),
                info.arch,
                info.bits,
                functions,
                if info.stripped { ", stripped" } else { "" }
            ),
            blob: Some(store.put_blob(&serde_json::to_vec(&info)?)?),
            truncated: false,
        };
        let step = store.record(&session.id, load)?;
        let ws = Self {
            engine: Box::new(engine),
            session,
            renames: BTreeMap::new(),
            comments: BTreeMap::new(),
            copies: std::cell::OnceCell::new(),
        };
        Ok((ws, step))
    }

    /// Reopens a recorded session's binary and replays its renames/comments.
    /// Fails if the file is no longer the binary the session recorded, so new
    /// steps never attach to a different file.
    pub fn resume(store: &Store, session_id: &str) -> Result<Self, SessionError> {
        let session = store.session(session_id)?;
        let engine = NativeEngine::open(Path::new(&session.binary_path))?;
        let found = engine.info()?.sha256;
        if found != session.binary_sha256 {
            return Err(SessionError::BinaryChanged {
                path: session.binary_path.clone(),
                expected: session.binary_sha256.clone(),
                found,
            });
        }
        let mut ws = Self {
            engine: Box::new(engine),
            session,
            renames: BTreeMap::new(),
            comments: BTreeMap::new(),
            copies: std::cell::OnceCell::new(),
        };
        for step in store.steps(session_id)? {
            let addr = step.action.get("addr").and_then(|a| a.as_u64());
            match (step.kind, addr) {
                (StepKind::Rename, Some(a)) => {
                    if let Some(n) = step.action.get("name").and_then(|n| n.as_str()) {
                        ws.renames.insert(a, n.to_string());
                    }
                }
                (StepKind::Comment, Some(a)) => {
                    if let Some(t) = step.action.get("text").and_then(|t| t.as_str()) {
                        ws.comments.insert(a, t.to_string());
                    }
                }
                _ => {}
            }
        }
        Ok(ws)
    }

    /// Functions with analyst renames applied.
    pub fn functions(&self) -> Result<Vec<FunctionInfo>, SessionError> {
        let mut fs = self.engine.functions()?;
        for f in &mut fs {
            if let Some(n) = self.renames.get(&f.addr) {
                f.name = n.clone();
            }
        }
        Ok(fs)
    }

    /// The display name for an address: rename, engine name, or hex.
    pub fn name_of(&self, addr: u64) -> Option<String> {
        if let Some(n) = self.renames.get(&addr) {
            return Some(n.clone());
        }
        let f = self.engine.function_at(addr).ok()??;
        Some(match (self.renames.get(&f.addr), f.addr == addr) {
            (Some(n), true) => n.clone(),
            (None, true) => f.name,
            (Some(n), false) => format!("{n}+{:#x}", addr - f.addr),
            (None, false) => format!("{}+{:#x}", f.name, addr - f.addr),
        })
    }

    /// Resolves a name (renamed, engine, import) or `0x…` address.
    pub fn resolve(&self, target: &str) -> Result<u64, SessionError> {
        if let Some(a) = parse_addr(target) {
            return Ok(a);
        }
        if let Some((a, _)) = self.renames.iter().find(|(_, n)| n.as_str() == target) {
            return Ok(*a);
        }
        self.engine
            .resolve(target)?
            .ok_or_else(|| SessionError::Unresolved(target.into()))
    }

    /// Step target for an address: hex, containing function (with its
    /// fingerprint, for cross-session recall), section.
    pub(crate) fn target_for(&self, addr: u64) -> Target {
        let containing = self.engine.function_at(addr).ok().flatten();
        let func_fp = containing
            .as_ref()
            .and_then(|f| self.engine.function_hash(f.addr).ok().flatten())
            .map(|h| h.fingerprint());
        let func = containing.map(|f| self.renames.get(&f.addr).cloned().unwrap_or(f.name));
        let section = self.engine.info().ok().and_then(|i| {
            i.sections
                .into_iter()
                .find(|s| addr >= s.addr && addr < s.addr + s.size)
                .map(|s| s.name)
        });
        Target {
            addr: Some(format!("{addr:#x}")),
            func,
            func_fp,
            section,
            name: self.display_name(addr),
        }
    }

    /// A string literal starting exactly at `addr`, quoted and shortened.
    pub(crate) fn string_at(&self, addr: u64) -> Option<String> {
        let s = self
            .engine
            .strings(4)
            .ok()?
            .into_iter()
            .find(|s| s.mapped && s.addr == addr)?;
        let mut v: String = s.value.chars().take(40).collect();
        if s.value.chars().count() > 40 {
            v.push('…');
        }
        Some(format!("\"{v}\""))
    }

    /// What a person would call `addr`: a function/import name or a string literal.
    pub(crate) fn display_name(&self, addr: u64) -> Option<String> {
        self.name_of(addr).or_else(|| self.string_at(addr))
    }

    /// The target to write in a recorded command: a symbol name when `addr` is
    /// exactly a named function or import, otherwise hex. Keeps commands
    /// replayable on the console and readable in datasets.
    pub(crate) fn canonical_target(&self, addr: u64) -> String {
        match self.name_of(addr) {
            Some(n) if !n.contains(['+', ' ', '"']) && self.resolve(&n).ok() == Some(addr) => n,
            _ => format!("{addr:#x}"),
        }
    }

    /// Records the analyst asking the agent to do something (a human `agent` step).
    pub fn record_task(&self, store: &Store, prompt: &str) -> Result<Step, SessionError> {
        let mut s = NewStep::human(StepKind::Agent, "crosure");
        s.command = Some(format!("ask {prompt}"));
        s.action = json!({ "op": "ask", "prompt": prompt });
        s.observation = Observation {
            summary: format!("asked the agent: {prompt}"),
            blob: None,
            truncated: false,
        };
        Ok(store.record(&self.session.id, s)?)
    }

    /// Records the agent's final report (an agent-authored `agent` step; full text in the blob).
    pub fn record_report(
        &self,
        store: &Store,
        model: &str,
        report: &str,
    ) -> Result<Step, SessionError> {
        let mut s = NewStep::agent(StepKind::Agent, "agent", model);
        let headline = report
            .lines()
            .map(|l| l.trim().trim_start_matches('#').trim())
            .find(|l| !l.is_empty() && !l.starts_with('*') && !l.eq_ignore_ascii_case("summary"))
            .unwrap_or("report")
            .chars()
            .take(160)
            .collect::<String>();
        s.command = Some("report".into());
        s.action = json!({ "op": "report" });
        s.observation = Observation {
            summary: format!("report: {headline}"),
            blob: Some(store.put_blob(report.as_bytes())?),
            truncated: false,
        };
        Ok(store.record(&self.session.id, s)?)
    }

    /// Adds intent and/or tags to an earlier step (as a new `annotate` step;
    /// recorded steps are never edited).
    pub fn annotate(
        &self,
        store: &Store,
        step_id: &str,
        intent: Option<Intent>,
        tags: Vec<String>,
    ) -> Result<Step, SessionError> {
        let mut s =
            NewStep::human(StepKind::Annotate, "crosure").parent(step_id, Relation::Annotates);
        s.action = json!({ "op": "annotate", "step": step_id });
        s.intent = intent;
        s.tags = tags;
        Ok(store.record(&self.session.id, s)?)
    }
}
