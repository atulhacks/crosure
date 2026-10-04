//! AArch64: stubs, calls and string references on a stripped binary.

use std::path::Path;

use crosure_engine::{Engine, EngineError, NativeEngine, XrefKind};

fn stripped() -> Result<NativeEngine, EngineError> {
    NativeEngine::open(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/crackme-arm64-stripped"),
    )
}

#[test]
fn plt_stubs_are_named_and_imports_point_at_them() -> Result<(), EngineError> {
    let e = stripped()?;
    let stubs: Vec<String> = e
        .functions()?
        .into_iter()
        .filter(|f| f.source == "import_stub")
        .map(|f| f.name)
        .collect();
    for name in ["puts@plt", "strcmp@plt", "printf@plt"] {
        assert!(stubs.iter().any(|s| s == name), "{name} in {stubs:?}");
    }
    let strcmp = e.resolve("strcmp")?.ok_or(EngineError::Unmapped(0))?;
    assert!(e
        .function_at(strcmp)?
        .is_some_and(|f| f.name == "strcmp@plt"));
    Ok(())
}

#[test]
fn calls_reach_named_stubs() -> Result<(), EngineError> {
    let e = stripped()?;
    // check_password (0x888 in the unstripped build) calls strcmp.
    let strcmp = e.resolve("strcmp")?.ok_or(EngineError::Unmapped(0))?;
    let check = e.function_at(0x888)?.ok_or(EngineError::Unmapped(0x888))?;
    assert_eq!(check.size, 80, "exact size from .eh_frame");
    assert!(e
        .xrefs_from(check.addr)?
        .iter()
        .any(|x| x.to == strcmp && x.kind == XrefKind::Call));
    Ok(())
}

#[test]
fn adrp_pairs_resolve_strings() -> Result<(), EngineError> {
    let e = stripped()?;
    let wrong = e
        .strings(4)?
        .into_iter()
        .find(|s| s.value == "Wrong password")
        .ok_or(EngineError::Unmapped(0))?;
    let refs = e.xrefs_to(wrong.addr)?;
    assert!(!refs.is_empty(), "adrp + add to the string is a reference");
    // main (0x8d8) uses it; the disassembly says so.
    let main = e.disasm_function(0x8d8)?;
    assert!(main
        .iter()
        .any(|i| i.comment.as_deref() == Some("\"Wrong password\"")));
    // The bare adrp page is never taken for an address.
    assert!(main
        .iter()
        .filter(|i| i.mnemonic == "adrp")
        .all(|i| i.target.is_none()));
    Ok(())
}
