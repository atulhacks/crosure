//! x86 switch tables. A `switch` compiles to an indirect jump through a
//! table of case addresses, so case blocks have no direct references. They
//! are recovered here so the cases are known to be inside a function.
//!
//! - PIC (gcc, clang): `lea rB, [rip + T]`, `movsxd rX, dword ptr [rB + rI*4]`,
//!   `add rX, rB`, `jmp rX`: 32-bit entries relative to `T`.
//! - Absolute: `jmp qword ptr [rI*8 + T]`: 64-bit addresses.
//!
//! The table length is not decoded from the bounds check: entries are read
//! until one points outside executable code, which is enough to mark cases.

use std::collections::{BTreeSet, HashMap};

/// Longest table read.
const MAX_ENTRIES: usize = 1024;

/// A table found in code.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Table {
    /// i32 entries, each relative to the table address.
    Relative(u64),
    /// u64 absolute entries.
    Absolute(u64),
}

/// Watches `lea reg, [rip + X]` results to spot table loads.
#[derive(Default)]
pub(crate) struct TableFinder {
    lea: HashMap<String, u64>,
}

fn first_operand(ops: &str) -> &str {
    ops.split(',').next().unwrap_or("").trim()
}

impl TableFinder {
    /// Feeds one instruction; `rip_target` is its resolved `[rip + X]`
    /// address, if any. Returns a table this instruction indexes.
    pub(crate) fn step(&mut self, m: &str, ops: &str, rip_target: Option<u64>) -> Option<Table> {
        let found = if m == "movsxd" && ops.contains("*4]") {
            ops.split('[')
                .nth(1)
                .and_then(|inner| inner.split('+').next())
                .and_then(|base| self.lea.get(base.trim()).copied())
                .map(Table::Relative)
        } else if m == "jmp" && ops.contains("*8 + 0x") {
            ops.rsplit("+ 0x")
                .next()
                .and_then(|h| u64::from_str_radix(h.trim_end_matches(']'), 16).ok())
                .map(Table::Absolute)
        } else {
            None
        };
        let dest = first_operand(ops).to_string();
        match (m, rip_target) {
            ("lea", Some(t)) => {
                self.lea.insert(dest, t);
            }
            _ if !matches!(m, "cmp" | "test" | "push") => {
                self.lea.remove(&dest);
            }
            _ => {}
        }
        if matches!(m, "ret" | "call") {
            self.lea.clear();
        }
        found
    }
}

/// The case addresses of `table`, read with `read` (little-endian bytes at a
/// virtual address) while they land in executable code (`is_code`).
pub(crate) fn cases(
    table: Table,
    read: &dyn Fn(u64, usize) -> Option<Vec<u8>>,
    is_code: &dyn Fn(u64) -> bool,
) -> BTreeSet<u64> {
    let mut out = BTreeSet::new();
    for k in 0..MAX_ENTRIES as u64 {
        let target = match table {
            Table::Relative(t) => read(t + 4 * k, 4)
                .and_then(|b| <[u8; 4]>::try_from(b.as_slice()).ok())
                .map(|b| t.wrapping_add_signed(i64::from(i32::from_le_bytes(b)))),
            Table::Absolute(t) => read(t + 8 * k, 8)
                .and_then(|b| <[u8; 8]>::try_from(b.as_slice()).ok())
                .map(u64::from_le_bytes),
        };
        match target {
            Some(a) if is_code(a) => {
                out.insert(a);
            }
            _ => break,
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::{cases, Table, TableFinder};

    #[test]
    fn pic_table_is_found_and_read() {
        let mut f = TableFinder::default();
        assert_eq!(f.step("lea", "rdx, [rip + 0x9b6c]", Some(0xe0b8)), None);
        assert_eq!(
            f.step("movsxd", "rax, dword ptr [rdx + rax*4]", None),
            Some(Table::Relative(0xe0b8))
        );
        // Entries: -0x100, -0x80, then garbage that leaves the code range.
        let bytes: Vec<u8> = [-0x100i32, -0x80, 0x7fff_0000]
            .iter()
            .flat_map(|v| v.to_le_bytes())
            .collect();
        let read = |a: u64, n: usize| {
            let off = usize::try_from(a.checked_sub(0xe0b8)?).ok()?;
            bytes.get(off..off + n).map(<[u8]>::to_vec)
        };
        let is_code = |a: u64| (0xd000..0xe100).contains(&a);
        let got: Vec<u64> = cases(Table::Relative(0xe0b8), &read, &is_code)
            .into_iter()
            .collect();
        assert_eq!(got, vec![0xdfb8, 0xe038], "stops at the entry leaving code");
    }

    #[test]
    fn absolute_table_and_clobbering() {
        let mut f = TableFinder::default();
        assert_eq!(
            f.step("jmp", "qword ptr [rax*8 + 0x4060a0]", None),
            Some(Table::Absolute(0x4060a0))
        );
        f.step("lea", "rdx, [rip + 0x10]", Some(0x2000));
        f.step("mov", "rdx, qword ptr [rbp - 8]", None);
        assert_eq!(f.step("movsxd", "rax, dword ptr [rdx + rax*4]", None), None);
    }
}
