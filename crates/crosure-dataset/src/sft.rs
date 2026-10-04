use crosure_recorder::ActorKind;
use serde::{Deserialize, Serialize};

use crate::record::Trajectory;

/// System prompt shared by SFT and DPO records.
pub const SYSTEM_PROMPT: &str = "You are a reverse engineer working in Crosure, a static-analysis \
workbench. Given the binary, the task and the steps taken so far, give the next step as one \
Crosure command on the first line, then `why: <reason>` on the second.";

const DEFAULT_TASK: &str = "Work out what this binary does.";

/// Which steps become examples, and how much history each one sees.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SftOptions {
    /// Only steps on a path to a finding, verdict or `key_step` (default true).
    pub key_path_only: bool,
    /// Only steps a human took (default false: AI steps are included and labelled).
    pub humans_only: bool,
    /// Most recent steps shown as history (default 40).
    pub history: usize,
}

impl Default for SftOptions {
    fn default() -> Self {
        Self {
            key_path_only: true,
            humans_only: false,
            history: 40,
        }
    }
}

/// A chat message.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Message {
    pub role: String,
    pub content: String,
}

impl Message {
    fn new(role: &str, content: String) -> Self {
        Self {
            role: role.into(),
            content,
        }
    }
}

/// Where an example came from.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ExampleMeta {
    pub session_id: String,
    pub seq: u64,
    pub actor: ActorKind,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    pub on_key_path: bool,
    /// Hash of the step, so every example can be traced to the chain.
    pub hash: String,
    /// The trajectory's group and split (see `assign_splits`).
    #[serde(default)]
    pub group: String,
    #[serde(default)]
    pub split: String,
}

/// One supervised example in chat format: system, context, next step.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SftExample {
    pub messages: Vec<Message>,
    pub meta: ExampleMeta,
}

/// The user turn for "what next?" after the first `end` steps.
///
/// The task is the analyst's latest question before that point, if any.
pub fn prompt_at(t: &Trajectory, end: usize, history: usize) -> String {
    let before = &t.steps[..end.min(t.steps.len())];
    let task = before
        .iter()
        .rev()
        .find_map(|s| s.question())
        .unwrap_or(DEFAULT_TASK);
    let shown: Vec<String> = before
        .iter()
        .filter(|s| s.is_action())
        .map(|s| s.history_line())
        .collect();
    let skip = shown.len().saturating_sub(history);
    let mut out = format!("Binary: {} ({})\nTask: {task}\n", t.binary, t.binary_sha256);
    if shown.is_empty() {
        out.push_str("No steps yet.\n");
    } else {
        out.push_str("Steps so far:\n");
        if skip > 0 {
            out.push_str(&format!("({skip} earlier steps omitted)\n"));
        }
        for line in &shown[skip..] {
            out.push_str(line);
            out.push('\n');
        }
    }
    out.push_str("What is the next step?");
    out
}

/// Supervised examples: for each chosen step, the context before it and the
/// step itself (command and reason) as the answer.
///
/// ```
/// use crosure_dataset::{sft_examples, trajectory, SftOptions};
/// use crosure_recorder::{NewStep, StepKind, Store};
/// let store = Store::open_in_memory()?;
/// let s = store.create_session("demo", "/tmp/demo.bin", "00")?;
/// let mut x = NewStep::human(StepKind::Xref, "crosure");
/// x.command = Some("xt strcmp".into());
/// store.record(&s.id, x)?;
/// let mut f = NewStep::human(StepKind::Finding, "crosure");
/// f.command = Some("find input is compared with a decoded key".into());
/// store.record(&s.id, f)?;
/// let t = trajectory(&store.session(&s.id)?, &store.steps(&s.id)?, true);
/// let ex = sft_examples(&t, &SftOptions::default());
/// assert_eq!(ex.len(), 2);
/// assert!(ex[1].messages[1].content.contains("#0 xt strcmp"));
/// assert_eq!(ex[1].messages[2].content, "find input is compared with a decoded key");
/// # Ok::<(), crosure_recorder::RecorderError>(())
/// ```
pub fn sft_examples(t: &Trajectory, opts: &SftOptions) -> Vec<SftExample> {
    t.steps
        .iter()
        .enumerate()
        .filter(|(_, s)| s.is_action())
        .filter(|(_, s)| !opts.key_path_only || s.on_key_path)
        .filter(|(_, s)| !opts.humans_only || s.actor == ActorKind::Human)
        .map(|(i, s)| SftExample {
            messages: vec![
                Message::new("system", SYSTEM_PROMPT.into()),
                Message::new("user", prompt_at(t, i, opts.history)),
                Message::new("assistant", s.answer()),
            ],
            meta: ExampleMeta {
                session_id: t.session_id.clone(),
                seq: s.seq,
                actor: s.actor,
                model: s.model.clone(),
                on_key_path: s.on_key_path,
                hash: s.hash.clone(),
                group: t.group.clone(),
                split: t.split.clone(),
            },
        })
        .collect()
}
