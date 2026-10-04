//! Function bounds the compiler already wrote down: ELF/Mach-O `.eh_frame`
//! FDEs and PE x64 `.pdata` entries. On stripped binaries these give exact
//! starts and sizes for nearly every non-leaf function, which a sweep cannot.

use std::collections::{BTreeMap, BTreeSet};

use gimli::{BaseAddresses, CieOrFde, EhFrame, UnwindSection};
use object::pe::{IMAGE_DIRECTORY_ENTRY_EXCEPTION, IMAGE_FILE_MACHINE_AMD64};
use object::read::pe::{ImageNtHeaders, ImageOptionalHeader, PeFile64};
use object::{BinaryFormat, Object, ObjectSection};

/// `.pdata` UNWIND_INFO flag: this entry continues another function's range.
const UNW_FLAG_CHAININFO: u8 = 0x4;

/// Function ranges from unwind tables: start -> size.
pub(crate) fn function_ranges(file: &object::File<'_>, data: &[u8]) -> BTreeMap<u64, u64> {
    match file.format() {
        BinaryFormat::Elf | BinaryFormat::MachO => eh_frame(file),
        BinaryFormat::Pe if file.architecture() == object::Architecture::X86_64 => pdata(data),
        _ => BTreeMap::new(),
    }
}

/// Every FDE's `[initial_address, +len)`. Parse errors end the walk early
/// rather than failing the load: whatever was read is still correct.
fn eh_frame(file: &object::File<'_>) -> BTreeMap<u64, u64> {
    let mut out = BTreeMap::new();
    let Some(sec) = file
        .section_by_name(".eh_frame")
        .or_else(|| file.section_by_name("__eh_frame"))
    else {
        return out;
    };
    let Ok(bytes) = sec.data() else {
        return out;
    };
    let mut bases = BaseAddresses::default().set_eh_frame(sec.address());
    if let Some(text) = file
        .section_by_name(".text")
        .or_else(|| file.section_by_name("__text"))
    {
        bases = bases.set_text(text.address());
    }
    let section = EhFrame::new(bytes, gimli::RunTimeEndian::Little);
    let mut entries = section.entries(&bases);
    while let Ok(Some(entry)) = entries.next() {
        if let CieOrFde::Fde(partial) = entry {
            if let Ok(fde) = partial.parse(EhFrame::cie_from_offset) {
                if fde.len() > 0 && fde.initial_address() != 0 {
                    out.insert(fde.initial_address(), fde.len());
                }
            }
        }
    }
    out
}

/// x64 RUNTIME_FUNCTION entries (begin, end, unwind info), 12 bytes each.
/// Chained entries are skipped: they describe a chunk of another function.
fn pdata(data: &[u8]) -> BTreeMap<u64, u64> {
    let mut out = BTreeMap::new();
    let Ok(pe) = PeFile64::parse(data) else {
        return out;
    };
    if pe
        .nt_headers()
        .file_header()
        .machine
        .get(object::LittleEndian)
        != IMAGE_FILE_MACHINE_AMD64
    {
        return out;
    }
    let base = pe.nt_headers().optional_header().image_base();
    let sections = pe.section_table();
    let Some(Ok(table)) = pe
        .data_directories()
        .get(IMAGE_DIRECTORY_ENTRY_EXCEPTION)
        .map(|d| d.data(data, &sections))
    else {
        return out;
    };
    for e in table.chunks_exact(12) {
        let word = |i: usize| u32::from_le_bytes([e[i], e[i + 1], e[i + 2], e[i + 3]]);
        let (begin, end, unwind) = (word(0), word(4), word(8));
        if begin == 0 || end <= begin {
            continue;
        }
        let flags = sections
            .pe_data_at(data, unwind)
            .and_then(|u| u.first())
            .map_or(0, |b| b >> 3);
        if flags & UNW_FLAG_CHAININFO == 0 {
            out.insert(base + u64::from(begin), u64::from(end - begin));
        }
    }
    out
}

/// Code addresses stored as pointers in data, found through the loader's
/// relocations (ELF `*_RELATIVE`, PE DIR64 base relocations). These are
/// mostly function pointers in tables and structs, which no call reaches.
pub(crate) fn relocated_pointers(file: &object::File<'_>, data: &[u8]) -> BTreeSet<u64> {
    match file.format() {
        BinaryFormat::Elf => elf_relative(file),
        BinaryFormat::Pe if file.is_64() => pe_dir64(data),
        _ => BTreeSet::new(),
    }
}

/// `R_X86_64_RELATIVE` (8) and `R_AARCH64_RELATIVE` (1027) carry the target
/// in their addend; `R_ARM_RELATIVE` (23) is REL, so the target is the word
/// already stored at the relocated address.
fn elf_relative(file: &object::File<'_>) -> BTreeSet<u64> {
    let (relative, implicit) = match file.architecture() {
        object::Architecture::X86_64 => (8, false),
        object::Architecture::Aarch64 => (1027, false),
        object::Architecture::Arm => (23, true),
        _ => return BTreeSet::new(),
    };
    let Some(relocs) = file.dynamic_relocations() else {
        return BTreeSet::new();
    };
    let word_at = |addr: u64| -> Option<u64> {
        let sec = file
            .sections()
            .find(|s| addr >= s.address() && addr + 4 <= s.address() + s.size())?;
        let off = usize::try_from(addr - sec.address()).ok()?;
        let b = sec.data().ok()?.get(off..off + 4)?;
        Some(u64::from(u32::from_le_bytes(b.try_into().ok()?)))
    };
    relocs
        .filter(|(_, r)| matches!(r.flags(), object::RelocationFlags::Elf { r_type } if r_type == relative))
        .filter_map(|(at, r)| if implicit { word_at(at) } else { u64::try_from(r.addend()).ok() })
        .collect()
}

/// The 8-byte value at every DIR64 base relocation.
fn pe_dir64(data: &[u8]) -> BTreeSet<u64> {
    let mut out = BTreeSet::new();
    let Ok(pe) = PeFile64::parse(data) else {
        return out;
    };
    let sections = pe.section_table();
    let Ok(Some(mut blocks)) = pe.data_directories().relocation_blocks(data, &sections) else {
        return out;
    };
    while let Ok(Some(block)) = blocks.next() {
        for r in block {
            if r.typ != object::pe::IMAGE_REL_BASED_DIR64 {
                continue;
            }
            if let Some(v) = sections
                .pe_data_at(data, r.virtual_address)
                .and_then(|b| b.get(..8))
                .and_then(|b| <[u8; 8]>::try_from(b).ok())
            {
                out.insert(u64::from_le_bytes(v));
            }
        }
    }
    out
}
