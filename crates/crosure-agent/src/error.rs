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
    /// Network failure.
    #[error("network error: {0}")]
    Network(String),
    /// The response did not have the expected shape.
    #[error("unexpected response: {0}")]
    Protocol(String),
}
