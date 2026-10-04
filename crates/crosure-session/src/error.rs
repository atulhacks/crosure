use thiserror::Error;

/// Errors from running or recording an op.
#[derive(Debug, Error)]
pub enum SessionError {
    #[error(transparent)]
    Engine(#[from] crosure_engine::EngineError),
    #[error(transparent)]
    Recorder(#[from] crosure_recorder::RecorderError),
    #[error("serialization error: {0}")]
    Json(#[from] serde_json::Error),
    /// A name or address the engine cannot resolve.
    #[error("cannot resolve `{0}`")]
    Unresolved(String),
    /// A console line that does not parse.
    #[error("bad command: {0}")]
    BadCommand(String),
    /// The file at the session's path is no longer the binary it recorded.
    #[error("{path} changed since the session was recorded (expected {expected}, found {found})")]
    BinaryChanged {
        path: String,
        expected: String,
        found: String,
    },
}
