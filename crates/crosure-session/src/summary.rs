use crosure_engine::{Instruction, Xref, XrefKind};

/// Joins up to `max` items, adding `+N more`.
///
/// ```ignore
/// assert_eq!(list(&["a".into(), "b".into(), "c".into()], 2), "a, b +1 more");
/// ```
pub(crate) fn list(items: &[String], max: usize) -> String {
    let mut out = items
        .iter()
        .take(max)
        .cloned()
        .collect::<Vec<_>>()
        .join(", ");
    if items.len() > max {
        out.push_str(&format!(" +{} more", items.len() - max));
    }
    out
}

fn dedup(mut v: Vec<String>) -> Vec<String> {
    let mut seen = std::collections::HashSet::new();
    v.retain(|x| seen.insert(x.clone()));
    v
}

/// "22 instructions; calls decode, strcmp@plt; uses \"Wrong password\"".
pub(crate) fn disasm(name: &str, insns: &[Instruction]) -> String {
    let calls = dedup(
        insns
            .iter()
            .filter(|i| i.mnemonic.contains("call"))
            .filter_map(|i| i.comment.clone())
            .collect(),
    );
    let strings = dedup(
        insns
            .iter()
            .filter_map(|i| i.comment.clone())
            .filter(|c| c.starts_with('"'))
            .collect(),
    );
    let mut s = format!("{name}: {} instructions", insns.len());
    if !calls.is_empty() {
        s.push_str(&format!("; calls {}", list(&calls, 4)));
    }
    if !strings.is_empty() {
        s.push_str(&format!("; uses {}", list(&strings, 2)));
    }
    s
}

/// "3 refs (2 call, 1 data) from main, check_password".
pub(crate) fn xrefs(what: &str, refs: &[Xref]) -> String {
    if refs.is_empty() {
        return format!("no references to {what}");
    }
    let count = |k: XrefKind| refs.iter().filter(|r| r.kind == k).count();
    let funcs = dedup(
        refs.iter()
            .map(|r| {
                r.from_func
                    .clone()
                    .unwrap_or_else(|| format!("{:#x}", r.from))
            })
            .collect(),
    );
    format!(
        "{} {} to {what} ({} call, {} jump, {} data) from {}",
        refs.len(),
        if refs.len() == 1 { "ref" } else { "refs" },
        count(XrefKind::Call),
        count(XrefKind::Jump),
        count(XrefKind::Data),
        list(&funcs, 4)
    )
}
