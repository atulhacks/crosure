use crosure_recorder::{
    Intent, NewStep, Observation, RecorderError, Relation, StepKind, Store, Target,
};
use rusqlite::{params, Connection};

fn sample(store: &Store) -> Result<(String, Vec<String>), RecorderError> {
    let s = store.create_session("crackme", "/tmp/crackme", "sha256:aa")?;
    let a = store.record(&s.id, NewStep::human(StepKind::Load, "crosure"))?;
    let mut strings = NewStep::human(StepKind::Strings, "crosure");
    strings.observation = Observation {
        summary: "42 strings".into(),
        ..Default::default()
    };
    let b = store.record(&s.id, strings)?;
    let mut xref = NewStep::human(StepKind::Xref, "crosure").parent(&b.id, Relation::DerivedFrom);
    xref.target = Some(Target {
        addr: Some("0x401000".into()),
        ..Default::default()
    });
    xref.intent = Some(Intent {
        chip: Some("find_c2".into()),
        note: None,
    });
    let c = store.record(&s.id, xref)?;
    Ok((s.id, vec![a.id, b.id, c.id]))
}

#[test]
fn records_a_linked_chain() -> Result<(), RecorderError> {
    let store = Store::open_in_memory()?;
    let (sid, ids) = sample(&store)?;
    let steps = store.steps(&sid)?;
    assert_eq!(steps.len(), 3);
    assert_eq!(steps[1].prev_hash, steps[0].hash);
    assert_eq!(steps[2].prev_hash, steps[1].hash);
    // automatic `next` edge plus the explicit data edge
    assert_eq!(steps[2].parents.len(), 2);
    assert_eq!(steps[2].parents[0].id, ids[1]);
    assert_eq!(steps[2].parents[0].rel, Relation::Next);
    assert_eq!(steps[2].parents[1].rel, Relation::DerivedFrom);
    assert!(steps[0].parents.is_empty());
    assert_eq!(store.session(&sid)?.head_hash, steps[2].hash);
    Ok(())
}

#[test]
fn branch_parent_suppresses_automatic_next() -> Result<(), RecorderError> {
    let store = Store::open_in_memory()?;
    let (sid, ids) = sample(&store)?;
    let step = store.record(
        &sid,
        NewStep::human(StepKind::Decompile, "crosure").parent(&ids[0], Relation::Branch),
    )?;
    assert_eq!(step.parents.len(), 1);
    assert_eq!(step.parents[0].rel, Relation::Branch);
    Ok(())
}

#[test]
fn rejects_unknown_parent_and_session() -> Result<(), RecorderError> {
    let store = Store::open_in_memory()?;
    let (sid, _) = sample(&store)?;
    let bad = NewStep::human(StepKind::Xref, "crosure").parent("stp_nope", Relation::DerivedFrom);
    assert!(matches!(
        store.record(&sid, bad),
        Err(RecorderError::UnknownParent(_))
    ));
    let r = store.record("ses_nope", NewStep::human(StepKind::Load, "crosure"));
    assert!(matches!(r, Err(RecorderError::UnknownSession(_))));
    Ok(())
}

#[test]
fn verify_passes_on_untouched_chain() -> Result<(), RecorderError> {
    let store = Store::open_in_memory()?;
    let (sid, _) = sample(&store)?;
    let report = store.verify(&sid)?;
    assert!(report.ok, "{report:?}");
    assert_eq!(report.checked, 3);
    Ok(())
}

fn tampered(sql: &str) -> Result<crosure_recorder::VerifyReport, Box<dyn std::error::Error>> {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("c.db");
    let sid = {
        let store = Store::open(&path)?;
        sample(&store)?.0
    };
    Connection::open(&path)?.execute(sql, params![sid])?;
    Ok(Store::open(&path)?.verify(&sid)?)
}

#[test]
fn verify_detects_edited_content() -> Result<(), Box<dyn std::error::Error>> {
    let r = tampered(
        "UPDATE steps SET json = replace(json, '42 strings', '41 strings') WHERE session_id = ?1",
    )?;
    assert!(!r.ok);
    let f = r.failure.ok_or("no failure")?;
    assert_eq!(f.seq, 1);
    assert!(f.reason.contains("modified"));
    Ok(())
}

#[test]
fn verify_detects_deleted_tail() -> Result<(), Box<dyn std::error::Error>> {
    let r = tampered("DELETE FROM steps WHERE session_id = ?1 AND seq = 2")?;
    assert!(!r.ok);
    assert_eq!(r.checked, 2);
    Ok(())
}

#[test]
fn verify_detects_deleted_middle() -> Result<(), Box<dyn std::error::Error>> {
    let r = tampered("DELETE FROM steps WHERE session_id = ?1 AND seq = 1")?;
    assert!(!r.ok);
    assert_eq!(r.failure.ok_or("no failure")?.seq, 2);
    Ok(())
}

#[test]
fn persists_across_reopen() -> Result<(), Box<dyn std::error::Error>> {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("p.db");
    let sid = sample(&Store::open(&path)?)?.0;
    let store = Store::open(&path)?;
    assert_eq!(store.sessions()?.len(), 1);
    let next = store.record(&sid, NewStep::human(StepKind::Rename, "crosure"))?;
    assert_eq!(next.seq, 3);
    assert!(store.verify(&sid)?.ok);
    Ok(())
}
