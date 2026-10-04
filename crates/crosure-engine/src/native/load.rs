use std::collections::{BTreeMap, BTreeSet};

use object::{
    Architecture, BinaryFormat, Object, ObjectSection, ObjectSymbol, ObjectSymbolTable, SectionKind,
};
use sha2::{Digest, Sha256};

use crate::{BinaryInfo, EngineError, SectionInfo};

/// Everything extracted from the object file, owned (no borrow of the bytes).
pub(crate) struct Loaded {
    pub info: BinaryInfo,
    /// Defined code symbols: addr -> (name, size).
    pub symbols: BTreeMap<u64, (String, u64)>,
    /// Exported addresses: addr -> name.
    pub exports: BTreeMap<u64, String>,
    /// `(library, name)` of every import.
    pub imports: Vec<(String, String)>,
    /// Import slot (GOT entry / IAT entry) -> import name.
    pub import_slots: BTreeMap<u64, String>,
    /// Function ranges from `.eh_frame` / `.pdata`: start -> size.
    pub unwind: BTreeMap<u64, u64>,
    /// Addresses stored as pointers via load-time relocations.
    pub relocated: BTreeSet<u64>,
}

/// `sha256:<hex>` of a buffer.
pub(crate) fn sha256(data: &[u8]) -> String {
    let d = Sha256::digest(data);
    format!(
        "sha256:{}",
        d.iter().map(|b| format!("{b:02x}")).collect::<String>()
    )
}

fn arch_name(a: Architecture) -> &'static str {
    match a {
        Architecture::X86_64 | Architecture::X86_64_X32 => "x86_64",
        Architecture::I386 => "x86",
        Architecture::Aarch64 => "aarch64",
        Architecture::Arm => "arm",
        Architecture::Mips | Architecture::Mips64 => "mips",
        Architecture::Riscv64 | Architecture::Riscv32 => "riscv",
        Architecture::PowerPc | Architecture::PowerPc64 => "ppc",
        _ => "unknown",
    }
}

fn format_name(f: BinaryFormat) -> &'static str {
    match f {
        BinaryFormat::Elf => "elf",
        BinaryFormat::Pe => "pe",
        BinaryFormat::MachO => "macho",
        BinaryFormat::Coff => "coff",
        BinaryFormat::Wasm => "wasm",
        _ => "unknown",
    }
}

fn utf8(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

/// Parses `data` into owned analysis inputs.
pub(crate) fn load(path: &str, data: &[u8]) -> Result<Loaded, EngineError> {
    let file = object::File::parse(data).map_err(|e| EngineError::Parse(e.to_string()))?;
    let sections: Vec<SectionInfo> = file
        .sections()
        .filter(|s| s.address() != 0 || s.kind() == SectionKind::Text)
        .map(|s| SectionInfo {
            name: s.name().unwrap_or("?").to_string(),
            addr: s.address(),
            size: s.size(),
            file_offset: s.file_range().map(|(off, _)| off),
            executable: s.kind() == SectionKind::Text,
        })
        .collect();

    let mut symbols = BTreeMap::new();
    for sym in file.symbols().chain(file.dynamic_symbols()) {
        if sym.kind() == object::SymbolKind::Text && sym.is_definition() && sym.address() != 0 {
            if let Ok(name) = sym.name() {
                if !name.is_empty() {
                    symbols
                        .entry(sym.address())
                        .or_insert((name.to_string(), sym.size()));
                }
            }
        }
    }
    let exports = file
        .exports()
        .map(|ex| {
            ex.into_iter()
                .map(|e| (e.address(), utf8(e.name())))
                .collect()
        })
        .unwrap_or_default();
    let imports = file
        .imports()
        .map(|im| {
            im.iter()
                .map(|i| (utf8(i.library()), utf8(i.name())))
                .collect()
        })
        .unwrap_or_default();

    let mut import_slots = BTreeMap::new();
    match file.format() {
        BinaryFormat::Elf => elf_slots(&file, &mut import_slots),
        BinaryFormat::Pe => super::pe::iat_slots(data, file.is_64(), &mut import_slots),
        _ => {}
    }

    let stripped = file.symbols().next().is_none();
    let info = BinaryInfo {
        path: path.to_string(),
        format: format_name(file.format()).into(),
        arch: arch_name(file.architecture()).into(),
        bits: if file.is_64() { 64 } else { 32 },
        little_endian: file.is_little_endian(),
        entry: file.entry(),
        size: data.len() as u64,
        sha256: sha256(data),
        stripped,
        sections,
    };
    Ok(Loaded {
        info,
        symbols,
        exports,
        imports,
        import_slots,
        unwind: super::unwind::function_ranges(&file, data),
        relocated: super::unwind::relocated_pointers(&file, data),
    })
}

/// ELF: each dynamic relocation against a symbol marks a GOT slot.
fn elf_slots(file: &object::File<'_>, out: &mut BTreeMap<u64, String>) {
    let (Some(relocs), Some(dynsyms)) = (file.dynamic_relocations(), file.dynamic_symbol_table())
    else {
        return;
    };
    for (slot, reloc) in relocs {
        if let object::RelocationTarget::Symbol(idx) = reloc.target() {
            if let Ok(sym) = dynsyms.symbol_by_index(idx) {
                if let Ok(name) = sym.name() {
                    if !name.is_empty() && !sym.is_definition() {
                        out.insert(slot, name.to_string());
                    }
                }
            }
        }
    }
}
