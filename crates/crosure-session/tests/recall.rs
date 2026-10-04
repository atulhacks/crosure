//! Knowledge recorded on one binary is found again on another copy of the code.

use std::path::PathBuf;

use crosure_recorder::{StepKind, Store};
use crosure_session::{parse_command, Origin, SessionError, Workspace};

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../crosure-engine/tests/fixtures")
        .join(name)
}

#[test]
fn a_rename_on_one_binary_is_recalled_on_its_stripped_copy() -> Result<(), SessionError> {
    let store = Store::open_in_memory()?;
    let (mut a, _) = Workspace::open(&store, &fixture("crackme-x64"), None)?;
    for line in [
        "dis decode",
        "ren decode xor_decode",
        "find decode XORs each byte with 0x43",
    ] {
        a.run(&store, parse_command(line)?, None, Origin::Console)?;
    }
    let renamed = store
        .steps(&a.session.id)?
        .into_iter()
        .find(|s| s.kind == StepKind::Rename)
        .and_then(|s| s.target)
        .and_then(|t| t.func_fp);
    assert!(
        renamed.is_some_and(|fp| fp.starts_with("fid1:")),
        "targets carry fingerprints"
    );

    let (mut b, _) = Workspace::open(&store, &fixture("crackme-x64-stripped"), None)?;
    let out = b.run(
        &store,
        parse_command("recall 0x1189")?,
        None,
        Origin::Console,
    )?;
    assert_eq!(out.step.kind, StepKind::Recall);
    let m = &out.result["matches"][0];
    assert_eq!(m["level"], "exact");
    assert_eq!(m["binary"], "crackme-x64");
    assert_eq!(m["renamed_to"][0], "xor_decode");
    assert!(
        m["notes"][0].as_str().is_some_and(|n| n.contains("0x43")),
        "{m}"
    );
    assert_eq!(out.result["ambiguous"], false);
    assert!(
        out.step.observation.summary.contains("xor_decode"),
        "{}",
        out.step.observation.summary
    );

    // A function no session has looked at (main, unnamed in the stripped copy).
    let none = b.run(
        &store,
        parse_command("recall 0x1227")?,
        None,
        Origin::Console,
    )?;
    assert_eq!(none.result["matches"].as_array().map(Vec::len), Some(0));
    // Its own session's steps never count as "earlier sessions".
    let own = a.run(
        &store,
        parse_command("recall xor_decode")?,
        None,
        Origin::Console,
    )?;
    assert!(own.result["matches"]
        .as_array()
        .is_some_and(|m| m.iter().all(|m| m["session"] != a.session.id.as_str())));
    Ok(())
}

#[test]
fn sessions_that_disagree_make_a_match_ambiguous() -> Result<(), SessionError> {
    let store = Store::open_in_memory()?;
    for name in ["xor_decode", "deobfuscate"] {
        let (mut ws, _) = Workspace::open(&store, &fixture("crackme-x64"), None)?;
        ws.run(
            &store,
            parse_command(&format!("ren decode {name}"))?,
            None,
            Origin::Console,
        )?;
    }
    let (mut b, _) = Workspace::open(&store, &fixture("crackme-x64-stripped"), None)?;
    let out = b.run(
        &store,
        parse_command("recall 0x1189")?,
        None,
        Origin::Console,
    )?;
    assert_eq!(out.result["matches"].as_array().map(Vec::len), Some(2));
    assert_eq!(out.result["ambiguous"], true);
    assert!(out.step.observation.summary.contains("ambiguous"));
    Ok(())
}
