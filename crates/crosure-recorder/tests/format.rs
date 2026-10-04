//! Store format versioning.

use crosure_recorder::{RecorderError, Store};

fn temp_db(name: &str) -> std::path::PathBuf {
    std::env::temp_dir().join(format!("crosure-{name}-{}.db", std::process::id()))
}

#[test]
fn a_pre_versioning_store_opens_and_keeps_its_sessions() -> Result<(), RecorderError> {
    let path = temp_db("v0");
    let _ = std::fs::remove_file(&path);
    let id = {
        let s = Store::open(&path)?;
        s.create_session("old", "/bin/true", "sha256:00")?.id
    };
    // Simulate a store written before versioning existed.
    rusqlite::Connection::open(&path)?.pragma_update(None, "user_version", 0)?;
    let s = Store::open(&path)?;
    assert_eq!(s.format_version()?, 2);
    assert_eq!(s.session(&id)?.name, "old");
    drop(s);
    let _ = std::fs::remove_file(&path);
    Ok(())
}

#[test]
fn a_store_from_a_newer_crosure_is_refused() -> Result<(), RecorderError> {
    let path = temp_db("v99");
    let _ = std::fs::remove_file(&path);
    drop(Store::open(&path)?);
    rusqlite::Connection::open(&path)?.pragma_update(None, "user_version", 99)?;
    let err = Store::open(&path).err();
    let _ = std::fs::remove_file(&path);
    assert!(
        matches!(err, Some(RecorderError::NewerFormat { found: 99, .. })),
        "{err:?}"
    );
    Ok(())
}

#[test]
fn the_fingerprint_index_exists_after_migration() -> Result<(), RecorderError> {
    let s = Store::open_in_memory()?;
    assert!(s
        .steps_with_fingerprint("fid1:0000000000000000/")?
        .is_empty());
    Ok(())
}
