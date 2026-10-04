//! AArch64 address formation. Code builds an address in two instructions,
//! `adrp xN, page` then `add xD, xN, #off` or `ldr/str xD, [xN, #off]`, so
//! neither instruction alone says what is referenced. This tracks the page
//! held by each register within straight-line code (after r2's `anal/fcn.c`).

use std::collections::HashMap;

/// Pages loaded by `adrp`, per register, since the last control transfer.
#[derive(Default)]
pub(crate) struct AdrpTracker {
    pages: HashMap<String, u64>,
}

/// `w5` and `x5` are the same register.
fn reg(token: &str) -> Option<String> {
    let t = token.trim();
    let rest = t.strip_prefix('x').or_else(|| t.strip_prefix('w'))?;
    (!rest.is_empty() && rest.chars().all(|c| c.is_ascii_digit())).then(|| format!("x{rest}"))
}

fn imm(token: &str) -> Option<u64> {
    let t = token.trim().trim_start_matches('#');
    match t.strip_prefix("0x") {
        Some(h) => u64::from_str_radix(h, 16).ok(),
        None => t.parse().ok(),
    }
}

/// `[base]`, `[base, #off]`, `[base, #off]!` -> (base, off).
fn memory(operands: &str) -> Option<(String, u64)> {
    let inner = &operands[operands.find('[')? + 1..operands.find(']')?];
    let mut parts = inner.split(',');
    let base = reg(parts.next()?)?;
    let off = match parts.next() {
        Some(o) => imm(o)?,
        None => 0,
    };
    Some((base, off))
}

impl AdrpTracker {
    /// Feeds one instruction; returns the absolute address it refers to
    /// through an earlier `adrp`, if any (see the tests below).
    pub(crate) fn step(&mut self, mnemonic: &str, operands: &str) -> Option<u64> {
        let first = operands.split(',').next().unwrap_or("");
        if mnemonic == "adrp" {
            let page = operands.split(',').nth(1).and_then(imm);
            if let (Some(r), Some(p)) = (reg(first), page) {
                self.pages.insert(r, p);
            }
            return None;
        }
        let resolved = if mnemonic == "add" && !operands.contains("lsl") {
            let mut ops = operands.split(',').skip(1);
            match (ops.next().and_then(reg), ops.next().and_then(imm)) {
                (Some(base), Some(off)) => self.pages.get(&base).map(|p| p + off),
                _ => None,
            }
        } else if mnemonic.starts_with("ld") || mnemonic.starts_with("st") {
            memory(operands).and_then(|(base, off)| self.pages.get(&base).map(|p| p + off))
        } else {
            None
        };
        if matches!(mnemonic, "ret" | "b" | "br" | "blr" | "bl" | "eret") {
            // Calls clobber caller-saved registers; jumps leave this block.
            self.pages.clear();
        } else if !mnemonic.starts_with("st") && !mnemonic.starts_with("cmp") {
            // The destination now holds something else.
            if let Some(d) = reg(first) {
                self.pages.remove(&d);
            }
            if mnemonic == "ldp" {
                if let Some(d2) = operands.split(',').nth(1).and_then(reg) {
                    self.pages.remove(&d2);
                }
            }
        }
        resolved
    }
}

#[cfg(test)]
mod tests {
    use super::AdrpTracker;

    #[test]
    fn pairs_adrp_with_its_consumers() {
        let mut t = AdrpTracker::default();
        assert_eq!(t.step("adrp", "x16, #0x1f000"), None);
        assert_eq!(t.step("ldr", "x17, [x16, #0xde8]"), Some(0x1fde8));
        assert_eq!(t.step("add", "x16, x16, #0xde8"), Some(0x1fde8));
        // x16 now holds the full address, not the page.
        assert_eq!(t.step("ldr", "x1, [x16, #8]"), None);
        assert_eq!(t.step("adrp", "x0, #0x20000"), None);
        assert_eq!(t.step("add", "w20, w0, #0x40"), Some(0x20040));
        assert_eq!(t.step("ldrb", "w1, [x0]"), Some(0x20000));
        assert_eq!(t.step("bl", "#0x1700"), None);
        assert_eq!(t.step("add", "x1, x0, #0x10"), None, "calls clobber");
    }
}
