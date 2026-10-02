use serde::{Deserialize, Serialize};
use serde_json::Value;

/// What kind of investigation step a node represents.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StepKind {
    Load,
    Recon,
    Navigate,
    Functions,
    Disasm,
    Decompile,
    Xref,
    Strings,
    Imports,
    Rename,
    Comment,
    Patch,
    Debug,
    Sandbox,
    Shell,
    Agent,
    Hypothesis,
    Finding,
    Verdict,
    /// Adds intent/tags to an earlier step without mutating it.
    Annotate,
}

/// How a step relates to one of its parents.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Relation {
    /// Time order: the parent happened right before.
    Next,
    /// Data dependency: this step used something the parent produced.
    DerivedFrom,
    /// The analyst went back to the parent and tried something else.
    Branch,
    /// This step supports the parent hypothesis.
    Confirms,
    /// This step disproves the parent hypothesis.
    Refutes,
    /// This step annotates the parent (intent, tags).
    Annotates,
}

/// A typed edge to an earlier step.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ParentRef {
    pub id: String,
    pub rel: Relation,
}

/// Whether a human or an AI agent took the step.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActorKind {
    Human,
    Agent,
}

/// Who took the step.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Actor {
    #[serde(rename = "type")]
    pub kind: ActorKind,
    /// Anonymous, stable id (never a real username).
    pub id: String,
    /// Model id when `kind` is `Agent`.
    pub model: Option<String>,
}

/// What in the binary the step looked at.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Target {
    pub addr: Option<String>,
    pub func: Option<String>,
    /// Structural fingerprint of the function, for cross-sample matching.
    pub func_fp: Option<String>,
    pub section: Option<String>,
    /// Human name of what was targeted (function, import, or string literal).
    /// Omitted when empty, so steps recorded before this field verify unchanged.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

/// What came back from the step.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Observation {
    /// Short human-readable summary.
    pub summary: String,
    /// `sha256:` key of the full output in the blob store.
    pub blob: Option<String>,
    pub truncated: bool,
}

/// Intent chip for steps taken only to give an AI prompt context (an `@`
/// mention), not as an analysis decision. Dataset exports leave them out as
/// training targets.
pub const CONTEXT_CHIP: &str = "ai_context";

/// Why the analyst took the step.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Intent {
    /// One-click intent chip such as `find_c2`.
    pub chip: Option<String>,
    /// Free-text note.
    pub note: Option<String>,
}

/// Implicit attention signals.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Attention {
    pub dwell_ms: u64,
    pub revisits: u32,
}

/// A recorded, hash-chained step. Immutable once stored.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Step {
    pub id: String,
    pub session_id: String,
    /// Position in the session's chain, starting at 0.
    pub seq: u64,
    pub ts_ms: i64,
    pub parents: Vec<ParentRef>,
    pub actor: Actor,
    pub kind: StepKind,
    pub tool: String,
    pub command: Option<String>,
    pub action: Value,
    pub target: Option<Target>,
    pub observation: Observation,
    pub intent: Option<Intent>,
    pub tags: Vec<String>,
    pub attention: Option<Attention>,
    pub prev_hash: String,
    pub hash: String,
}

/// A step as submitted by a caller; the store fills in id, seq, time and hashes.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct NewStep {
    pub parents: Vec<ParentRef>,
    pub actor: Actor,
    pub kind: StepKind,
    pub tool: String,
    pub command: Option<String>,
    pub action: Value,
    pub target: Option<Target>,
    pub observation: Observation,
    pub intent: Option<Intent>,
    pub tags: Vec<String>,
    pub attention: Option<Attention>,
}

impl NewStep {
    /// A minimal step taken by the local human analyst.
    ///
    /// ```
    /// use crosure_recorder::{ActorKind, NewStep, StepKind};
    /// let s = NewStep::human(StepKind::Disasm, "crosure");
    /// assert_eq!(s.actor.kind, ActorKind::Human);
    /// ```
    pub fn human(kind: StepKind, tool: &str) -> Self {
        Self::with_actor(
            Actor {
                kind: ActorKind::Human,
                id: "local".into(),
                model: None,
            },
            kind,
            tool,
        )
    }

    /// A minimal step taken by an AI agent running `model`.
    ///
    /// ```
    /// use crosure_recorder::{ActorKind, NewStep, StepKind};
    /// let s = NewStep::agent(StepKind::Xref, "crosure", "qwen2.5-coder");
    /// assert_eq!(s.actor.model.as_deref(), Some("qwen2.5-coder"));
    /// ```
    pub fn agent(kind: StepKind, tool: &str, model: &str) -> Self {
        Self::with_actor(
            Actor {
                kind: ActorKind::Agent,
                id: "agent".into(),
                model: Some(model.into()),
            },
            kind,
            tool,
        )
    }

    fn with_actor(actor: Actor, kind: StepKind, tool: &str) -> Self {
        Self {
            parents: Vec::new(),
            actor,
            kind,
            tool: tool.into(),
            command: None,
            action: Value::Null,
            target: None,
            observation: Observation::default(),
            intent: None,
            tags: Vec::new(),
            attention: None,
        }
    }

    /// Adds a parent edge (builder style).
    ///
    /// ```
    /// use crosure_recorder::{NewStep, Relation, StepKind};
    /// let s = NewStep::human(StepKind::Xref, "crosure").parent("stp_1", Relation::DerivedFrom);
    /// assert_eq!(s.parents.len(), 1);
    /// ```
    pub fn parent(mut self, id: &str, rel: Relation) -> Self {
        self.parents.push(ParentRef { id: id.into(), rel });
        self
    }
}
