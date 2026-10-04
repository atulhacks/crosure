//! 32-bit ARM address formation, the counterpart of `arm64.rs`.
//!
//! - Thumb PIC: `ldr rX, [pc, #imm]` loads an offset from the literal pool,
//!   then `add rX, pc` adds the current address (`pc` reads as insn + 4).
//! - ARM PLT: `add ip, pc, #a, #rot`, `add ip, ip, #b`, `ldr pc, [ip, #c]!`
//!   (`pc` reads as insn + 8).
//! - Non-PIE code: the literal itself is an absolute address.
//!
//! Register contents are tracked in straight-line code and forgotten when a
//! register is overwritten or control leaves the block.

use std::collections::HashMap;

#[derive(Clone, Copy)]
enum Val {
    /// A word read from the literal pool (an offset in PIC code).
    Literal(u64),
    /// A computed absolute address.
    Addr(u64),
}

/// Register contents since the last control transfer.
#[derive(Default)]
pub(crate) struct Arm32Tracker {
    regs: HashMap<String, Val>,
}

fn reg(token: &str) -> Option<String> {
    let t = token.trim().to_lowercase();
    let known = matches!(t.as_str(), "ip" | "lr" | "sb" | "sl" | "fp")
        || t.strip_prefix('r')
            .is_some_and(|n| !n.is_empty() && n.chars().all(|c| c.is_ascii_digit()));
    known.then_some(t)
}

/// `#0xb000`, `#45056`, and the rotated form `#0, #12` (imm ror rot).
fn imm(tokens: &[&str]) -> Option<u64> {
    let one = |t: &str| -> Option<u32> {
        let t = t.trim().trim_start_matches('#');
        match t.strip_prefix("0x") {
            Some(h) => u32::from_str_radix(h, 16).ok(),
            None => t.parse().ok(),
        }
    };
    let v = one(tokens.first()?)?;
    let rot = tokens.get(1).and_then(|r| one(r)).unwrap_or(0);
    Some(u64::from(v.rotate_right(rot)))
}

/// `[base, #off]` (optionally with `!`) -> (base, off).
fn memory(operands: &str) -> Option<(String, u64)> {
    let inner = &operands[operands.find('[')? + 1..operands.find(']')?];
    let mut parts = inner.split(',');
    let base = parts.next()?.trim().to_lowercase();
    let off = match parts.next() {
        Some(o) => imm(&[o])?,
        None => 0,
    };
    Some((base, off))
}

impl Arm32Tracker {
    /// Feeds one instruction at `addr`; returns the absolute address it
    /// refers to, if any. `read` fetches a literal-pool word; `pie` says
    /// literals are offsets, not addresses.
    pub(crate) fn step(
        &mut self,
        addr: u64,
        mnemonic: &str,
        operands: &str,
        thumb: bool,
        pie: bool,
        read: &dyn Fn(u64) -> Option<u32>,
    ) -> Option<u64> {
        let m = mnemonic.trim_end_matches(".w").trim_end_matches(".n");
        let ops: Vec<&str> = operands.split(',').map(str::trim).collect();
        let pc = addr + if thumb { 4 } else { 8 };
        let dest = ops.first().and_then(|d| reg(d));
        let mut resolved = None;
        let mut set: Option<Val> = None;

        if m.starts_with("ldr") && operands.contains("[pc") {
            // Literal pool: [Align(pc, 4) + off].
            if let Some((_, off)) = memory(operands) {
                let word = read((pc & !3) + off).map(u64::from);
                if let Some(w) = word {
                    set = Some(Val::Literal(w));
                    if !pie {
                        resolved = Some(w);
                    }
                }
            }
        } else if m == "add" && ops.len() == 2 && ops[1] == "pc" {
            // Thumb `add rX, pc`: literal offset + pc.
            if let Some(Val::Literal(v)) = dest.as_ref().and_then(|d| self.regs.get(d)) {
                let a = v.wrapping_add(pc) & 0xffff_ffff;
                resolved = Some(a);
                set = Some(Val::Addr(a));
            }
        } else if m == "add" && ops.len() >= 3 {
            let base = match ops[1] {
                "pc" => Some(pc),
                r => match reg(r).and_then(|r| self.regs.get(&r).copied()) {
                    Some(Val::Addr(a)) => Some(a),
                    _ => None,
                },
            };
            if let (Some(b), Some(i)) = (base, imm(&ops[2..])) {
                let a = (b + i) & 0xffff_ffff;
                resolved = Some(a);
                set = Some(Val::Addr(a));
            }
        } else if m.starts_with("ldr") || m.starts_with("str") {
            if let Some((base, off)) = memory(operands) {
                if let Some(Val::Addr(a)) = self.regs.get(&base) {
                    resolved = Some((a + off) & 0xffff_ffff);
                }
            }
        }

        let leaves = matches!(m, "b" | "bl" | "blx" | "bx")
            || (m.starts_with("pop") && operands.contains("pc"))
            || ops.first() == Some(&"pc");
        if leaves {
            self.regs.clear();
        } else if let Some(d) = dest.filter(|_| !m.starts_with("str") && !m.starts_with("cmp")) {
            match set {
                Some(v) => {
                    self.regs.insert(d, v);
                }
                None => {
                    self.regs.remove(&d);
                }
            }
        }
        resolved
    }
}

#[cfg(test)]
mod tests {
    use super::Arm32Tracker;

    #[test]
    fn arm_plt_slot_is_computed() {
        let mut t = Arm32Tracker::default();
        let none = |_: u64| None;
        assert_eq!(
            t.step(0xcf4, "add", "ip, pc, #0, #12", false, true, &none),
            Some(0xcfc)
        );
        assert_eq!(
            t.step(0xcf8, "add", "ip, ip, #0xb000", false, true, &none),
            Some(0xbcfc)
        );
        assert_eq!(
            t.step(0xcfc, "ldr", "pc, [ip, #0x1f8]!", false, true, &none),
            Some(0xbef4)
        );
    }

    #[test]
    fn thumb_literal_plus_pc() {
        let mut t = Arm32Tracker::default();
        // literal at Align(0x1000 + 4, 4) + 8 = 0x100c holds 0x2000
        let read = |a: u64| (a == 0x100c).then_some(0x2000u32);
        assert_eq!(
            t.step(0x1000, "ldr", "r3, [pc, #8]", true, true, &read),
            None,
            "PIE: an offset"
        );
        assert_eq!(
            t.step(0x1002, "add", "r3, pc", true, true, &read),
            Some(0x3006)
        );
        assert_eq!(t.step(0x1004, "bl", "#0x2000", true, true, &read), None);
        assert_eq!(
            t.step(0x1008, "add", "r3, pc", true, true, &read),
            None,
            "calls clobber"
        );
        // Non-PIE: the literal is the address.
        assert_eq!(
            t.step(0x1000, "ldr", "r0, [pc, #8]", true, false, &read),
            Some(0x2000)
        );
    }
}
