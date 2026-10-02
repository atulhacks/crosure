use crate::{
    build_cfg, BasicBlock, BinaryInfo, EngineError, FunctionInfo, Import, Instruction, StringRef,
    Xref,
};

/// The backend-neutral analysis surface. Every op the UI or an agent runs
/// goes through this trait, so it can be recorded the same way whatever
/// engine is underneath.
pub trait Engine: Send + Sync {
    /// Backend name, e.g. `native`.
    fn backend(&self) -> &'static str;
    /// Format, architecture, hashes, sections.
    fn info(&self) -> Result<BinaryInfo, EngineError>;
    /// All discovered functions, sorted by address.
    fn functions(&self) -> Result<Vec<FunctionInfo>, EngineError>;
    /// The function containing `addr`, if any.
    fn function_at(&self, addr: u64) -> Result<Option<FunctionInfo>, EngineError>;
    /// Disassembly of a whole function.
    fn disasm_function(&self, addr: u64) -> Result<Vec<Instruction>, EngineError>;
    /// Up to `count` instructions starting at `addr`.
    fn disasm(&self, addr: u64, count: usize) -> Result<Vec<Instruction>, EngineError>;
    /// Printable strings of at least `min_len` characters.
    fn strings(&self, min_len: usize) -> Result<Vec<StringRef>, EngineError>;
    /// Imported functions.
    fn imports(&self) -> Result<Vec<Import>, EngineError>;
    /// References to `addr`.
    fn xrefs_to(&self, addr: u64) -> Result<Vec<Xref>, EngineError>;
    /// References made from inside the function at `addr`.
    fn xrefs_from(&self, addr: u64) -> Result<Vec<Xref>, EngineError>;
    /// Raw bytes at a virtual address.
    fn read_bytes(&self, addr: u64, len: usize) -> Result<Vec<u8>, EngineError>;
    /// Resolves a function or import name to an address.
    fn resolve(&self, name: &str) -> Result<Option<u64>, EngineError>;

    /// Basic blocks of the function containing `addr` (built from its disassembly).
    fn function_graph(&self, addr: u64) -> Result<Vec<BasicBlock>, EngineError> {
        let f = self.function_at(addr)?.ok_or(EngineError::Unmapped(addr))?;
        let insns = self.disasm_function(f.addr)?;
        Ok(build_cfg(&insns, f.addr, f.addr + f.size))
    }
}
