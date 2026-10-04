//! Splits never put the same code on both sides.

use std::path::PathBuf;

use crosure_dataset::{assign_splits, collect, export, ExportOptions, TrajStep, Trajectory};
use crosure_recorder::{ActorKind, StepKind, Store};
use crosure_session::{parse_command, Origin, Workspace};

type R = Result<(), Box<dyn std::error::Error>>;

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../crosure-engine/tests/fixtures")
        .join(name)
}

fn investigate(
    store: &Store,
    binary: &str,
    lines: &[&str],
) -> Result<String, Box<dyn std::error::Error>> {
    let (mut ws, _) = Workspace::open(store, &fixture(binary), None)?;
    for l in lines {
        ws.run(store, parse_command(l)?, None, Origin::Console)?;
    }
    Ok(ws.session.id.clone())
}

#[test]
fn copies_of_the_same_code_share_a_split() -> R {
    let store = Store::open_in_memory()?;
    let a = investigate(
        &store,
        "crackme-x64",
        &["dis decode", "find decode XORs with 0x43"],
    )?;
    let b = investigate(
        &store,
        "crackme-x64-stripped",
        &["dis 0x1189", "find sub_1189 decodes the secret"],
    )?;
    let c = investigate(
        &store,
        "crackme-x64.exe",
        &["imp", "find it imports strcmp"],
    )?;
    let (mut ts, _) = collect(&store, &ExportOptions::default())?;
    assign_splits(&mut ts, 50);
    let get = |id: &str| ts.iter().find(|t| t.session_id == id).ok_or("missing");
    let (ta, tb, tc) = (get(&a)?, get(&b)?, get(&c)?);
    assert_ne!(ta.binary_sha256, tb.binary_sha256, "different files");
    assert_eq!(ta.group, tb.group, "they share decode's code");
    assert_eq!(ta.split, tb.split);
    assert_ne!(ta.group, tc.group, "the PE shares no code with them");
    assert!(
        ta.steps.iter().any(|s| s.func_fp.is_some()),
        "steps carry fingerprints"
    );
    Ok(())
}

fn synthetic(id: &str, sha: &str, fps: &[&str]) -> Trajectory {
    let step = |fp: &&str| TrajStep {
        seq: 1,
        actor: ActorKind::Human,
        model: None,
        kind: StepKind::Disasm,
        command: Some("dis f".into()),
        target: None,
        why: None,
        summary: String::new(),
        tags: vec![],
        on_key_path: false,
        context: false,
        parents: vec![],
        hash: String::new(),
        func_fp: Some((*fp).to_string()),
    };
    Trajectory {
        format: "crosure.trajectory.v1".into(),
        session_id: id.into(),
        binary: id.into(),
        binary_sha256: sha.into(),
        head_hash: String::new(),
        verified: true,
        steps: fps.iter().map(step).collect(),
        group: String::new(),
        split: String::new(),
    }
}

#[test]
fn library_code_shared_by_many_binaries_does_not_merge_groups() {
    let libc = "fid1:1111111111111111/2222222222222222/40";
    let own = |n: u8| format!("fid1:{n:016x}/{n:016x}/40");
    let mut ts: Vec<Trajectory> = (0..4u8)
        .map(|i| synthetic(&format!("s{i}"), &format!("sha:{i}"), &[libc, &own(i + 1)]))
        .collect();
    // s4 shares real (non-library) code with s0 only.
    ts.push(synthetic("s4", "sha:4", &[&own(1)]));
    // Tiny functions never link anything.
    ts.push(synthetic(
        "s5",
        "sha:5",
        &["fid1:3333333333333333/3333333333333333/5"],
    ));
    ts.push(synthetic(
        "s6",
        "sha:6",
        &["fid1:3333333333333333/3333333333333333/5"],
    ));
    assign_splits(&mut ts, 50);
    let groups: std::collections::BTreeSet<&str> =
        ts[..4].iter().map(|t| t.group.as_str()).collect();
    assert_eq!(groups.len(), 4, "a function in 4 binaries is library code");
    assert_eq!(ts[0].group, ts[4].group, "real shared code joins");
    assert_ne!(
        ts[5].group, ts[6].group,
        "functions under 12 instructions do not"
    );
}

#[test]
fn identical_examples_are_exported_once() -> R {
    let store = Store::open_in_memory()?;
    for _ in 0..2 {
        investigate(
            &store,
            "crackme-x64",
            &["dis decode", "find decode XORs with 0x43"],
        )?;
    }
    let dir = tempfile::tempdir()?;
    let m = export(&store, dir.path(), &ExportOptions::default())?;
    assert!(m.sft_duplicates > 0, "{m:?}");
    assert_eq!(m.train.trajectories + m.test.trajectories, m.trajectories);
    let sessions: Vec<_> = m.sessions.iter().map(|s| (&s.group, &s.split)).collect();
    assert_eq!(
        sessions[0], sessions[1],
        "same binary, same group and split"
    );
    Ok(())
}
