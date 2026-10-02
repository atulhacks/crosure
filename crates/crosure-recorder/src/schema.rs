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
