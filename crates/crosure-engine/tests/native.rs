use std::path::PathBuf;

use crosure_engine::{Engine, EngineError, NativeEngine, XrefKind};

fn fixture(name: &str) -> Result<NativeEngine, EngineError> {
    let p = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name);
    NativeEngine::open(&p)
}

#[test]
fn elf_info() -> Result<(), EngineError> {
    let e = fixture("crackme-x64")?;
    let info = e.info()?;
    assert_eq!(
        (info.format.as_str(), info.arch.as_str(), info.bits),
        ("elf", "x86_64", 64)
    );
    assert!(!info.stripped);
    assert!(info.sha256.starts_with("sha256:"));
    assert!(info
        .sections
        .iter()
        .any(|s| s.name == ".text" && s.executable));
    Ok(())
}

#[test]
fn elf_functions_from_symbols() -> Result<(), EngineError> {
    let e = fixture("crackme-x64")?;
    let check = e
        .resolve("check_password")?
        .ok_or(EngineError::Unmapped(0))?;
    let f = e
        .function_at(check + 4)?
        .ok_or(EngineError::Unmapped(check))?;
    assert_eq!(f.name, "check_password");
    assert_eq!(f.source, "symbol");
    assert!(e.functions()?.windows(2).all(|w| w[0].addr < w[1].addr));
    Ok(())
}

#[test]
fn elf_imports_resolve_to_plt_stubs() -> Result<(), EngineError> {
    let e = fixture("crackme-x64")?;
    let imports = e.imports()?;
    let strcmp = imports
        .iter()
        .find(|i| i.name == "strcmp")
        .ok_or(EngineError::Unmapped(0))?;
    assert_eq!(strcmp.library, "libc.so.6");
    let stub = strcmp.addr.ok_or(EngineError::Unmapped(0))?;
    let callers = e.xrefs_to(stub)?;
    assert!(callers
        .iter()
        .any(|x| x.kind == XrefKind::Call && x.from_func.as_deref() == Some("check_password")));
    Ok(())
}

#[test]
fn disasm_is_annotated() -> Result<(), EngineError> {
    let e = fixture("crackme-x64")?;
    let check = e
        .resolve("check_password")?
        .ok_or(EngineError::Unmapped(0))?;
    let insns = e.disasm_function(check)?;
    let comments: Vec<String> = insns.iter().filter_map(|i| i.comment.clone()).collect();
    assert!(comments.contains(&"decode".to_string()), "{comments:?}");
    assert!(comments.contains(&"strcmp@plt".to_string()), "{comments:?}");
    assert_eq!(insns.last().map(|i| i.mnemonic.as_str()), Some("ret"));
    Ok(())
}

#[test]
fn strings_and_data_xrefs() -> Result<(), EngineError> {
    let e = fixture("crackme-x64")?;
    let wrong = e
        .strings(4)?
        .into_iter()
        .find(|s| s.value == "Wrong password")
        .ok_or(EngineError::Unmapped(0))?;
    assert!(wrong.mapped);
    assert_eq!(wrong.section.as_deref(), Some(".rodata"));
    let refs = e.xrefs_to(wrong.addr)?;
    assert!(refs
        .iter()
        .any(|x| x.kind == XrefKind::Data && x.from_func.as_deref() == Some("main")));
    Ok(())
}

#[test]
fn stripped_elf_still_finds_code() -> Result<(), EngineError> {
    let e = fixture("crackme-x64-stripped")?;
    assert!(e.info()?.stripped);
    let funcs = e.functions()?;
    // main (0x1227) is never called; `.eh_frame` and `lea rdi, [rip + main]` find it
    assert!(funcs.iter().any(|f| f.addr == 0x1227 && f.size == 144));
    assert!(funcs.iter().any(|f| f.name == "strcmp@plt"));
    let strcmp = e.resolve("strcmp")?.ok_or(EngineError::Unmapped(0))?;
    let caller = e
        .xrefs_to(strcmp)?
        .first()
        .cloned()
        .ok_or(EngineError::Unmapped(strcmp))?;
    let check = e
        .function_at(caller.from)?
        .ok_or(EngineError::Unmapped(caller.from))?;
    assert!(e.xrefs_from(check.addr)?.iter().any(|x| x.to == strcmp));
    Ok(())
}

#[test]
fn pe_imports_and_strings() -> Result<(), EngineError> {
    let e = fixture("crackme-x64.exe")?;
    let info = e.info()?;
    assert_eq!((info.format.as_str(), info.arch.as_str()), ("pe", "x86_64"));
    let imports = e.imports()?;
    let strcmp = imports
        .iter()
        .find(|i| i.name == "strcmp")
        .ok_or(EngineError::Unmapped(0))?;
    assert!(strcmp.library.eq_ignore_ascii_case("msvcrt.dll"));
    let addr = strcmp.addr.ok_or(EngineError::Unmapped(0))?;
    assert!(!e.xrefs_to(addr)?.is_empty());
    assert!(e
        .strings(6)?
        .iter()
        .any(|s| s.value.contains("c2.example.invalid")));
    Ok(())
}

#[test]
fn read_bytes_and_errors() -> Result<(), EngineError> {
    let e = fixture("crackme-x64")?;
    let entry = e.info()?.entry;
    assert_eq!(e.read_bytes(entry, 4)?, vec![0xf3, 0x0f, 0x1e, 0xfa]); // endbr64
    assert!(matches!(
        e.read_bytes(0xdead_0000, 4),
        Err(EngineError::Unmapped(_))
    ));
    assert!(matches!(
        NativeEngine::from_bytes("x", b"not a binary".to_vec()),
        Err(EngineError::Parse(_))
    ));
    Ok(())
}

#[test]
fn cfg_of_main_has_branches() -> Result<(), EngineError> {
    use crosure_engine::EdgeKind;
    let e = fixture("crackme-x64")?;
    let main = e.resolve("main")?.ok_or(EngineError::Unmapped(0))?;
    let blocks = e.function_graph(main)?;
    assert!(blocks.len() >= 4, "{blocks:?}");
    assert_eq!(blocks[0].addr, main);
    let taken = blocks
        .iter()
        .flat_map(|b| &b.succs)
        .filter(|s| s.kind == EdgeKind::Taken)
        .count();
    assert_eq!(taken, 2, "argc check + password check");
    let total: usize = blocks.iter().map(|b| b.count).sum();
    assert_eq!(total, e.disasm_function(main)?.len());
    let check = e
        .resolve("check_password")?
        .ok_or(EngineError::Unmapped(0))?;
    assert_eq!(e.function_graph(check)?.len(), 1, "straight-line function");
    Ok(())
}
