//! Function starts found from layout, for code without unwind tables.
//!
//! Compilers end a function with a terminator (`ret`, `jmp`, `ud2`, or a
//! call that never returns) and align the next one with padding. So an
//! instruction after a terminator and padding starts a function, unless a
//! conditional branch targets it (a loop head or cold block of the same
//! function). This finds functions nothing calls (exported but unused, dead
//! code) and tail-call targets, without prologue byte patterns.

use std::collections::BTreeSet;

/// Functions that never return (after r2's `types-*.sdb` lists), matched on
/// the bare name (`exit`, `exit@plt`, `__imp_ExitProcess`).
const NORETURN: &[&str] = &[
    "exit",
    "_exit",
    "_Exit",
    "quick_exit",
    "abort",
    "__assert_fail",
    "__assert_rtn",
    "__stack_chk_fail",
    "__fortify_fail",
    "__chk_fail",
    "__cxa_throw",
    "__cxa_rethrow",
    "_Unwind_Resume",
    "err",
    "errx",
    "verr",
    "verrx",
    "longjmp",
    "siglongjmp",
    "_longjmp",
    "__longjmp_chk",
    "pthread_exit",
    "thrd_exit",
    "xalloc_die",
    "ExitProcess",
    "ExitThread",
    "FatalExit",
    "FatalAppExitA",
    "FatalAppExitW",
];

/// Whether `name` (possibly decorated) is a function that never returns.
pub(crate) fn is_noreturn(name: &str) -> bool {
    let bare = name
        .split('@')
        .next()
        .unwrap_or(name)
        .trim_start_matches("__imp_");
    NORETURN.contains(&bare)
}

fn terminates(m: &str, ops: &str) -> bool {
    matches!(m, "ret" | "retn" | "jmp" | "ud2" | "hlt" | "b" | "b.w")
        || (m == "bx" && ops.trim() == "lr")
        || (m.starts_with("pop") && ops.contains("pc"))
}

/// Tracks terminator + padding sequences over one section's instructions.
#[derive(Default)]
pub(crate) struct Boundaries {
    after_terminator: bool,
    padded: bool,
    /// Addresses that follow a terminator and padding.
    pub(crate) found: BTreeSet<u64>,
}

impl Boundaries {
    /// Feeds one instruction. `pad` marks alignment padding; `callee` names
    /// the function a call goes to, when known.
    pub(crate) fn step(&mut self, addr: u64, m: &str, ops: &str, pad: bool, callee: Option<&str>) {
        if pad {
            self.padded |= self.after_terminator;
            return;
        }
        if self.after_terminator && self.padded {
            self.found.insert(addr);
        }
        let noreturn_call = callee.is_some_and(is_noreturn);
        self.after_terminator = terminates(m, ops) || noreturn_call;
        self.padded = false;
    }
}

#[cfg(test)]
mod tests {
    use super::{is_noreturn, Boundaries};

    #[test]
    fn noreturn_names_are_matched_bare() {
        assert!(is_noreturn("__stack_chk_fail@plt"));
        assert!(is_noreturn("__imp_ExitProcess"));
        assert!(!is_noreturn("printf@plt"));
    }

    #[test]
    fn a_start_needs_a_terminator_then_padding() {
        let mut b = Boundaries::default();
        b.step(0x10, "ret", "", false, None);
        b.step(0x11, "nop", "", true, None);
        b.step(0x20, "push", "rbp", false, None);
        // No padding after this ret: the next block may belong to it.
        b.step(0x30, "ret", "", false, None);
        b.step(0x31, "mov", "eax, 1", false, None);
        b.step(0x40, "call", "0x1000", false, Some("exit@plt"));
        b.step(0x45, "nop", "", true, None);
        b.step(0x50, "push", "rbp", false, None);
        assert_eq!(b.found.into_iter().collect::<Vec<_>>(), vec![0x20, 0x50]);
    }
}
