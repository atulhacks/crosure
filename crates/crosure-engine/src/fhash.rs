//! Function fingerprints that survive relinking, after Ghidra FunctionID.
//!
//! Each instruction is reduced to its mnemonic and operands with every
//! number masked, so the same code at a different address hashes the same
//! (the *structural* hash). A second, *specific* hash keeps small constants
//! (under 0x100, not branch targets or RIP-relative), which tells apart
//! functions that differ only in a key, a size or a flag. Padding is
//! skipped. Functions under [`MIN_INSNS`] instructions get no fingerprint:
//! too many unrelated stubs look alike.

use sha2::{Digest, Sha256};

use crate::Instruction;

/// Fewest counted instructions (calls excluded) worth fingerprinting.
pub const MIN_INSNS: usize = 4;
/// Format tag: bump it when the normalization changes, so old and new
/// fingerprints never compare equal by accident.
const VERSION: &str = "fid1";

/// A function's fingerprint.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FunctionHash {
    /// All numbers masked: survives relinking and different constants.
    pub structural: String,
    /// Small constants kept: survives relinking only.
    pub specific: String,
    /// Instructions hashed, not counting calls (FID's size measure).
    pub insns: usize,
}

impl FunctionHash {
    /// The compact form stored on steps: `fid1:<structural>/<specific>/<insns>`.
    ///
    /// ```
    /// use crosure_engine::FunctionHash;
    /// let h = FunctionHash { structural: "aa".into(), specific: "bb".into(), insns: 9 };
    /// assert_eq!(h.fingerprint(), "fid1:aa/bb/9");
    /// assert_eq!(FunctionHash::parse(&h.fingerprint()), Some(h));
    /// ```
    pub fn fingerprint(&self) -> String {
        format!(
            "{VERSION}:{}/{}/{}",
            self.structural, self.specific, self.insns
        )
    }

    /// Parses [`FunctionHash::fingerprint`] output (other versions: `None`).
    pub fn parse(s: &str) -> Option<Self> {
        let rest = s.strip_prefix(VERSION)?.strip_prefix(':')?;
        let mut parts = rest.split('/');
        let (structural, specific, insns) = (parts.next()?, parts.next()?, parts.next()?);
        Some(Self {
            structural: structural.into(),
            specific: specific.into(),
            insns: insns.parse().ok()?,
        })
    }

    /// The prefix shared by every fingerprint with this structural hash
    /// (`fid1:<structural>/`), for lookups.
    pub fn structural_key(&self) -> String {
        format!("{VERSION}:{}/", self.structural)
    }
}

/// `operands` with each number replaced by `#` (and its sign dropped),
/// except small constants when `keep_small` is set.
fn mask(operands: &str, keep_small: bool) -> String {
    let mut out = String::with_capacity(operands.len());
    let chars: Vec<char> = operands.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        let starts_word = i == 0 || !(chars[i - 1].is_ascii_alphanumeric() || chars[i - 1] == '_');
        if c.is_ascii_digit() && starts_word {
            let start = i;
            while i < chars.len() && (chars[i].is_ascii_alphanumeric()) {
                i += 1;
            }
            let token: String = chars[start..i].iter().collect();
            let value = token.strip_prefix("0x").map_or_else(
                || token.parse::<u64>().ok(),
                |h| u64::from_str_radix(h, 16).ok(),
            );
            match value {
                Some(v) if keep_small && v < 0x100 => out.push_str(&token),
                _ => {
                    // A masked offset's sign depends on layout (`rip - 0x10`
                    // vs `rip + 0x30` after relinking), so it is masked too.
                    if out.ends_with("- ") {
                        out.truncate(out.len() - 2);
                        out.push_str("+ ");
                    }
                    out.push('#');
                }
            }
            continue;
        }
        out.push(c);
        i += 1;
    }
    out
}

fn digest(lines: &str) -> String {
    let d = Sha256::digest(lines.as_bytes());
    d.iter().take(8).map(|b| format!("{b:02x}")).collect()
}

/// Fingerprints a function from its disassembly.
///
/// ```
/// use crosure_engine::{function_hash, Instruction};
/// let insn = |addr: u64, m: &str, ops: &str| Instruction {
///     addr, bytes: String::new(), mnemonic: m.into(), operands: ops.into(), target: None, comment: None,
/// };
/// let f = |key: &str, at: u64| vec![
///     insn(at, "push", "rbp"), insn(at + 1, "mov", "rbp, rsp"),
///     insn(at + 4, "xor", &format!("eax, {key}")), insn(at + 7, "call", &format!("{:#x}", at + 0x40)),
///     insn(at + 12, "pop", "rbp"), insn(at + 13, "ret", ""), insn(at + 14, "nop", ""),
/// ];
/// let a = function_hash(&f("0x43", 0x1000)).unwrap_or_else(|| unreachable!());
/// let moved = function_hash(&f("0x43", 0x5000)).unwrap_or_else(|| unreachable!());
/// let other_key = function_hash(&f("0x44", 0x1000)).unwrap_or_else(|| unreachable!());
/// assert_eq!(a, moved, "relocation does not change either hash");
/// assert_eq!(a.structural, other_key.structural);
/// assert_ne!(a.specific, other_key.specific, "small constants matter to the specific hash");
/// assert_eq!(a.insns, 5, "padding and calls are not counted");
/// ```
pub fn function_hash(insns: &[Instruction]) -> Option<FunctionHash> {
    let (mut structural, mut specific, mut counted) = (String::new(), String::new(), 0);
    for i in insns {
        if matches!(i.mnemonic.as_str(), "nop" | "int3") {
            continue;
        }
        let addressy =
            i.target.is_some() || i.operands.contains("rip") || i.operands.contains("pc");
        structural.push_str(&format!("{} {}\n", i.mnemonic, mask(&i.operands, false)));
        specific.push_str(&format!(
            "{} {}\n",
            i.mnemonic,
            mask(&i.operands, !addressy)
        ));
        if !i.mnemonic.starts_with("call") && i.mnemonic != "bl" && i.mnemonic != "blr" {
            counted += 1;
        }
    }
    (counted >= MIN_INSNS).then(|| FunctionHash {
        structural: digest(&structural),
        specific: digest(&specific),
        insns: counted,
    })
}

#[cfg(test)]
mod tests {
    use super::mask;

    #[test]
    fn masked_offsets_lose_their_sign_kept_constants_do_not() {
        assert_eq!(
            mask("rax, [rip - 0x2f0]", true),
            mask("rax, [rip + 0x51c]", true)
        );
        assert_ne!(
            mask("eax, [rbp - 0x18]", true),
            mask("eax, [rbp + 0x18]", true)
        );
        assert_eq!(mask("eax, [rbp - 0x18]", false), "eax, [rbp + #]");
        assert_eq!(
            mask("xmm0, xmm1", false),
            "xmm0, xmm1",
            "register digits stay"
        );
    }
}
