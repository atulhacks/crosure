//! 32-bit ARM (Thumb-2): modes, stubs, calls and literal-pool references.

use std::path::Path;

use crosure_engine::{Engine, EngineError, NativeEngine, XrefKind};

fn stripped() -> Result<NativeEngine, EngineError> {
    NativeEngine::open(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/crackme-arm32-stripped"),
    )
}

#[test]
fn thumb_code_is_decoded_and_functions_found() -> Result<(), EngineError> {
    let e = stripped()?;
    // decode, check_password, main of the unstripped build (Thumb bit cleared).
    for addr in [0x564, 0x5aa, 0x5f4] {
        assert!(e.functions()?.iter().any(|f| f.addr == addr), "{addr:#x}");
    }
    let check = e.disasm_function(0x5aa)?;
    assert!(
        check.iter().all(|i| i.mnemonic != "(bad)"),
        "decoded as Thumb"
    );
    Ok(())
}

#[test]
fn arm_plt_stubs_are_named_and_called() -> Result<(), EngineError> {
    let e = stripped()?;
    let strcmp = e.resolve("strcmp")?.ok_or(EngineError::Unmapped(0))?;
    assert!(e
        .function_at(strcmp)?
        .is_some_and(|f| f.name == "strcmp@plt"));
    assert!(e
        .xrefs_from(0x5aa)?
        .iter()
        .any(|x| x.to == strcmp && x.kind == XrefKind::Call));
    Ok(())
}

#[test]
fn literal_pool_references_reach_strings_and_code_holds_no_strings() -> Result<(), EngineError> {
    let e = stripped()?;
    let strings = e.strings(4)?;
    let wrong = strings
        .iter()
        .find(|s| s.value == "Wrong password")
        .ok_or(EngineError::Unmapped(0))?;
    assert!(e
        .xrefs_to(wrong.addr)?
        .iter()
        .any(|x| x.kind == XrefKind::Data && x.from >= 0x5f4 && x.from < 0x5f4 + 116));
    let text = e
        .info()?
        .sections
        .into_iter()
        .find(|s| s.name == ".text")
        .ok_or(EngineError::Unmapped(0))?;
    assert!(
        strings
            .iter()
            .filter(|s| s.mapped)
            .all(|s| s.addr < text.addr || s.addr >= text.addr + text.size),
        "printable runs inside code are not strings"
    );
    Ok(())
}
