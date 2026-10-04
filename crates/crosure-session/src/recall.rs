//! `recall`: what earlier sessions recorded about the same code.
//!
//! Functions are matched by fingerprint (see `crosure_engine::FunctionHash`),
//! so a match survives relinking and stripping. Thresholds follow Ghidra
//! FunctionID: an exact match (small constants equal too) needs
//! [`MIN_EXACT`] instructions, a structural one [`MIN_STRUCTURAL`]. Identical
//! code can still be different functions (template copies, tiny wrappers),
//! so a match is flagged ambiguous when the code repeats in this binary or
//! went by several names; it is never applied automatically.

use std::collections::{BTreeMap, BTreeSet};

use crosure_recorder::{Step, StepKind, Store};
use serde_json::{json, Value};

use crate::run::Done;
use crate::{SessionError, Workspace};

/// Instructions an exact (specific-hash) match needs.
pub const MIN_EXACT: usize = 6;
/// Instructions a structural-only match needs.
pub const MIN_STRUCTURAL: usize = 12;

/// What one earlier session recorded about the matched code.
fn session_match(
    store: &Store,
    id: &str,
    matched: &[(Step, &'static str)],
) -> Result<Value, SessionError> {
    let session = store.session(id)?;
    let ids: BTreeSet<&str> = matched.iter().map(|(s, _)| s.id.as_str()).collect();
    let level = if matched.iter().any(|(_, l)| *l == "exact") {
        "exact"
    } else {
        "structural"
    };
    let mut names = BTreeSet::new();
    let (mut renamed_to, mut comments) = (Vec::new(), Vec::new());
    for (s, _) in matched {
        if let Some(f) = s.target.as_ref().and_then(|t| t.func.clone()) {
            names.insert(f);
        }
        let text = |k: &str| s.action[k].as_str().map(str::to_string);
        match s.kind {
            StepKind::Rename => renamed_to.extend(text("name")),
            StepKind::Comment => comments.extend(text("text")),
            _ => {}
        }
    }
    // Conclusions recorded right after looking at this code.
    let notes: Vec<String> = store
        .steps(id)?
        .into_iter()
        .filter(|s| {
            matches!(
                s.kind,
                StepKind::Finding | StepKind::Hypothesis | StepKind::Verdict
            )
        })
        .filter(|s| s.parents.iter().any(|p| ids.contains(p.id.as_str())))
        .map(|s| s.observation.summary)
        .collect();
    let binary = std::path::Path::new(&session.binary_path)
        .file_name()
        .map_or_else(String::new, |n| n.to_string_lossy().into_owned());
    Ok(json!({
        "session": session.id,
        "session_name": session.name,
        "binary": binary,
        "binary_sha256": session.binary_sha256,
        "level": level,
        "names": names,
        "renamed_to": renamed_to,
        "comments": comments,
        "notes": notes,
        "steps": matched.len(),
    }))
}

impl Workspace {
    /// How many functions in this binary share each structural hash.
    fn code_copies(&self) -> &BTreeMap<String, usize> {
        self.copies.get_or_init(|| {
            let mut m = BTreeMap::new();
            for f in self.engine.functions().unwrap_or_default() {
                if let Ok(Some(h)) = self.engine.function_hash(f.addr) {
                    *m.entry(h.structural).or_insert(0) += 1;
                }
            }
            m
        })
    }

    pub(crate) fn recall(&self, store: &Store, target: &str) -> Result<Done, SessionError> {
        let addr = self.resolve(target)?;
        let f = self
            .engine
            .function_at(addr)?
            .ok_or_else(|| SessionError::Unresolved(target.into()))?;
        let name = self
            .name_of(f.addr)
            .unwrap_or_else(|| format!("{:#x}", f.addr));
        let function = json!({ "name": name, "addr": f.addr });
        let done = |summary: String, result: Value| Done {
            result,
            summary,
            target: Some(self.target_for(f.addr)),
            addr: Some(f.addr),
        };
        let Some(h) = self.engine.function_hash(f.addr)? else {
            return Ok(done(
                format!("{name} is a stub or too small to match reliably"),
                json!({ "function": function, "matches": [] }),
            ));
        };
        let mut by_session: BTreeMap<String, Vec<(Step, &'static str)>> = BTreeMap::new();
        for s in store.steps_with_fingerprint(&h.structural_key())? {
            if s.session_id == self.session.id {
                continue;
            }
            let other = s
                .target
                .as_ref()
                .and_then(|t| t.func_fp.as_deref())
                .and_then(crosure_engine::FunctionHash::parse);
            let level = match other {
                Some(o) if o.specific == h.specific && h.insns >= MIN_EXACT => "exact",
                Some(_) if h.insns >= MIN_STRUCTURAL => "structural",
                _ => continue,
            };
            by_session
                .entry(s.session_id.clone())
                .or_default()
                .push((s, level));
        }
        let matches = by_session
            .iter()
            .map(|(id, m)| session_match(store, id, m))
            .collect::<Result<Vec<_>, _>>()?;
        let copies = self.code_copies().get(&h.structural).copied().unwrap_or(1);
        // Each session's last word on the function: its latest rename, else
        // the name it had there. Sessions disagreeing is real ambiguity; a
        // rename inside one session is not.
        let names: BTreeSet<&str> = matches
            .iter()
            .filter_map(|m| {
                m["renamed_to"]
                    .as_array()
                    .and_then(|r| r.last())
                    .or_else(|| m["names"].as_array().and_then(|n| n.first()))
                    .and_then(Value::as_str)
            })
            .collect();
        let ambiguous = copies > 1 || names.len() > 1;
        let summary = if matches.is_empty() {
            format!("no earlier session has looked at {name}'s code")
        } else {
            let known: Vec<&str> = names.iter().copied().take(3).collect();
            format!(
                "{} earlier session(s) looked at this code{}{}",
                matches.len(),
                if known.is_empty() {
                    String::new()
                } else {
                    format!(" (as {})", known.join(", "))
                },
                if ambiguous {
                    "; ambiguous: identical code under several names"
                } else {
                    ""
                }
            )
        };
        Ok(done(
            summary,
            json!({
                "function": function,
                "fingerprint": h.fingerprint(),
                "copies_in_binary": copies,
                "ambiguous": ambiguous,
                "matches": matches,
            }),
        ))
    }
}
