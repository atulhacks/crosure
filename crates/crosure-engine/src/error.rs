use thiserror::Error;

/// Errors raised by an analysis backend.
#[derive(Debug, Error)]
pub enum EngineError {
    /// The file could not be read.
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    /// The file is not a binary format we can parse.
    #[error("parse error: {0}")]
    Parse(String),
    /// The architecture has no disassembler mapping.
    #[error("unsupported architecture: {0}")]
    UnsupportedArch(String),
    /// Capstone failed.
    #[error("disassembler error: {0}")]
    Disasm(String),
    /// The external decompiler is missing or failed.
    #[error("decompiler: {0}")]
    Decompiler(String),
    /// The address is not inside any mapped section.
    #[error("address {0:#x} is not mapped")]
    Unmapped(u64),
}
