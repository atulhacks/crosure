use crosure_dataset::{
    export, preference_pairs, sft_examples, trajectory, ExportOptions, PairRule, SftOptions,
};
use crosure_recorder::{Intent, NewStep, ParentRef, Relation, StepKind, Store, CONTEXT_CHIP};

type R = Result<(), Box<dyn std::error::Error>>;

fn step(kind: StepKind, cmd: &str) -> NewStep {
    let mut s = NewStep::human(kind, "crosure");
    s.command = Some(cmd.into());
    s.observation.summary = format!("result of {cmd}");
    s
}

/// load → str http → xt http_get (abandoned); branch back to `str http` →
/// dis check_password → finding.
fn branched(store: &Store) -> Result<String, Box<dyn std::error::Error>> {
    let s = store.create_session("crackme", "/home/me/secret/crackme-x64", "ab")?;
    store.record(
        &s.id,
        step(StepKind::Load, "open /home/me/secret/crackme-x64"),
    )?;
    let mut ctx = step(StepKind::Disasm, "dis main");
    ctx.intent = Some(Intent {
        chip: Some(CONTEXT_CHIP.into()),
        note: None,
    });
    store.record(&s.id, ctx)?;
    let strs = store.record(&s.id, step(StepKind::Strings, "str http"))?;
    store.record(&s.id, step(StepKind::Xref, "xt http_get"))?;
    let mut b = step(StepKind::Disasm, "dis check_password");
    b.parents = vec![ParentRef {
        id: strs.id.clone(),
        rel: Relation::Branch,
    }];
    store.record(&s.id, b)?;
    store.record(
        &s.id,
        step(StepKind::Finding, "find compares input with a decoded key"),
    )?;
    Ok(s.id)
}

#[test]
fn branch_becomes_a_preference_pair() -> R {
    let store = Store::open_in_memory()?;
    let id = branched(&store)?;
    let t = trajectory(&store.session(&id)?, &store.steps(&id)?, true);
    assert_eq!(
        t.binary, "crackme-x64",
        "local path is reduced to the file name"
    );

    let pairs = preference_pairs(&t, 40);
    assert_eq!(pairs.len(), 1);
    let p = &pairs[0];
    assert_eq!(p.meta.rule, PairRule::Branch);
    assert_eq!(
        (p.chosen.as_str(), p.rejected.as_str()),
        ("dis check_password", "xt http_get")
    );
    assert!(p.prompt.contains("#2 str http"));
    assert!(
        !p.prompt.contains("xt http_get"),
        "the prompt stops at the branch point"
    );

    let sft = sft_examples(&t, &SftOptions::default());
    let answers: Vec<&str> = sft.iter().map(|e| e.messages[2].content.as_str()).collect();
    assert_eq!(
        answers,
        [
            "str http",
            "dis check_password",
            "find compares input with a decoded key"
        ]
    );
    let all = sft_examples(
        &t,
        &SftOptions {
            key_path_only: false,
            ..Default::default()
        },
    );
    assert_eq!(
        all.len(),
        4,
        "the load and the @mention context are not targets"
    );
    assert_eq!(t.steps[0].command.as_deref(), Some("open crackme-x64"));
    Ok(())
}

#[test]
fn tampered_sessions_are_skipped() -> R {
    let dir = tempfile::tempdir()?;
    let db = dir.path().join("crosure.db");
    let (good, bad) = {
        let store = Store::open(&db)?;
        (branched(&store)?, branched(&store)?)
    };
    let conn = rusqlite::Connection::open(&db)?;
    conn.execute(
        "UPDATE steps SET json = replace(json, 'xt http_get', 'xt something_else') WHERE session_id = ?1",
        [&bad],
    )?;
    drop(conn);

    let store = Store::open(&db)?;
    let out = dir.path().join("out");
    let m = export(&store, &out, &ExportOptions::default())?;
    assert_eq!(m.trajectories, 1);
    let skipped: Vec<_> = m.sessions.iter().filter(|s| s.skipped.is_some()).collect();
    assert_eq!(skipped.len(), 1);
    assert_eq!(skipped[0].session_id, bad);
    let lines = std::fs::read_to_string(out.join("trajectories.jsonl"))?;
    assert_eq!(lines.lines().count(), 1);
    assert!(lines.contains(&good));
    assert!(!lines.contains("/home/me"), "no local paths in the export");

    let m = export(
        &store,
        &out,
        &ExportOptions {
            allow_unverified: true,
            ..Default::default()
        },
    )?;
    assert_eq!(m.trajectories, 2);
    Ok(())
}
