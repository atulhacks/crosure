use std::path::PathBuf;

use crosure_recorder::{Intent, ParentRef, Relation, StepKind, Store};
use crosure_session::{parse_command, Op, Origin, SessionError, Workspace};

fn crackme() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../crosure-engine/tests/fixtures/crackme-x64")
}

#[test]
fn open_records_load_step() -> Result<(), SessionError> {
    let store = Store::open_in_memory()?;
    let (ws, load) = Workspace::open(&store, &crackme(), None)?;
    assert_eq!(ws.session.name, "crackme-x64");
    assert_eq!(load.kind, StepKind::Load);
    assert!(load.observation.summary.starts_with("ELF x86_64 64-bit"));
    assert!(store
        .get_blob(load.observation.blob.as_deref().unwrap_or(""))?
        .is_some());
    Ok(())
}

#[test]
fn analyst_flow_is_recorded_and_linked() -> Result<(), SessionError> {
    let store = Store::open_in_memory()?;
    let (mut ws, _) = Workspace::open(&store, &crackme(), None)?;
    let strings = ws.run(
        &store,
        Op::Strings {
            filter: Some("wrong".into()),
            min_len: None,
        },
        None,
        Origin::Ui,
    )?;
    assert!(
        strings.step.observation.summary.starts_with("1 strings"),
        "{}",
        strings.step.observation.summary
    );
    let addr = strings.result["strings"][0]["addr"]
        .as_u64()
        .ok_or(SessionError::Unresolved("addr".into()))?;

    let parent = Some(ParentRef {
        id: strings.step.id.clone(),
        rel: Relation::DerivedFrom,
    });
    let xrefs = ws.run(
        &store,
        Op::XrefsTo {
            target: format!("{addr:#x}"),
        },
        parent,
        Origin::Ui,
    )?;
    assert!(
        xrefs.step.observation.summary.contains("from main"),
        "{}",
        xrefs.step.observation.summary
    );
    assert_eq!(xrefs.step.parents.len(), 2);
    assert_eq!(
        xrefs.step.command.as_deref(),
        Some(format!("xt {addr:#x}").as_str())
    );

    let dis = ws.run(
        &store,
        Op::Disasm {
            target: "check_password".into(),
        },
        None,
        Origin::Console,
    )?;
    assert_eq!(dis.step.tool, "console");
    assert!(
        dis.step
            .observation
            .summary
            .contains("calls decode, strcmp@plt"),
        "{}",
        dis.step.observation.summary
    );
    assert_eq!(
        dis.step.target.as_ref().and_then(|t| t.func.as_deref()),
        Some("check_password")
    );

    ws.annotate(
        &store,
        &dis.step.id,
        Some(Intent {
            chip: Some("find_crypto".into()),
            note: None,
        }),
        vec!["key_step".into()],
    )?;
    assert!(store.verify(&ws.session.id)?.ok);
    assert_eq!(store.steps(&ws.session.id)?.len(), 5);
    Ok(())
}

#[test]
fn renames_apply_and_survive_resume() -> Result<(), SessionError> {
    let store = Store::open_in_memory()?;
    let (mut ws, _) = Workspace::open(&store, &crackme(), None)?;
    let out = ws.run(
        &store,
        Op::Rename {
            target: "decode".into(),
            name: "xor_decode".into(),
        },
        None,
        Origin::Ui,
    )?;
    assert_eq!(out.step.observation.summary, "renamed decode → xor_decode");
    assert_eq!(out.step.command.as_deref(), Some("ren decode xor_decode"));
    let by_addr = ws.run(
        &store,
        Op::Disasm {
            target: "0x11d9".into(),
        },
        None,
        Origin::Ui,
    )?;
    assert_eq!(
        by_addr.step.command.as_deref(),
        Some("dis check_password"),
        "commands use names"
    );
    let dis = ws.run(
        &store,
        Op::Disasm {
            target: "check_password".into(),
        },
        None,
        Origin::Ui,
    )?;
    assert!(dis.step.observation.summary.contains("xor_decode"));

    let resumed = Workspace::resume(&store, &ws.session.id)?;
    assert!(resumed.functions()?.iter().any(|f| f.name == "xor_decode"));
    assert_eq!(resumed.resolve("xor_decode")?, ws.resolve("xor_decode")?);
    Ok(())
}

#[test]
fn console_round_trip_and_errors() -> Result<(), SessionError> {
    for line in [
        "info",
        "dis main",
        "xt strcmp",
        "xf main",
        "str http",
        "imp",
        "hex 0x1189 16",
        "ren 0x1189 xor_decode",
        "cmt 0x1189 decodes the password",
        "hyp xor key is 0x43",
        "find password is r3s3ts3",
        "verdict benign - crackme",
    ] {
        let op = parse_command(line)?;
        assert_eq!(parse_command(&op.command())?, op, "{line}");
    }
    let store = Store::open_in_memory()?;
    let (mut ws, _) = Workspace::open(&store, &crackme(), None)?;
    let bad = ws.run(
        &store,
        Op::Disasm {
            target: "nope".into(),
        },
        None,
        Origin::Console,
    );
    assert!(matches!(bad, Err(SessionError::Unresolved(_))));
    assert_eq!(
        store.steps(&ws.session.id)?.len(),
        1,
        "failed ops are not recorded"
    );
    Ok(())
}
