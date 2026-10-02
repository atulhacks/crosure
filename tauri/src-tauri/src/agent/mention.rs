//! `@` mentions in a prompt: `@check_password` attaches that function's
//! disassembly (recorded as a step, like any other read of the binary) and
//! `@#7` attaches step 7 of the investigation.

use crosure_agent::render_result;
use crosure_recorder::{Intent, Store};
use crosure_session::{Author, Op, Origin, Workspace};

const MAX_CONTEXT: usize = 40_000;

/// The mentions in `prompt`, in order, without duplicates.
///
/// ```ignore
/// assert_eq!(mentions("explain @main and @#3, then @main"), vec!["main", "#3"]);
/// ```
pub fn mentions(prompt: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for word in prompt.split(|c: char| c.is_whitespace() || ",;()?!".contains(c)) {
        let Some(m) = word.strip_prefix('@') else {
            continue;
        };
        let m = m.trim_end_matches(['.', ':']);
        let valid = m.strip_prefix('#').map_or_else(
            || {
                m.chars()
                    .next()
                    .is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
            },
            |n| !n.is_empty() && n.chars().all(|c| c.is_ascii_digit()),
        );
        if valid && !out.iter().any(|x| x == m) {
            out.push(m.to_string());
        }
    }
    out
}

/// Builds the "Attached context" block for a prompt's mentions.
pub fn attach(store: &Store, ws: &mut Workspace, prompt: &str) -> String {
    let mut blocks = Vec::new();
    for m in mentions(prompt) {
        let block = match m.strip_prefix('#') {
            Some(n) => step_block(store, ws, n),
            None => function_block(store, ws, &m),
        };
        if let Some(b) = block {
            blocks.push(b);
        }
    }
    if blocks.is_empty() {
        return String::new();
    }
    let mut out = String::from("\n\nAttached context:\n");
    for b in blocks {
        if out.len() + b.len() > MAX_CONTEXT {
            out.push_str("\n[more context omitted: too large]");
            break;
        }
        out.push_str(&b);
    }
    out
}

fn function_block(store: &Store, ws: &mut Workspace, name: &str) -> Option<String> {
    let addr = ws.resolve(name).ok()?;
    if ws.engine.function_at(addr).ok()??.addr != addr {
        return None;
    }
    let author = Author {
        model: None,
        intent: Some(Intent {
            chip: Some(crosure_recorder::CONTEXT_CHIP.into()),
            note: Some("attached to an AI prompt".into()),
        }),
    };
    let out = ws
        .run_as(
            store,
            Op::Disasm {
                target: name.into(),
            },
            None,
            Origin::Ui,
            author,
        )
        .ok()?;
    Some(format!(
        "\n### @{name}\n{}\n",
        render_result("disasm", &out.step.observation.summary, &out.result)
    ))
}

fn step_block(store: &Store, ws: &Workspace, n: &str) -> Option<String> {
    let seq: u64 = n.parse().ok()?;
    let step = store
        .steps(&ws.session.id)
        .ok()?
        .into_iter()
        .find(|s| s.seq == seq)?;
    let who = if step.actor.model.is_some() {
        "AI"
    } else {
        "analyst"
    };
    Some(format!(
        "\n### step #{seq} ({who})\n{}\n{}\n",
        step.command.unwrap_or_default(),
        step.observation.summary
    ))
}

#[cfg(test)]
mod tests {
    use super::mentions;

    #[test]
    fn finds_functions_and_steps() {
        assert_eq!(
            mentions("explain @main and @#3, then @main."),
            vec!["main", "#3"]
        );
        assert!(mentions("mail me@ x @ @1abc @#x").is_empty());
        assert_eq!(mentions("(@check_password)?"), vec!["check_password"]);
    }
}
