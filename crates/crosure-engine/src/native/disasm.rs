use capstone::prelude::*;

use crate::{EngineError, Instruction};

/// A Capstone handle and how far to skip past undecodable bytes (the
/// instruction alignment), so decoding resumes on a real boundary.
pub(crate) struct Decoder {
    cs: Capstone,
    skip: usize,
}

/// A decoder for the binary's architecture (Intel syntax on x86). `thumb`
/// selects Thumb-2 on 32-bit ARM.
pub(crate) fn decoder_for(arch: &str, bits: u8, thumb: bool) -> Result<Decoder, EngineError> {
    let skip = match arch {
        "x86_64" | "x86" => 1,
        "arm" if thumb => 2,
        _ => 4,
    };
    let cs = capstone_for(arch, bits, thumb)?;
    Ok(Decoder { cs, skip })
}

fn capstone_for(arch: &str, bits: u8, thumb: bool) -> Result<Capstone, EngineError> {
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
        "arm" => Capstone::new()
            .arm()
            .mode(if thumb {
                arch::arm::ArchMode::Thumb
            } else {
                arch::arm::ArchMode::Arm
            })
            .build(),
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
    d: &Decoder,
    code: &[u8],
    addr: u64,
    limit: usize,
) -> Result<Vec<Instruction>, EngineError> {
    let mut out = Vec::new();
    let mut offset = 0usize;
    while offset < code.len() && out.len() < limit {
        let chunk = &code[offset..];
        let at = addr + offset as u64;
        let insns =
            d.cs.disasm_count(chunk, at, (limit - out.len()).min(4096))
                .map_err(|e| EngineError::Disasm(e.to_string()))?;
        if insns.is_empty() {
            let n = d.skip.min(chunk.len());
            out.push(Instruction {
                addr: at,
                bytes: chunk[..n].iter().map(|b| format!("{b:02x}")).collect(),
                mnemonic: "(bad)".into(),
                operands: String::new(),
                target: None,
                comment: None,
            });
            offset += n;
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
