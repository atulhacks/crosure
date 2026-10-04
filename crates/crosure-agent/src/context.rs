//! Keeps a conversation inside the model's context window.
//!
//! Old tool results are the bulk of a long run, and every one of them is a
//! recorded step the model can fetch again. So instead of summarizing (an
//! extra paid call that loses addresses), the oldest large results are
//! replaced by their first line plus a note to re-run the call.

use crate::{Block, Entry, Transcript};

/// Characters per token, for a cheap size estimate.
const CHARS_PER_TOKEN: usize = 4;
/// Results shorter than this are left alone: eliding them saves little.
const MIN_ELIDE_CHARS: usize = 600;
/// The newest result entries are never elided: the model is working on them.
const KEEP_RECENT: usize = 3;
const STUB: &str = "[elided to save context";

/// A rough token count of what a request for `t` sends.
///
/// ```
/// use crosure_agent::{estimate_tokens, Entry, Transcript};
/// let t = Transcript { entries: vec![Entry::User("x".repeat(4000))], ..Default::default() };
/// assert!(estimate_tokens(&t) >= 1000);
/// ```
pub fn estimate_tokens(t: &Transcript) -> usize {
    let mut chars = t.system_prompt().len();
    for e in &t.entries {
        chars += match e {
            Entry::User(s) => s.len(),
            Entry::Assistant { turn, .. } => turn
                .blocks
                .iter()
                .map(|b| match b {
                    Block::Text(s) | Block::Thinking(s) => s.len(),
                    Block::ToolUse { name, input, .. } => name.len() + input.to_string().len(),
                })
                .sum(),
            Entry::Results { results, note } => {
                results.iter().map(|r| r.content.len()).sum::<usize>()
                    + note.as_ref().map_or(0, String::len)
            }
        };
    }
    chars / CHARS_PER_TOKEN
}

/// Elides the oldest large tool results until the estimate is at most
/// `target` tokens, keeping the newest few intact. Returns how many were
/// elided. Tool-call/result pairing is untouched, so the request stays valid.
///
/// ```
/// use crosure_agent::{elide_old_results, estimate_tokens, Entry, ToolResult, Transcript};
/// let big = |id: &str| Entry::Results {
///     results: vec![ToolResult { id: id.into(), content: format!("12 functions\n{}", "x".repeat(8000)), is_error: false }],
///     note: None,
/// };
/// let mut t = Transcript { entries: (0..6).map(|i| big(&i.to_string())).collect(), ..Default::default() };
/// let n = elide_old_results(&mut t, 7000);
/// assert_eq!(n, 3, "the 3 newest are kept");
/// assert!(estimate_tokens(&t) <= 7000);
/// ```
pub fn elide_old_results(t: &mut Transcript, target: usize) -> usize {
    let result_entries: Vec<usize> = t
        .entries
        .iter()
        .enumerate()
        .filter(|(_, e)| matches!(e, Entry::Results { .. }))
        .map(|(i, _)| i)
        .collect();
    let protected = result_entries.len().saturating_sub(KEEP_RECENT);
    let mut tokens = estimate_tokens(t);
    let mut elided = 0;
    for &i in result_entries.iter().take(protected) {
        if tokens <= target {
            break;
        }
        if let Some(Entry::Results { results, .. }) = t.entries.get_mut(i) {
            for r in results.iter_mut() {
                if r.is_error || r.content.len() < MIN_ELIDE_CHARS || r.content.starts_with(STUB) {
                    continue;
                }
                let first = r.content.lines().next().unwrap_or("").to_string();
                let stub =
                    format!("{STUB}; it was: {first}. Call the same tool again if you need it.]");
                tokens = tokens.saturating_sub((r.content.len() - stub.len()) / CHARS_PER_TOKEN);
                r.content = stub;
                elided += 1;
            }
        }
    }
    elided
}

/// Whether a provider error means the request was too long for the model.
///
/// ```
/// assert!(crosure_agent::is_context_overflow("prompt is too long: 210000 tokens > 200000 maximum"));
/// assert!(crosure_agent::is_context_overflow("This model's maximum context length is 32768 tokens"));
/// assert!(!crosure_agent::is_context_overflow("rate limited"));
/// ```
pub fn is_context_overflow(message: &str) -> bool {
    let m = message.to_lowercase();
    [
        "prompt is too long",
        "context_length_exceeded",
        "maximum context length",
        "context length",
        "context window",
        "too many tokens",
        "exceeds the maximum number of tokens",
        "n_ctx",
    ]
    .iter()
    .any(|p| m.contains(p))
}
