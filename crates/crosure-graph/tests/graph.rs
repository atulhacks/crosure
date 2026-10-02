use crosure_graph::build;
use crosure_recorder::{Intent, NewStep, RecorderError, Relation, Step, StepKind, Store, Target};

fn at(kind: StepKind, func: &str) -> NewStep {
    let mut s = NewStep::human(kind, "crosure");
    s.target = Some(Target {
        func: Some(func.into()),
        ..Default::default()
    });
    s
}

/// load → strings → xref ─→ decompile A (dead end, via annotation)
///                     └─(branch)→ decompile B → finding
fn scenario() -> Result<Vec<Step>, RecorderError> {
    let store = Store::open_in_memory()?;
    let s = store.create_session("demo", "/tmp/a", "00")?;
    let load = store.record(&s.id, NewStep::human(StepKind::Load, "crosure"))?;
    let strings = store.record(&s.id, NewStep::human(StepKind::Strings, "crosure"))?;
    let xref = store.record(
        &s.id,
        at(StepKind::Xref, "url").parent(&strings.id, Relation::DerivedFrom),
    )?;
    let dead = store.record(
        &s.id,
        at(StepKind::Decompile, "sub_a").parent(&xref.id, Relation::DerivedFrom),
    )?;
    let mut note =
        NewStep::human(StepKind::Annotate, "crosure").parent(&dead.id, Relation::Annotates);
    note.tags = vec!["dead_end".into()];
    note.intent = Some(Intent {
        chip: Some("find_c2".into()),
        note: Some("just logging".into()),
    });
    store.record(&s.id, note)?;
    let good = store.record(
        &s.id,
        at(StepKind::Decompile, "sub_b").parent(&xref.id, Relation::Branch),
    )?;
    store.record(
        &s.id,
        at(StepKind::Finding, "sub_b").parent(&good.id, Relation::DerivedFrom),
    )?;
    let _ = load;
    store.steps(&s.id)
}

#[test]
fn folds_annotations_into_their_step() -> Result<(), RecorderError> {
    let g = build(&scenario()?);
    assert_eq!(g.nodes.len(), 6, "annotation is not a node");
    let dead = g
        .nodes
        .iter()
        .find(|n| n.label == "decompile sub_a")
        .ok_or(RecorderError::UnknownParent("a".into()))?;
    assert_eq!(dead.tags, vec!["dead_end".to_string()]);
    assert_eq!(
        dead.intent.as_ref().and_then(|i| i.chip.as_deref()),
        Some("find_c2")
    );
    Ok(())
}

#[test]
fn edges_are_typed_and_skip_annotations() -> Result<(), RecorderError> {
    let g = build(&scenario()?);
    let ids: Vec<&str> = g.nodes.iter().map(|n| n.id.as_str()).collect();
    assert!(g
        .edges
        .iter()
        .all(|e| ids.contains(&e.from.as_str()) && ids.contains(&e.to.as_str())));
    assert_eq!(
        g.edges.iter().filter(|e| e.rel == Relation::Branch).count(),
        1
    );
    assert!(g.edges.iter().all(|e| e.rel != Relation::Annotates));
    Ok(())
}

#[test]
fn key_path_excludes_dead_end() -> Result<(), RecorderError> {
    let g = build(&scenario()?);
    let on: Vec<&str> = g
        .nodes
        .iter()
        .filter(|n| n.on_key_path)
        .map(|n| n.label.as_str())
        .collect();
    assert_eq!(
        on,
        vec![
            "load",
            "strings",
            "xref url",
            "decompile sub_b",
            "finding sub_b"
        ]
    );
    Ok(())
}

#[test]
fn stats_and_replay() -> Result<(), RecorderError> {
    let steps = scenario()?;
    let g = build(&steps);
    assert_eq!(g.stats.steps, 6);
    assert_eq!(g.stats.dead_ends, 1);
    assert_eq!(g.stats.findings, 1);
    assert_eq!(g.stats.branches, 1);
    assert_eq!(g.stats.by_kind.get("decompile"), Some(&2));
    let early = crosure_graph::replay(&steps, 2);
    assert_eq!(early.nodes.len(), 3);
    assert!(early.nodes.iter().all(|n| !n.on_key_path));
    Ok(())
}
