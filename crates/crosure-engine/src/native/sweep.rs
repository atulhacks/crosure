use std::collections::{BTreeMap, BTreeSet};

use super::arm64::AdrpTracker;
use super::disasm::{base_mnemonic, capstone_for, decode};
use super::load::Loaded;
use crate::operand::{immediates, is_call, is_jump, parse_branch_target, parse_rip_relative};
use crate::{EngineError, FunctionInfo, SectionInfo, Xref, XrefKind};

/// Results of one linear sweep over every executable section.
pub(crate) struct Analysis {
    pub functions: Vec<FunctionInfo>,
    /// Sorted by `to`, then `from`.
    pub xrefs: Vec<Xref>,
    /// Every address we can name: functions, import stubs, import slots.
    pub names: BTreeMap<u64, String>,
}

struct Sweep {
    xrefs: Vec<Xref>,
    call_targets: BTreeSet<u64>,
    code_pointers: BTreeSet<u64>,
    stubs: BTreeMap<u64, String>,
    /// Alignment padding runs (nop / int3), keyed by end address -> start.
    padding: BTreeMap<u64, u64>,
}

pub(crate) fn exec_contains(sections: &[SectionInfo], addr: u64) -> Option<&SectionInfo> {
    sections
        .iter()
        .find(|s| s.executable && addr >= s.addr && addr < s.addr + s.size)
}

fn mapped(sections: &[SectionInfo], addr: u64) -> bool {
    sections
        .iter()
        .any(|s| s.addr != 0 && addr >= s.addr && addr < s.addr + s.size)
}

/// Sweeps the code, collects xrefs, and discovers functions.
pub(crate) fn analyze(data: &[u8], l: &Loaded) -> Result<Analysis, EngineError> {
    let info = &l.info;
    let cs = capstone_for(&info.arch, info.bits)?;
    let mut sw = Sweep {
        xrefs: Vec::new(),
        call_targets: BTreeSet::new(),
        code_pointers: BTreeSet::new(),
        stubs: BTreeMap::new(),
        padding: BTreeMap::new(),
    };
    for sec in info.sections.iter().filter(|s| s.executable) {
        let Some(off) = sec.file_offset else { continue };
        let start = off as usize;
        let end = (start + sec.size as usize).min(data.len());
        if start >= end {
            continue;
        }
        let insns = decode(&cs, &data[start..end], sec.addr, usize::MAX)?;
        let mut stub_start: Option<u64> = Some(sec.addr);
        let mut pad_start: Option<u64> = None;
        let mut adrp = AdrpTracker::default();
        let arm64 = info.arch == "aarch64";
        let plt = sec.name.starts_with(".plt");
        for insn in &insns {
            let m = base_mnemonic(&insn.mnemonic);
            let next_ip = insn.addr + (insn.bytes.len() / 2) as u64;
            if matches!(m, "nop" | "int3") {
                let start = *pad_start.get_or_insert(insn.addr);
                sw.padding.insert(next_ip, start);
            } else {
                pad_start = None;
            }
            let paired = if arm64 {
                adrp.step(m, &insn.operands)
            } else {
                None
            };
            visit(
                &mut sw,
                l,
                Insn {
                    from: insn.addr,
                    m,
                    ops: &insn.operands,
                    next_ip,
                    paired,
                    plt,
                },
                stub_start,
            );
            // An AArch64 stub starts with the `adrp` of its slot's page.
            stub_start = stub_boundary(m, insn.addr, next_ip).or(if m == "adrp" {
                stub_start
            } else {
                None
            });
        }
    }
    Ok(finish(sw, l))
}

/// Where an import stub could start after this instruction: right here
/// (after `endbr`), or at the next instruction (after flow cannot fall
/// through). A `jmp [slot]` anywhere else is a tail call, not a stub.
fn stub_boundary(m: &str, addr: u64, next_ip: u64) -> Option<u64> {
    if m.starts_with("endbr") {
        Some(addr)
    } else if matches!(
        m,
        "ret" | "jmp" | "int3" | "nop" | "ud2" | "hlt" | "br" | "b"
    ) {
        Some(next_ip)
    } else {
        None
    }
}

/// One decoded instruction, as the sweep sees it.
struct Insn<'a> {
    from: u64,
    m: &'a str,
    ops: &'a str,
    next_ip: u64,
    /// AArch64: the address an earlier `adrp` makes this instruction refer to.
    paired: Option<u64>,
    /// The instruction is in a `.plt*` section.
    plt: bool,
}

fn visit(sw: &mut Sweep, l: &Loaded, i: Insn<'_>, stub_start: Option<u64>) {
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
        // Immediates on AArch64 are pages and offsets; only paired ones count.
        "aarch64" => i
            .paired
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

fn finish(sw: Sweep, l: &Loaded) -> Analysis {
    let sections = &l.info.sections;
    let mut cands: BTreeMap<u64, (String, u64, &'static str)> = BTreeMap::new();
    for (addr, name) in &sw.stubs {
        cands.insert(*addr, (format!("{name}@plt"), 0, "import_stub"));
    }
    for (addr, (name, size)) in &l.symbols {
        cands.insert(*addr, (name.clone(), *size, "symbol"));
    }
    for (addr, name) in &l.exports {
        cands.entry(*addr).or_insert((name.clone(), 0, "export"));
    }
    if l.info.entry != 0 {
        cands
            .entry(l.info.entry)
            .or_insert(("entry".into(), 0, "entry"));
    }
    // `.plt` has an FDE too; its stubs are imports, not one function.
    let in_plt = |a: u64| exec_contains(sections, a).is_some_and(|s| s.name.starts_with(".plt"));
    for addr in l.unwind.keys().filter(|a| !in_plt(**a)) {
        cands
            .entry(*addr)
            .or_insert((format!("sub_{addr:x}"), 0, "unwind"));
    }
    for t in &l.relocated {
        cands
            .entry(*t)
            .or_insert((format!("sub_{t:x}"), 0, "data_pointer"));
    }
    for (set, source) in [
        (&sw.call_targets, "call_target"),
        (&sw.code_pointers, "code_pointer"),
    ] {
        for t in set.iter() {
            cands.entry(*t).or_insert((format!("sub_{t:x}"), 0, source));
        }
    }
    // An inferred start strictly inside an unwind range is a label (switch
    // case, `call $+5`), not a function.
    let inside_unwind = |t: u64| {
        l.unwind
            .range(..t)
            .next_back()
            .is_some_and(|(s, n)| t < s + n)
    };
    cands.retain(|addr, (_, _, source)| {
        exec_contains(sections, *addr).is_some()
            && !(matches!(*source, "call_target" | "code_pointer" | "data_pointer")
                && inside_unwind(*addr))
    });

    let starts: Vec<u64> = cands.keys().copied().collect();
    let mut functions = Vec::with_capacity(cands.len());
    for (i, (addr, (name, size, source))) in cands.into_iter().enumerate() {
        let sec_end = exec_contains(sections, addr).map_or(addr, |s| s.addr + s.size);
        let next = starts.get(i + 1).copied().unwrap_or(sec_end).min(sec_end);
        let size = if size > 0 {
            size
        } else if let Some(n) = l.unwind.get(&addr) {
            *n
        } else {
            // Up to the next start, minus the alignment padding before it.
            let end = match sw.padding.get(&next) {
                Some(pad) if *pad > addr => *pad,
                _ => next,
            };
            end.saturating_sub(addr)
        };
        functions.push(FunctionInfo {
            addr,
            name,
            size,
            source: source.into(),
        });
    }

    let mut names: BTreeMap<u64, String> = l.import_slots.clone();
    for f in &functions {
        names.insert(f.addr, f.name.clone());
    }
    let mut xrefs = sw.xrefs;
    for x in &mut xrefs {
        x.from_func = containing(&functions, x.from).map(|f| f.name.clone());
    }
    xrefs.sort_by_key(|x| (x.to, x.from));
    Analysis {
        functions,
        xrefs,
        names,
    }
}

/// The function whose `[addr, addr + size)` contains `addr`.
pub(crate) fn containing(functions: &[FunctionInfo], addr: u64) -> Option<&FunctionInfo> {
    let idx = functions.partition_point(|f| f.addr <= addr);
    let f = functions.get(idx.checked_sub(1)?)?;
    (addr < f.addr + f.size.max(1)).then_some(f)
}
