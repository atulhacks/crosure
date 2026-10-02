use std::collections::{BTreeMap, HashMap, HashSet};

use crosure_recorder::{ActorKind, Intent, Relation, Step, StepKind};

use crate::model::{GraphEdge, GraphNode, GraphStats, InvestigationGraph};
use crate::paths::mark_key_paths;

/// A short, human title for a step.
///
/// ```
/// use crosure_recorder::{NewStep, StepKind, Store, Target};
/// let store = Store::open_in_memory()?;
/// let s = store.create_session("t", "/bin/true", "00")?;
/// let mut x = NewStep::human(StepKind::Xref, "crosure");
/// x.target = Some(Target { addr: Some("0x401000".into()), ..Default::default() });
/// let step = store.record(&s.id, x)?;
/// assert_eq!(crosure_graph::label(&step), "xref 0x401000");
/// # Ok::<(), crosure_recorder::RecorderError>(())
/// ```
pub fn label(step: &Step) -> String {
    let kind = serde_json::to_value(step.kind)
        .ok()
        .and_then(|v| v.as_str().map(str::to_string))
        .unwrap_or_default();
    if step.tool != "crosure" {
        if let Some(cmd) = &step.command {
            return cmd.clone();
        }
    }
    let target = step
        .target
        .as_ref()
        .and_then(|t| t.func.clone().or_else(|| t.addr.clone()));
    match target {
        Some(t) => format!("{kind} {t}"),
        None => kind,
    }
}

fn merge_intent(base: &mut Option<Intent>, extra: &Intent) {
    let b = base.get_or_insert_with(Intent::default);
    if extra.chip.is_some() {
        b.chip = extra.chip.clone();
    }
    if extra.note.is_some() {
        b.note = extra.note.clone();
    }
}

/// Builds the investigation graph from a session's steps (in `seq` order).
/// `annotate` steps are folded into the step they annotate.
pub fn build(steps: &[Step]) -> InvestigationGraph {
    let mut nodes: Vec<GraphNode> = Vec::new();
    let mut index: HashMap<String, usize> = HashMap::new();
    let mut alias: HashMap<String, String> = HashMap::new();
    let mut edges: Vec<GraphEdge> = Vec::new();
    let mut seen: HashSet<(String, String)> = HashSet::new();

    for step in steps {
        if step.kind == StepKind::Annotate {
            for p in step.parents.iter().filter(|p| p.rel == Relation::Annotates) {
                let id = alias.get(&p.id).unwrap_or(&p.id);
                if let Some(&i) = index.get(id) {
                    let node = &mut nodes[i];
                    if let Some(extra) = &step.intent {
                        merge_intent(&mut node.intent, extra);
                    }
                    for t in &step.tags {
                        if !node.tags.contains(t) {
                            node.tags.push(t.clone());
                        }
                    }
                }
            }
            if let Some(last) = nodes.last() {
                alias.insert(step.id.clone(), last.id.clone());
            }
            continue;
        }
        for p in &step.parents {
            let from = alias.get(&p.id).cloned().unwrap_or_else(|| p.id.clone());
            if from != step.id
                && index.contains_key(&from)
                && seen.insert((from.clone(), step.id.clone()))
            {
                edges.push(GraphEdge {
                    from,
                    to: step.id.clone(),
                    rel: p.rel,
                });
            }
        }
        index.insert(step.id.clone(), nodes.len());
        nodes.push(GraphNode {
            id: step.id.clone(),
            seq: step.seq,
            ts_ms: step.ts_ms,
            kind: step.kind,
            actor: step.actor.kind,
            tool: step.tool.clone(),
            label: label(step),
            summary: step.observation.summary.clone(),
            command: step.command.clone(),
            target: step.target.clone(),
            intent: step.intent.clone(),
            tags: step.tags.clone(),
            dwell_ms: step.attention.as_ref().map_or(0, |a| a.dwell_ms),
            on_key_path: false,
            hash: step.hash.clone(),
        });
    }
    for i in 0..nodes.len().saturating_sub(1) {
        if nodes[i].dwell_ms == 0 {
            nodes[i].dwell_ms = (nodes[i + 1].ts_ms - nodes[i].ts_ms).max(0) as u64;
        }
    }
    mark_key_paths(&mut nodes, &edges);
    let stats = stats(&nodes, &edges);
    InvestigationGraph {
        nodes,
        edges,
        stats,
    }
}

fn stats(nodes: &[GraphNode], edges: &[GraphEdge]) -> GraphStats {
    let mut by_kind = BTreeMap::new();
    for n in nodes {
        let k = serde_json::to_value(n.kind)
            .ok()
            .and_then(|v| v.as_str().map(str::to_string))
            .unwrap_or_default();
        *by_kind.entry(k).or_insert(0) += 1;
    }
    let tagged = |t: &str| {
        nodes
            .iter()
            .filter(|n| n.tags.iter().any(|x| x == t))
            .count()
    };
    GraphStats {
        steps: nodes.len(),
        human_steps: nodes.iter().filter(|n| n.actor == ActorKind::Human).count(),
        agent_steps: nodes.iter().filter(|n| n.actor == ActorKind::Agent).count(),
        by_kind,
        dead_ends: tagged("dead_end"),
        findings: nodes
            .iter()
            .filter(|n| matches!(n.kind, StepKind::Finding | StepKind::Verdict))
            .count(),
        branches: edges.iter().filter(|e| e.rel == Relation::Branch).count(),
        duration_ms: match (nodes.first(), nodes.last()) {
            (Some(a), Some(b)) => (b.ts_ms - a.ts_ms).max(0) as u64,
            _ => 0,
        },
    }
}
