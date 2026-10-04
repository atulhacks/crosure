use thiserror::Error;

/// Errors that stop an agent run.
#[derive(Debug, Error)]
pub enum AgentError {
    /// No API key is configured.
    #[error("no ready provider: add an API key or a local model in Agent settings")]
    NoApiKey,
    /// The key was rejected (HTTP 401/403).
    #[error("the provider rejected the API key: {0}")]
    Auth(String),
    /// The API returned an error after retries.
    #[error("API error ({status}): {message}")]
    Api { status: u16, message: String },
    /// A temporary server condition (rate limit, overload, gateway error);
    /// worth retrying, after `retry_after` seconds when the server says so.
    #[error("temporary API error ({status}): {message}")]
    Transient {
        status: u16,
        message: String,
        retry_after: Option<u64>,
    },
    /// Network failure.
    #[error("network error: {0}")]
    Network(String),
    /// The response did not have the expected shape.
    #[error("unexpected response: {0}")]
    Protocol(String),
}

impl AgentError {
    /// Whether retrying the same request may succeed.
    ///
    /// ```
    /// use crosure_agent::AgentError;
    /// assert!(AgentError::Network("reset".into()).is_transient());
    /// assert!(!AgentError::Auth("bad key".into()).is_transient());
    /// ```
    pub fn is_transient(&self) -> bool {
        matches!(self, AgentError::Transient { .. } | AgentError::Network(_))
    }

    /// Seconds the server asked us to wait, if any.
    pub fn retry_after(&self) -> Option<u64> {
        match self {
            AgentError::Transient { retry_after, .. } => *retry_after,
            _ => None,
        }
    }
}
