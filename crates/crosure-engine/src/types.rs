use serde::{Deserialize, Serialize};

/// A section of the binary.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SectionInfo {
    pub name: String,
    pub addr: u64,
    pub size: u64,
    pub file_offset: Option<u64>,
    pub executable: bool,
}

/// Headline facts about the binary.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BinaryInfo {
    pub path: String,
    /// `elf`, `pe`, `macho`, ...
    pub format: String,
    /// `x86_64`, `x86`, `aarch64`, `arm`, ...
    pub arch: String,
    pub bits: u8,
    pub little_endian: bool,
    pub entry: u64,
    pub size: u64,
    /// `sha256:<hex>` of the whole file.
    pub sha256: String,
    pub stripped: bool,
    pub sections: Vec<SectionInfo>,
}

/// A discovered function.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FunctionInfo {
    pub addr: u64,
    pub name: String,
    pub size: u64,
    /// `symbol`, `entry`, `call_target`.
    pub source: String,
}

/// One disassembled instruction.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Instruction {
    pub addr: u64,
    /// Raw bytes as lowercase hex.
    pub bytes: String,
    pub mnemonic: String,
    pub operands: String,
    /// Resolved branch/call target, if any.
    pub target: Option<u64>,
    /// Name of what `target` (or a data reference) points to, if known.
    pub comment: Option<String>,
}

/// A printable string found in the file.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StringRef {
    /// Virtual address when mapped, otherwise the file offset.
    pub addr: u64,
    pub mapped: bool,
    pub value: String,
    /// `ascii` or `utf16le`.
    pub encoding: String,
    pub section: Option<String>,
}

/// An imported symbol.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Import {
    pub name: String,
    pub library: String,
    /// Address code uses to reach the import (PLT stub / IAT slot), if known.
    pub addr: Option<u64>,
}

/// How one address refers to another.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum XrefKind {
    Call,
    Jump,
    Data,
}

/// A cross-reference `from` an instruction `to` an address.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Xref {
    pub from: u64,
    pub to: u64,
    pub kind: XrefKind,
    /// Function containing `from`, if known.
    pub from_func: Option<String>,
}
