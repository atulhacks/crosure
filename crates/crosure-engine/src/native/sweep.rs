use std::collections::{BTreeMap, BTreeSet};

use super::arm32::Arm32Tracker;
use super::arm64::AdrpTracker;
use super::boundary::Boundaries;
use super::disasm::{base_mnemonic, decode, decoder_for};
use super::jumptable::{cases, TableFinder};
use super::load::Loaded;
use super::visit::{visit, Insn};
use crate::operand::{is_call, parse_branch_target, parse_rip_relative};
use crate::{EngineError, FunctionInfo, SectionInfo, Xref};

/// Results of one linear sweep over every executable section.
pub(crate) struct Analysis {
    pub functions: Vec<FunctionInfo>,
    /// Sorted by `to`, then `from`.
    pub xrefs: Vec<Xref>,
    /// Every address we can name: functions, import stubs, import slots.
    pub names: BTreeMap<u64, String>,
}

pub(crate) struct Sweep {
    pub(crate) xrefs: Vec<Xref>,
    pub(crate) call_targets: BTreeSet<u64>,
    pub(crate) code_pointers: BTreeSet<u64>,
    pub(crate) stubs: BTreeMap<u64, String>,
    /// Alignment padding runs (nop / int3), keyed by end address -> start.
    pub(crate) padding: BTreeMap<u64, u64>,
    /// Targets of conditional branches (always inside a function).
    pub(crate) cond_targets: BTreeSet<u64>,
    /// Addresses after a terminator and padding (see `boundary.rs`).
    pub(crate) boundaries: BTreeSet<u64>,
}

pub(crate) fn exec_contains(sections: &[SectionInfo], addr: u64) -> Option<&SectionInfo> {
    sections
        .iter()
        .find(|s| s.executable && addr >= s.addr && addr < s.addr + s.size)
}

/// Sweeps the code, collects xrefs, and discovers functions.
pub(crate) fn analyze(data: &[u8], l: &Loaded) -> Result<Analysis, EngineError> {
    let info = &l.info;
    let arm_mode = decoder_for(&info.arch, info.bits, false)?;
    let thumb_mode = decoder_for(&info.arch, info.bits, l.thumb)?;
    let mut sw = Sweep {
        xrefs: Vec::new(),
        call_targets: BTreeSet::new(),
        code_pointers: BTreeSet::new(),
        stubs: BTreeMap::new(),
        padding: BTreeMap::new(),
        cond_targets: BTreeSet::new(),
        boundaries: BTreeSet::new(),
    };
    let x86 = matches!(info.arch.as_str(), "x86_64" | "x86");
    let mut found_tables = Vec::new();
    for sec in info.sections.iter().filter(|s| s.executable) {
        let Some(off) = sec.file_offset else { continue };
        let start = off as usize;
        let end = (start + sec.size as usize).min(data.len());
        if start >= end {
            continue;
        }
        let d = if l.thumb_in(&sec.name) {
            &thumb_mode
        } else {
            &arm_mode
        };
        let insns = decode(d, &data[start..end], sec.addr, usize::MAX)?;
        let mut stub_start: Option<u64> = Some(sec.addr);
        let mut pad_start: Option<u64> = None;
        let mut adrp = AdrpTracker::default();
        let mut arm32 = Arm32Tracker::default();
        let arm64 = info.arch == "aarch64";
        let arm = info.arch == "arm";
        let thumb = l.thumb_in(&sec.name);
        let read = |a: u64| l.word(data, a);
        let plt = sec.name.starts_with(".plt");
        let mut bounds = Boundaries::default();
        let mut tables = TableFinder::default();
        for insn in &insns {
            let m = base_mnemonic(&insn.mnemonic);
            let next_ip = insn.addr + (insn.bytes.len() / 2) as u64;
            let pad = matches!(m, "nop" | "int3");
            if pad {
                let start = *pad_start.get_or_insert(insn.addr);
                sw.padding.insert(next_ip, start);
            } else {
                pad_start = None;
            }
            let callee = is_call(m)
                .then(|| {
                    parse_branch_target(&insn.operands)
                        .and_then(|t| sw.stubs.get(&t).or_else(|| l.symbols.get(&t).map(|s| &s.0)))
                        .or_else(|| {
                            parse_rip_relative(&insn.operands, next_ip)
                                .and_then(|t| l.import_slots.get(&t))
                        })
                })
                .flatten();
            if !plt && x86 {
                let rip = parse_rip_relative(&insn.operands, next_ip);
                found_tables.extend(tables.step(m, &insn.operands, rip));
                bounds.step(
                    insn.addr,
                    m,
                    &insn.operands,
                    pad,
                    callee.map(String::as_str),
                );
            }
            let paired = if arm64 {
                adrp.step(m, &insn.operands)
            } else if arm {
                arm32.step(insn.addr, m, &insn.operands, thumb, l.pie, &read)
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
            // A stub starts with the instructions that build its slot address
            // (AArch64 `adrp`, ARM `add ip, ...`); `ldr pc, ...` ends it.
            let builds_slot = m == "adrp" || (plt && m == "add");
            // An ARM PLT entry always opens with `add ip, pc, ...`.
            let opens_entry = plt && m == "add" && insn.operands.contains(", pc");
            stub_start = stub_boundary(m, insn.addr, next_ip)
                .or(insn.operands.starts_with("pc,").then_some(next_ip))
                .or(if opens_entry {
                    Some(insn.addr)
                } else if builds_slot {
                    stub_start
                } else {
                    None
                });
        }
        sw.boundaries.extend(bounds.found);
    }
    // Switch cases are reached only through their table: mark them as
    // inside a function so the boundary rule does not split it there.
    let read = |a: u64, n: usize| {
        let s = info
            .sections
            .iter()
            .find(|s| a >= s.addr && a + n as u64 <= s.addr + s.size)?;
        let at = usize::try_from(s.file_offset? + (a - s.addr)).ok()?;
        data.get(at..at + n).map(<[u8]>::to_vec)
    };
    let is_code = |a: u64| exec_contains(&info.sections, a).is_some();
    for t in found_tables {
        sw.cond_targets.extend(cases(t, &read, &is_code));
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
    for b in sw
        .boundaries
        .iter()
        .filter(|b| !sw.cond_targets.contains(b))
    {
        cands
            .entry(*b)
            .or_insert((format!("sub_{b:x}"), 0, "boundary"));
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
            && !(matches!(
                *source,
                "call_target" | "code_pointer" | "data_pointer" | "boundary"
            ) && inside_unwind(*addr))
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
