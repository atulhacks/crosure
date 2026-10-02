//! Runs only where rizin + rz-ghidra are installed; elsewhere it checks the
//! "not available" path.

use std::path::Path;

use crosure_engine::{decompile, decompiler_status, EngineError};

#[test]
fn decompiles_check_password_or_explains_why_not() -> Result<(), EngineError> {
    let bin = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/crackme-x64");
    if !decompiler_status().available {
        let e = decompile(&bin, 0x11d9).err().map(|e| e.to_string());
        assert!(e.is_some_and(|e| e.contains("rz-ghidra")));
        return Ok(());
    }
    let d = decompile(&bin, 0x11d9)?;
    let code: Vec<&str> = d.lines.iter().map(|l| l.text.as_str()).collect();
    let code = code.join("\n");
    assert!(code.contains("check_password"), "{code}");
    assert!(code.contains("strcmp"), "{code}");
    assert!(
        d.lines.iter().any(|l| l.addr.is_some()),
        "lines carry addresses"
    );
    Ok(())
}
