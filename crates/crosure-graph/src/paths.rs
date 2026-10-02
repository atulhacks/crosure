use std::collections::{HashMap, HashSet};

use crosure_recorder::StepKind;

use crate::build;
use crate::model::{GraphEdge, GraphNode, InvestigationGraph};

fn is_key(n: &GraphNode) -> bool {
    matches!(n.kind, StepKind::Finding | StepKind::Verdict)
        || n.tags.iter().any(|t| t == "key_step")
}

/// Marks every ancestor of a key node (finding, verdict, `key_step` tag).
/// Ancestors are followed through `derived_from`, `branch`, `confirms` and
/// `refutes` edges, and through `next` edges unless the parent is a dead end.
pub(crate) fn mark_key_paths(nodes: &mut [GraphNode], edges: &[GraphEdge]) {
    let dead: HashSet<&str> = nodes
        .iter()
        .filter(|n| n.tags.iter().any(|t| t == "dead_end"))
        .map(|n| n.id.as_str())
        .collect();
    let mut parents: HashMap<&str, Vec<&str>> = HashMap::new();
    for e in edges {
        if !dead.contains(e.from.as_str()) {
            parents
                .entry(e.to.as_str())
                .or_default()
                .push(e.from.as_str());
        }
    }
    let mut stack: Vec<&str> = nodes
        .iter()
        .filter(|n| is_key(n))
        .map(|n| n.id.as_str())
        .collect();
    let mut on: HashSet<String> = HashSet::new();
    while let Some(id) = stack.pop() {
        if on.insert(id.to_string()) {
            stack.extend(parents.get(id).into_iter().flatten());
        }
    }
    for n in nodes.iter_mut() {
        n.on_key_path = on.contains(&n.id);
    }
}

/// The graph as it looked right after step `upto_seq` (for the timeline).
///
/// ```
/// use crosure_recorder::{NewStep, StepKind, Store};
/// let store = Store::open_in_memory()?;
/// let s = store.create_session("t", "/bin/true", "00")?;
/// for k in [StepKind::Load, StepKind::Strings, StepKind::Xref] {
///     store.record(&s.id, NewStep::human(k, "crosure"))?;
/// }
/// let g = crosure_graph::replay(&store.steps(&s.id)?, 1);
/// assert_eq!(g.nodes.len(), 2);
/// # Ok::<(), crosure_recorder::RecorderError>(())
/// ```
pub fn replay(steps: &[crosure_recorder::Step], upto_seq: u64) -> InvestigationGraph {
    let prefix: Vec<_> = steps
        .iter()
        .filter(|s| s.seq <= upto_seq)
        .cloned()
        .collect();
    build(&prefix)
}
