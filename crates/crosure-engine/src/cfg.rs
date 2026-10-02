use std::collections::BTreeSet;

use crate::operand::{is_call, is_jump, parse_branch_target};
use crate::{BasicBlock, BlockEdge, EdgeKind, Instruction};

fn base(m: &str) -> &str {
    m.trim_start_matches("bnd ").trim_start_matches("notrack ")
}

fn is_unconditional(m: &str) -> bool {
    matches!(m, "jmp" | "b" | "bx" | "br")
}

fn is_return(m: &str) -> bool {
    matches!(
        m,
        "ret" | "retn" | "retf" | "iret" | "iretq" | "hlt" | "ud2"
    )
}

fn next_addr(i: &Instruction) -> u64 {
    i.addr + (i.bytes.len() / 2) as u64
}

/// Splits a function's linear disassembly into basic blocks with typed
/// successor edges. Branches leaving the function (tail calls) get no edge.
///
/// ```
/// use crosure_engine::{build_cfg, EdgeKind, Instruction};
/// let insn = |addr: u64, bytes: &str, m: &str, ops: &str| Instruction {
///     addr, bytes: bytes.into(), mnemonic: m.into(), operands: ops.into(), target: None, comment: None,
/// };
/// let code = vec![
///     insn(0x10, "85c0", "test", "eax, eax"),
///     insn(0x12, "7402", "je", "0x16"),
///     insn(0x14, "31c0", "xor", "eax, eax"),
///     insn(0x16, "c3", "ret", ""),
/// ];
/// let blocks = build_cfg(&code, 0x10, 0x17);
/// assert_eq!(blocks.len(), 3);
/// assert_eq!(blocks[0].succs[0].kind, EdgeKind::Taken);
/// assert_eq!(blocks[1].succs[0].kind, EdgeKind::Fall);
/// assert!(blocks[2].succs.is_empty());
/// ```
pub fn build_cfg(insns: &[Instruction], start: u64, end: u64) -> Vec<BasicBlock> {
    let inside = |a: u64| a >= start && a < end;
    let mut leaders: BTreeSet<u64> = BTreeSet::new();
    if let Some(first) = insns.first() {
        leaders.insert(first.addr);
    }
    for i in insns {
        let m = base(&i.mnemonic);
        if is_jump(m) && !is_call(m) {
            if let Some(t) = parse_branch_target(&i.operands).filter(|t| inside(*t)) {
                leaders.insert(t);
            }
            leaders.insert(next_addr(i));
        } else if is_return(m) {
            leaders.insert(next_addr(i));
        }
    }
    let mut blocks: Vec<BasicBlock> = Vec::new();
    for (idx, i) in insns.iter().enumerate() {
        if leaders.contains(&i.addr) || blocks.is_empty() {
            blocks.push(BasicBlock {
                addr: i.addr,
                end: i.addr,
                first: idx,
                count: 0,
                succs: Vec::new(),
            });
        }
        if let Some(b) = blocks.last_mut() {
            b.count += 1;
            b.end = next_addr(i);
        }
    }
    for b in &mut blocks {
        let Some(last) = insns.get(b.first + b.count - 1) else {
            continue;
        };
        let m = base(&last.mnemonic);
        let target = parse_branch_target(&last.operands).filter(|t| inside(*t));
        let fall = Some(b.end).filter(|a| inside(*a) && leaders.contains(a));
        let mut push = |to: Option<u64>, kind| {
            if let Some(to) = to {
                b.succs.push(BlockEdge { to, kind });
            }
        };
        if is_return(m) {
            continue;
        }
        if is_jump(m) && is_unconditional(m) {
            push(target, EdgeKind::Jump);
        } else if is_jump(m) {
            push(target, EdgeKind::Taken);
            push(fall, EdgeKind::Fall);
        } else {
            push(fall, EdgeKind::Fall);
        }
    }
    blocks
}
