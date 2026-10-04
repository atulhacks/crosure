use std::collections::HashSet;

use crosure_recorder::Relation;
use serde::{Deserialize, Serialize};

use crate::record::{TrajStep, Trajectory};
use crate::sft::{prompt_at, SYSTEM_PROMPT};

/// Why a pair was formed.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PairRule {
    /// The analyst branched back to an earlier step: the branch that led to a
    /// result is preferred over the one left behind.
    Branch,
    /// A step tagged `dead_end`, against the next step that led to a result.
    DeadEnd,
}

/// One preference pair: from the same context, `chosen` over `rejected`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PreferencePair {
    pub system: String,
    pub prompt: String,
    pub chosen: String,
    pub rejected: String,
    pub meta: PairMeta,
}

/// Where a pair came from.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PairMeta {
    pub session_id: String,
    pub rule: PairRule,
    pub chosen_seq: u64,
    pub rejected_seq: u64,
    /// The trajectory's group and split (see `assign_splits`).
    #[serde(default)]
    pub group: String,
    #[serde(default)]
    pub split: String,
}

fn index_of(t: &Trajectory, seq: u64) -> Option<usize> {
    t.steps.iter().position(|s| s.seq == seq)
}

fn primary_parent(s: &TrajStep) -> Option<u64> {
    s.parents.first().map(|l| l.seq)
}

fn good(s: &TrajStep) -> bool {
    s.is_action() && s.on_key_path && !s.tags.iter().any(|t| t == "dead_end")
}

fn bad(s: &TrajStep) -> bool {
    s.is_action() && (!s.on_key_path || s.tags.iter().any(|t| t == "dead_end"))
}

/// Preference pairs from branches and dead ends.
///
/// - **Branch**: a step that branches from step P and leads to a result is
///   preferred over P's other children that did not. The prompt is the
///   history up to and including P.
/// - **Dead end**: a step tagged `dead_end` is rejected in favour of the next
///   step that leads to a result. The prompt is the history before the dead end.
///
/// ```
/// use crosure_dataset::{preference_pairs, trajectory, PairRule};
/// use crosure_recorder::{NewStep, StepKind, Store};
/// let store = Store::open_in_memory()?;
/// let s = store.create_session("demo", "/tmp/demo.bin", "00")?;
/// let mut a = NewStep::human(StepKind::Strings, "crosure");
/// a.command = Some("str http".into());
/// a.tags = vec!["dead_end".into()];
/// store.record(&s.id, a)?;
/// let mut b = NewStep::human(StepKind::Xref, "crosure");
/// b.command = Some("xt strcmp".into());
/// store.record(&s.id, b)?;
/// let mut f = NewStep::human(StepKind::Finding, "crosure");
/// f.command = Some("find compares with a decoded key".into());
/// store.record(&s.id, f)?;
/// let t = trajectory(&store.session(&s.id)?, &store.steps(&s.id)?, true);
/// let pairs = preference_pairs(&t, 40);
/// assert_eq!(pairs.len(), 1);
/// assert_eq!(pairs[0].meta.rule, PairRule::DeadEnd);
/// assert_eq!((pairs[0].chosen.as_str(), pairs[0].rejected.as_str()), ("xt strcmp", "str http"));
/// # Ok::<(), crosure_recorder::RecorderError>(())
/// ```
pub fn preference_pairs(t: &Trajectory, history: usize) -> Vec<PreferencePair> {
    let mut out = Vec::new();
    let mut used: HashSet<(u64, u64)> = HashSet::new();
    let mut push = |rule, prompt_end: usize, chosen: &TrajStep, rejected: &TrajStep| {
        if used.insert((chosen.seq, rejected.seq)) {
            out.push(PreferencePair {
                system: SYSTEM_PROMPT.into(),
                prompt: prompt_at(t, prompt_end, history),
                chosen: chosen.answer(),
                rejected: rejected.answer(),
                meta: PairMeta {
                    session_id: t.session_id.clone(),
                    rule,
                    chosen_seq: chosen.seq,
                    rejected_seq: rejected.seq,
                    group: t.group.clone(),
                    split: t.split.clone(),
                },
            });
        }
    };

    for b in t.steps.iter().filter(|s| good(s)) {
        for link in b.parents.iter().filter(|l| l.rel == Relation::Branch) {
            let Some(p) = index_of(t, link.seq) else {
                continue;
            };
            let abandoned = t
                .steps
                .iter()
                .filter(|s| s.seq != b.seq && primary_parent(s) == Some(link.seq) && bad(s));
            for r in abandoned {
                push(PairRule::Branch, p + 1, b, r);
            }
        }
    }

    for (i, d) in t.steps.iter().enumerate() {
        if !d.is_action() || !d.tags.iter().any(|t| t == "dead_end") {
            continue;
        }
        if let Some(c) = t.steps[i + 1..].iter().find(|s| good(s)) {
            push(PairRule::DeadEnd, i, c, d);
        }
    }
    out
}
