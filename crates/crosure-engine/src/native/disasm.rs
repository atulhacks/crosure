use capstone::prelude::*;

use crate::{EngineError, Instruction};

/// Builds a Capstone handle for the binary's architecture (Intel syntax on x86).
pub(crate) fn capstone_for(arch: &str, bits: u8) -> Result<Capstone, EngineError> {
    let cs = match arch {
        "x86_64" | "x86" => Capstone::new()
            .x86()
            .mode(if bits == 64 {
                arch::x86::ArchMode::Mode64
            } else {
                arch::x86::ArchMode::Mode32
            })
            .syntax(arch::x86::ArchSyntax::Intel)
            .build(),
        "aarch64" => Capstone::new()
            .arm64()
            .mode(arch::arm64::ArchMode::Arm)
            .build(),
        "arm" => Capstone::new().arm().mode(arch::arm::ArchMode::Arm).build(),
        other => return Err(EngineError::UnsupportedArch(other.to_string())),
    };
    cs.map_err(|e| EngineError::Disasm(e.to_string()))
}

/// Strips x86 prefixes Capstone folds into the mnemonic (`bnd jmp` -> `jmp`).
pub(crate) fn base_mnemonic(m: &str) -> &str {
    m.trim_start_matches("bnd ").trim_start_matches("notrack ")
}

/// Linear decode of `code` at `addr`. Undecodable bytes become `(bad)` and
/// decoding resumes at the next byte, so a sweep never stops early.
/// Stops after `limit` instructions.
pub(crate) fn decode(
    cs: &Capstone,
    code: &[u8],
    addr: u64,
    limit: usize,
) -> Result<Vec<Instruction>, EngineError> {
    let mut out = Vec::new();
    let mut offset = 0usize;
    while offset < code.len() && out.len() < limit {
        let chunk = &code[offset..];
        let at = addr + offset as u64;
        let insns = cs
            .disasm_count(chunk, at, (limit - out.len()).min(4096))
            .map_err(|e| EngineError::Disasm(e.to_string()))?;
        if insns.is_empty() {
            out.push(Instruction {
                addr: at,
                bytes: format!("{:02x}", chunk[0]),
                mnemonic: "(bad)".into(),
                operands: String::new(),
                target: None,
                comment: None,
            });
            offset += 1;
            continue;
        }
        for i in insns.iter() {
            out.push(Instruction {
                addr: i.address(),
                bytes: i.bytes().iter().map(|b| format!("{b:02x}")).collect(),
                mnemonic: i.mnemonic().unwrap_or("?").to_string(),
                operands: i.op_str().unwrap_or("").to_string(),
                target: None,
                comment: None,
            });
            offset += i.bytes().len();
        }
    }
    Ok(out)
}
