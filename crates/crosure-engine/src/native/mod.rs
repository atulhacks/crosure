mod disasm;
mod load;
mod pe;
mod strings;
mod sweep;
mod unwind;

use std::collections::BTreeMap;
use std::path::Path;

use crate::operand::{immediates, parse_branch_target, parse_rip_relative};
use crate::{BinaryInfo, Engine, EngineError, FunctionInfo, Import, Instruction, StringRef, Xref};
use load::Loaded;
use sweep::{containing, Analysis};

/// Pure-Rust backend: `object` for parsing, Capstone for disassembly.
/// Analysis (one linear sweep) runs once, when the file is opened.
pub struct NativeEngine {
    data: Vec<u8>,
    loaded: Loaded,
    analysis: Analysis,
    strings: Vec<StringRef>,
    string_at: BTreeMap<u64, usize>,
}

impl NativeEngine {
    /// Reads and analyzes a binary from disk.
    pub fn open(path: &Path) -> Result<Self, EngineError> {
        let data = std::fs::read(path)?;
        Self::from_bytes(&path.display().to_string(), data)
    }

    /// Analyzes an in-memory binary; `path` is only used for display.
    pub fn from_bytes(path: &str, data: Vec<u8>) -> Result<Self, EngineError> {
        let loaded = load::load(path, &data)?;
        let analysis = sweep::analyze(&data, &loaded)?;
        let strings = strings::scan(&data, &loaded.info.sections, 4);
        let string_at = strings
            .iter()
            .enumerate()
            .filter(|(_, s)| s.mapped)
            .map(|(i, s)| (s.addr, i))
            .collect();
        Ok(Self {
            data,
            loaded,
            analysis,
            strings,
            string_at,
        })
    }

    /// File bytes behind a virtual address range, clamped to its section.
    fn slice(&self, addr: u64, len: usize) -> Result<&[u8], EngineError> {
        let sec = self
            .loaded
            .info
            .sections
            .iter()
            .find(|s| s.file_offset.is_some() && addr >= s.addr && addr < s.addr + s.size)
            .ok_or(EngineError::Unmapped(addr))?;
        let off = (sec.file_offset.unwrap_or(0) + (addr - sec.addr)) as usize;
        let max = (sec.addr + sec.size - addr) as usize;
        let end = (off + len.min(max)).min(self.data.len());
        self.data.get(off..end).ok_or(EngineError::Unmapped(addr))
    }

    fn decode(&self, addr: u64, len: usize, limit: usize) -> Result<Vec<Instruction>, EngineError> {
        let info = &self.loaded.info;
        let cs = disasm::capstone_for(&info.arch, info.bits)?;
        let mut insns = disasm::decode(&cs, self.slice(addr, len)?, addr, limit)?;
        for insn in &mut insns {
            self.annotate(insn);
        }
        Ok(insns)
    }

    /// Fills `target` and a human-readable `comment` (callee name or string).
    fn annotate(&self, insn: &mut Instruction) {
        let next_ip = insn.addr + (insn.bytes.len() / 2) as u64;
        let target = parse_branch_target(&insn.operands)
            .or_else(|| parse_rip_relative(&insn.operands, next_ip))
            .or_else(|| {
                immediates(&insn.operands)
                    .into_iter()
                    .find(|t| self.analysis.names.contains_key(t) || self.string_at.contains_key(t))
            });
        insn.target = target;
        insn.comment = target.and_then(|t| {
            if let Some(name) = self.analysis.names.get(&t) {
                return Some(name.clone());
            }
            let s = &self.strings[*self.string_at.get(&t)?];
            let mut v: String = s.value.chars().take(60).collect();
            if s.value.chars().count() > 60 {
                v.push('…');
            }
            Some(format!("\"{v}\""))
        });
    }
}

impl Engine for NativeEngine {
    fn backend(&self) -> &'static str {
        "native"
    }

    fn info(&self) -> Result<BinaryInfo, EngineError> {
        Ok(self.loaded.info.clone())
    }

    fn functions(&self) -> Result<Vec<FunctionInfo>, EngineError> {
        Ok(self.analysis.functions.clone())
    }

    fn function_at(&self, addr: u64) -> Result<Option<FunctionInfo>, EngineError> {
        Ok(containing(&self.analysis.functions, addr).cloned())
    }

    fn disasm_function(&self, addr: u64) -> Result<Vec<Instruction>, EngineError> {
        let f = self.function_at(addr)?.ok_or(EngineError::Unmapped(addr))?;
        self.decode(f.addr, f.size.max(1) as usize, usize::MAX)
    }

    fn disasm(&self, addr: u64, count: usize) -> Result<Vec<Instruction>, EngineError> {
        self.decode(addr, count.saturating_mul(16), count)
    }

    fn strings(&self, min_len: usize) -> Result<Vec<StringRef>, EngineError> {
        Ok(self
            .strings
            .iter()
            .filter(|s| s.value.len() >= min_len)
            .cloned()
            .collect())
    }

    fn imports(&self) -> Result<Vec<Import>, EngineError> {
        // Slots first, so a PLT/thunk stub (what code actually calls) wins.
        let by_name: BTreeMap<&str, u64> = self
            .loaded
            .import_slots
            .iter()
            .map(|(a, n)| (n.as_str(), *a))
            .chain(
                self.analysis
                    .functions
                    .iter()
                    .filter(|f| f.source == "import_stub")
                    .filter_map(|f| f.name.strip_suffix("@plt").map(|n| (n, f.addr))),
            )
            .collect();
        Ok(self
            .loaded
            .imports
            .iter()
            .map(|(library, name)| Import {
                addr: by_name.get(name.as_str()).copied(),
                name: name.clone(),
                library: library.clone(),
            })
            .collect())
    }

    fn xrefs_to(&self, addr: u64) -> Result<Vec<Xref>, EngineError> {
        let x = &self.analysis.xrefs;
        let start = x.partition_point(|r| r.to < addr);
        Ok(x[start..]
            .iter()
            .take_while(|r| r.to == addr)
            .cloned()
            .collect())
    }

    fn xrefs_from(&self, addr: u64) -> Result<Vec<Xref>, EngineError> {
        let f = self.function_at(addr)?.ok_or(EngineError::Unmapped(addr))?;
        let mut out: Vec<Xref> = self
            .analysis
            .xrefs
            .iter()
            .filter(|r| r.from >= f.addr && r.from < f.addr + f.size)
            .cloned()
            .collect();
        out.sort_by_key(|r| r.from);
        Ok(out)
    }

    fn read_bytes(&self, addr: u64, len: usize) -> Result<Vec<u8>, EngineError> {
        Ok(self.slice(addr, len)?.to_vec())
    }

    fn resolve(&self, name: &str) -> Result<Option<u64>, EngineError> {
        let found = self
            .analysis
            .functions
            .iter()
            .find(|f| f.name == name)
            .map(|f| f.addr);
        Ok(found.or_else(|| {
            self.imports()
                .ok()?
                .into_iter()
                .find(|i| i.name == name)
                .and_then(|i| i.addr)
        }))
    }
}
