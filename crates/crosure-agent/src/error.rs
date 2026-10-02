use thiserror::Error;

/// Errors that stop an agent run.
#[derive(Debug, Error)]
pub enum AgentError {
    /// No API key is configured.
    #[error("no Anthropic API key configured")]
    NoApiKey,
    /// The key was rejected (HTTP 401/403).
    #[error("the Anthropic API rejected the key: {0}")]
    Auth(String),
    /// The API returned an error after retries.
    #[error("Anthropic API error ({status}): {message}")]
    Api { status: u16, message: String },
    /// Network failure.
    #[error("network error: {0}")]
    Network(String),
    /// The response did not have the expected shape.
    #[error("unexpected response: {0}")]
    Protocol(String),
}
