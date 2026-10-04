//! Function starts and sizes on stripped binaries, checked against the
//! unstripped fixture's symbol table.

use std::path::{Path, PathBuf};

use crosure_engine::{Engine, EngineError, NativeEngine};
use object::{Object, ObjectSection};

fn path(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

/// No function may end in alignment padding. A single `nop` right after a
/// call is code: compilers emit it after noreturn calls so the return
/// address stays inside the function.
fn ends_in_code(e: &NativeEngine) -> Result<(), EngineError> {
    for f in e.functions()?.iter().filter(|f| f.source != "import_stub") {
        let insns = e.disasm_function(f.addr)?;
        let m = |i: usize| {
            insns
                .len()
                .checked_sub(i)
                .and_then(|j| insns.get(j))
                .map_or("", |x| x.mnemonic.as_str())
        };
        let padded = m(1) == "int3" || (m(1) == "nop" && m(2) != "call");
        assert!(
            !padded,
            "{} at {:#x} ({} bytes, {}) ends in padding",
            f.name, f.addr, f.size, f.source
        );
    }
    Ok(())
}

#[test]
fn stripped_elf_matches_symbol_starts_and_sizes() -> Result<(), EngineError> {
    let truth = NativeEngine::open(&path("crackme-x64"))?;
    let stripped = NativeEngine::open(&path("crackme-x64-stripped"))?;
    let found = stripped.functions()?;
    // `.init`/`.fini` are only reached by the loader and have no unwind entry.
    let text = truth
        .info()?
        .sections
        .into_iter()
        .find(|s| s.name == ".text")
        .ok_or(EngineError::Unmapped(0))?;
    let (mut total, mut exact) = (0, 0);
    for f in truth.functions()?.iter().filter(|f| {
        f.source == "symbol" && f.size > 0 && f.addr >= text.addr && f.addr < text.addr + text.size
    }) {
        total += 1;
        let Some(g) = found.iter().find(|g| g.addr == f.addr) else {
            continue;
        };
        if g.size == f.size {
            exact += 1;
        }
        // Sizes read from `.eh_frame` must be exact.
        if g.source == "unwind" {
            assert_eq!(g.size, f.size, "{} at {:#x}", f.name, f.addr);
        }
    }
    // Only the CRT's deregister_tm_clones / register_tm_clones lack an FDE.
    assert!(total >= 5 && exact + 2 >= total, "{exact}/{total} exact");
    // The PLT has an FDE too, but its stubs are imports, not one function.
    assert!(
        found.iter().all(|g| g.addr != 0x1020),
        "PLT header is not a function"
    );
    ends_in_code(&stripped)
}

#[test]
fn pe_functions_take_pdata_extents() -> Result<(), EngineError> {
    // The fixture has no COFF symbols: `.pdata` is what names the functions.
    let e = NativeEngine::open(&path("crackme-x64.exe"))?;
    let fs = e.functions()?;
    let unwind = fs.iter().filter(|f| f.source == "unwind").count();
    assert!(
        unwind * 2 > fs.len(),
        "{unwind} of {} from .pdata",
        fs.len()
    );
    ends_in_code(&e)
}

#[test]
fn without_unwind_tables_padding_is_trimmed() -> Result<(), Box<dyn std::error::Error>> {
    let mut data = std::fs::read(path("crackme-x64-stripped"))?;
    let range = {
        let file = object::File::parse(&*data)?;
        let sec = file.section_by_name(".eh_frame").ok_or("no .eh_frame")?;
        sec.file_range().ok_or("no file range")?
    };
    let (off, len) = (range.0 as usize, range.1 as usize);
    data[off..off + len].fill(0);
    let e = NativeEngine::from_bytes("crackme-no-eh", data)?;
    let fs = e.functions()?;
    assert!(fs.iter().all(|f| f.source != "unwind"));
    // Without unwind data, main is still found from `lea rdi, [rip + main]`.
    assert!(fs
        .iter()
        .any(|f| f.addr == 0x1227 && f.source == "code_pointer"));
    ends_in_code(&e)?;
    Ok(())
}
