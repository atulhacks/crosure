use std::collections::BTreeMap;

use crosure_recorder::{ActorKind, Intent, Relation, StepKind, Target};
use serde::{Deserialize, Serialize};

/// One visible step on the canvas.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct GraphNode {
    pub id: String,
    pub seq: u64,
    pub ts_ms: i64,
    pub kind: StepKind,
    pub actor: ActorKind,
    pub tool: String,
    /// Short title, e.g. `xrefs 0x401000`.
    pub label: String,
    /// What the step showed.
    pub summary: String,
    pub command: Option<String>,
    pub target: Option<Target>,
    /// Intent, with later annotations merged in.
    pub intent: Option<Intent>,
    /// Tags, with later annotations merged in.
    pub tags: Vec<String>,
    /// Time until the next step (the analyst's attention on this one).
    pub dwell_ms: u64,
    /// True when this step leads to a `key_step`, `finding` or `verdict`.
    pub on_key_path: bool,
    pub hash: String,
}

/// A typed edge between two visible steps.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct GraphEdge {
    pub from: String,
    pub to: String,
    pub rel: Relation,
}

/// Session-level counts.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct GraphStats {
    pub steps: usize,
    pub human_steps: usize,
    pub agent_steps: usize,
    pub by_kind: BTreeMap<String, usize>,
    pub dead_ends: usize,
    pub findings: usize,
    pub branches: usize,
    pub duration_ms: u64,
}

/// The whole investigation, ready to draw or export.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct InvestigationGraph {
    pub nodes: Vec<GraphNode>,
    pub edges: Vec<GraphEdge>,
    pub stats: GraphStats,
}
