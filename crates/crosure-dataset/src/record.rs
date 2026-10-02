use std::collections::HashMap;
use std::path::Path;

use crosure_recorder::{ActorKind, Relation, Session, Step, StepKind, CONTEXT_CHIP};
use serde::{Deserialize, Serialize};

/// Format tag written on every trajectory.
pub const TRAJECTORY_FORMAT: &str = "crosure.trajectory.v1";

/// A link from a step to an earlier step of the same trajectory.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Link {
    pub seq: u64,
    pub rel: Relation,
}

/// One step as it appears in a dataset: what was done, why, what it showed,
/// and how the investigation judged it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TrajStep {
    pub seq: u64,
    pub actor: ActorKind,
    /// Provider and model for AI steps, e.g. `anthropic:claude-opus-5-5`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    pub kind: StepKind,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub command: Option<String>,
    /// Function, import or string the step targeted (or its address).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target: Option<String>,
    /// The stated reason (free text, else the intent chip).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub why: Option<String>,
    pub summary: String,
    pub tags: Vec<String>,
    /// The step leads to a finding, verdict or `key_step`.
    pub on_key_path: bool,
    /// Taken only to give an AI prompt context (an `@` mention).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub context: bool,
    pub parents: Vec<Link>,
    pub hash: String,
}

impl TrajStep {
    /// True for steps a model could be trained to take (not the load, the
    /// analyst's question, the agent's report, or context for a prompt).
    pub fn is_action(&self) -> bool {
        self.command.is_some()
            && !self.context
            && !matches!(self.kind, StepKind::Load | StepKind::Agent)
    }

    /// The step as a training target: the command, then the reason.
    ///
    /// ```
    /// use crosure_dataset::TrajStep;
    /// use crosure_recorder::{ActorKind, StepKind};
    /// let s = TrajStep {
    ///     seq: 3, actor: ActorKind::Human, model: None, kind: StepKind::Xref,
    ///     command: Some("xt strcmp".into()), target: None,
    ///     why: Some("the check compares strings".into()), summary: String::new(),
    ///     tags: vec![], on_key_path: true, context: false, parents: vec![],
    ///     hash: String::new(),
    /// };
    /// assert_eq!(s.answer(), "xt strcmp\nwhy: the check compares strings");
    /// ```
    pub fn answer(&self) -> String {
        let cmd = self.command.clone().unwrap_or_default();
        match &self.why {
            Some(w) if !w.trim().is_empty() => format!("{cmd}\nwhy: {w}"),
            _ => cmd,
        }
    }

    /// One line of history: `#4 dis check_password: <summary>`.
    pub fn history_line(&self) -> String {
        let cmd = self.command.as_deref().unwrap_or("");
        format!("#{} {cmd}: {}", self.seq, self.summary)
    }

    /// The analyst's question when this is an `ask …` step.
    pub fn question(&self) -> Option<&str> {
        if self.kind != StepKind::Agent || self.actor != ActorKind::Human {
            return None;
        }
        self.command.as_deref()?.strip_prefix("ask ")
    }
}

/// A whole verified investigation of one binary.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Trajectory {
    pub format: String,
    pub session_id: String,
    /// File name only; local paths are not exported.
    pub binary: String,
    pub binary_sha256: String,
    /// Hash of the last step: anchors every step in this record.
    pub head_hash: String,
    pub verified: bool,
    pub steps: Vec<TrajStep>,
}

/// Builds a trajectory from a session's steps. Annotations are folded into
/// the steps they annotate, and key paths are marked (via `crosure-graph`).
///
/// ```
/// use crosure_recorder::{NewStep, StepKind, Store};
/// let store = Store::open_in_memory()?;
/// let s = store.create_session("demo", "/tmp/demo.bin", "00")?;
/// store.record(&s.id, NewStep::human(StepKind::Load, "crosure"))?;
/// let mut f = NewStep::human(StepKind::Finding, "crosure");
/// f.command = Some("find it decodes a key".into());
/// store.record(&s.id, f)?;
/// let t = crosure_dataset::trajectory(&store.session(&s.id)?, &store.steps(&s.id)?, true);
/// assert_eq!(t.binary, "demo.bin");
/// assert!(t.steps.iter().all(|s| s.on_key_path));
/// # Ok::<(), crosure_recorder::RecorderError>(())
/// ```
pub fn trajectory(session: &Session, steps: &[Step], verified: bool) -> Trajectory {
    let binary = Path::new(&session.binary_path)
        .file_name()
        .map(|f| f.to_string_lossy().into_owned())
        .unwrap_or_else(|| session.name.clone());
    // Local paths (the load command, for one) are reduced to the file name.
    let scrub = |text: &str| -> String {
        if session.binary_path.is_empty() {
            text.to_string()
        } else {
            text.replace(&session.binary_path, &binary)
        }
    };
    let graph = crosure_graph::build(steps);
    let by_id: HashMap<&str, &Step> = steps.iter().map(|s| (s.id.as_str(), s)).collect();
    let seq_of: HashMap<&str, u64> = graph.nodes.iter().map(|n| (n.id.as_str(), n.seq)).collect();
    let mut parents: HashMap<&str, Vec<Link>> = HashMap::new();
    for e in &graph.edges {
        if let Some(&seq) = seq_of.get(e.from.as_str()) {
            parents
                .entry(e.to.as_str())
                .or_default()
                .push(Link { seq, rel: e.rel });
        }
    }
    let steps = graph
        .nodes
        .iter()
        .map(|n| {
            let target = n.target.as_ref().and_then(|t| {
                t.name
                    .clone()
                    .or_else(|| t.func.clone())
                    .or_else(|| t.addr.clone())
            });
            let why = n.intent.as_ref().and_then(|i| {
                i.note
                    .clone()
                    .filter(|s| !s.trim().is_empty())
                    .or_else(|| i.chip.clone())
            });
            TrajStep {
                seq: n.seq,
                actor: n.actor,
                model: by_id.get(n.id.as_str()).and_then(|s| s.actor.model.clone()),
                kind: n.kind,
                command: n.command.as_deref().map(scrub),
                target: target.as_deref().map(scrub),
                why: why.as_deref().map(scrub),
                summary: scrub(&n.summary),
                tags: n.tags.clone(),
                on_key_path: n.on_key_path,
                context: n
                    .intent
                    .as_ref()
                    .is_some_and(|i| i.chip.as_deref() == Some(CONTEXT_CHIP)),
                parents: parents.remove(n.id.as_str()).unwrap_or_default(),
                hash: n.hash.clone(),
            }
        })
        .collect();
    Trajectory {
        format: TRAJECTORY_FORMAT.into(),
        session_id: session.id.clone(),
        binary,
        binary_sha256: session.binary_sha256.clone(),
        head_hash: session.head_hash.clone(),
        verified,
        steps,
    }
}
