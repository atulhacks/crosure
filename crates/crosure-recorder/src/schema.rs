/// Store format version, kept in SQLite's `PRAGMA user_version`. Databases
/// from before versioning read as 0 and have the version-1 layout.
pub(crate) const SCHEMA_VERSION: i64 = 1;

/// `MIGRATIONS[i]` upgrades version `i + 1` to `i + 2`; each runs in one
/// transaction. Never rewrite a step's `json` or `hash`: the hash chain
/// covers them. Add columns or tables instead, and upcast old steps on read.
pub(crate) const MIGRATIONS: &[&str] = &[];

/// Tables: sessions, steps (append-only, one JSON document per step) and
/// content-addressed blobs for full tool outputs.
pub(crate) const SCHEMA_SQL: &str = "
CREATE TABLE IF NOT EXISTS sessions (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    binary_path TEXT NOT NULL,
    binary_sha256 TEXT NOT NULL,
    created_ms INTEGER NOT NULL,
    head_hash TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS steps (
    session_id TEXT NOT NULL,
    seq INTEGER NOT NULL,
    id TEXT NOT NULL UNIQUE,
    kind TEXT NOT NULL,
    json TEXT NOT NULL,
    hash TEXT NOT NULL,
    PRIMARY KEY (session_id, seq)
);
CREATE TABLE IF NOT EXISTS blobs (
    key TEXT PRIMARY KEY,
    data BLOB NOT NULL
);
";
