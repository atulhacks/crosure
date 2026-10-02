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
}
