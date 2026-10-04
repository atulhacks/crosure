use thiserror::Error;

/// Everything that can go wrong while recording or reading steps.
#[derive(Debug, Error)]
pub enum RecorderError {
    /// The underlying SQLite database failed.
    #[error("database error: {0}")]
    Db(#[from] rusqlite::Error),
    /// A step could not be (de)serialized.
    #[error("serialization error: {0}")]
    Json(#[from] serde_json::Error),
    /// The session id does not exist.
    #[error("unknown session: {0}")]
    UnknownSession(String),
    /// A parent reference points at a step that is not in the session.
    #[error("unknown parent step: {0}")]
    UnknownParent(String),
    /// The database was written by a newer Crosure.
    #[error(
        "store format {found} is newer than this Crosure supports ({supported}); update Crosure"
    )]
    NewerFormat { found: i64, supported: i64 },
}
