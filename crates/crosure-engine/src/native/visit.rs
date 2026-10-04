//! What one instruction references: branch targets, RIP-relative and
//! AArch64/ARM paired addresses, import stubs and code pointers.

use super::load::Loaded;
use super::sweep::{exec_contains, Sweep};
use crate::operand::{immediates, is_call, is_jump, parse_branch_target, parse_rip_relative};
use crate::{SectionInfo, Xref, XrefKind};

/// Bit 0 of a 32-bit ARM code pointer selects Thumb; the address is even.
fn thumb_bit(sections: &[SectionInfo], arm: bool, t: u64) -> u64 {
    if arm && exec_contains(sections, t & !1).is_some() {
        t & !1
    } else {
        t
    }
}

fn mapped(sections: &[SectionInfo], addr: u64) -> bool {
    sections
        .iter()
        .any(|s| s.addr != 0 && addr >= s.addr && addr < s.addr + s.size)
}

/// One decoded instruction, as the sweep sees it.
pub(crate) struct Insn<'a> {
    pub(crate) from: u64,
    pub(crate) m: &'a str,
    pub(crate) ops: &'a str,
    pub(crate) next_ip: u64,
    /// AArch64: the address an earlier `adrp` makes this instruction refer to.
    pub(crate) paired: Option<u64>,
    /// The instruction is in a `.plt*` section.
    pub(crate) plt: bool,
}

pub(crate) fn visit(sw: &mut Sweep, l: &Loaded, i: Insn<'_>, stub_start: Option<u64>) {
    let sections = &l.info.sections;
    let (from, m) = (i.from, i.m);
    let branch_kind = if is_call(m) {
        Some(XrefKind::Call)
    } else if is_jump(m) {
        Some(XrefKind::Jump)
    } else {
        None
    };
    if let (Some(kind), Some(t)) = (branch_kind, parse_branch_target(i.ops)) {
        sw.xrefs.push(Xref {
            from,
            to: t,
            kind,
            from_func: None,
        });
        if kind == XrefKind::Call {
            sw.call_targets.insert(t);
        }
        return;
    }
    let memory: Vec<u64> = match l.info.arch.as_str() {
        "x86_64" => parse_rip_relative(i.ops, i.next_ip).into_iter().collect(),
        // On ARM, immediates are pages, offsets and literals: only resolved
        // addresses count, and in a PLT only the slot load is a reference.
        "aarch64" | "arm" if i.plt && !m.starts_with("ldr") => Vec::new(),
        "aarch64" | "arm" => i
            .paired
            .map(|t| thumb_bit(sections, l.info.arch == "arm", t))
            .filter(|t| mapped(sections, *t))
            .into_iter()
            .collect(),
        _ => immediates(i.ops)
            .into_iter()
            .filter(|t| mapped(sections, *t))
            .collect(),
    };
    for t in memory {
        let slot = l.import_slots.get(&t);
        let kind = match (branch_kind, slot) {
            (Some(k), _) => k,
            _ => XrefKind::Data,
        };
        // x86: `jmp [slot]`. AArch64 PLT: `ldr x17, [x16, #slot]` then `br x17`.
        let loads_slot = branch_kind == Some(XrefKind::Jump) || (i.plt && m.starts_with("ldr"));
        if let (true, Some(name), Some(start)) = (loads_slot, slot, stub_start) {
            sw.stubs.insert(start, name.clone());
        }
        let address_of = m == "lea" || (m == "add" && i.paired.is_some());
        if kind == XrefKind::Data && address_of && exec_contains(sections, t).is_some() {
            sw.code_pointers.insert(t);
        }
        sw.xrefs.push(Xref {
            from,
            to: t,
            kind,
            from_func: None,
        });
    }
}
