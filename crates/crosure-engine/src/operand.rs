/// Parses a direct branch/call target from Capstone operand text, e.g.
/// `0x401020` (x86) or `#0x401020` (ARM).
///
/// ```
/// use crosure_engine::parse_branch_target;
/// assert_eq!(parse_branch_target("0x401020"), Some(0x401020));
/// assert_eq!(parse_branch_target("#0x10"), Some(0x10));
/// assert_eq!(parse_branch_target("rax"), None);
/// assert_eq!(parse_branch_target("qword ptr [rip + 0x2f]"), None);
/// ```
pub fn parse_branch_target(operands: &str) -> Option<u64> {
    let t = operands.trim().trim_start_matches('#');
    let hex = t.strip_prefix("0x")?;
    u64::from_str_radix(hex, 16).ok()
}

/// Resolves an x86-64 RIP-relative memory operand to an absolute address.
/// `next_ip` is the address of the following instruction.
///
/// ```
/// use crosure_engine::parse_rip_relative;
/// assert_eq!(parse_rip_relative("rax, [rip + 0x10]", 0x1000), Some(0x1010));
/// assert_eq!(parse_rip_relative("qword ptr [rip - 0x8]", 0x1000), Some(0xff8));
/// assert_eq!(parse_rip_relative("eax, [rbp - 4]", 0x1000), None);
/// ```
pub fn parse_rip_relative(operands: &str, next_ip: u64) -> Option<u64> {
    let start = operands.find("[rip")?;
    let rest = &operands[start + 4..];
    let end = rest.find(']')?;
    let inner = rest[..end].trim();
    if inner.is_empty() {
        return Some(next_ip);
    }
    let (sign, num) = match inner.strip_prefix('+') {
        Some(n) => (1i64, n.trim()),
        None => (-1i64, inner.strip_prefix('-')?.trim()),
    };
    let value = i64::from_str_radix(num.strip_prefix("0x")?, 16).ok()?;
    Some(next_ip.wrapping_add_signed(sign * value))
}

/// Absolute immediates such as `0x404020` in `mov eax, 0x404020` (32-bit x86).
pub(crate) fn immediates(operands: &str) -> Vec<u64> {
    operands
        .split(|c: char| c == ',' || c.is_whitespace() || c == '[' || c == ']' || c == '#')
        .filter_map(|tok| tok.strip_prefix("0x"))
        .filter_map(|hex| u64::from_str_radix(hex, 16).ok())
        .collect()
}

/// True for mnemonics that transfer control.
pub(crate) fn is_call(mnemonic: &str) -> bool {
    matches!(mnemonic, "call" | "bl" | "blx" | "jal" | "jalr")
}

/// True for jumps and conditional branches.
pub(crate) fn is_jump(mnemonic: &str) -> bool {
    mnemonic.starts_with('j')
        || mnemonic == "b"
        || mnemonic.starts_with("b.")
        || mnemonic == "bx"
        || mnemonic.starts_with("cb")
        || mnemonic.starts_with("tb")
}
