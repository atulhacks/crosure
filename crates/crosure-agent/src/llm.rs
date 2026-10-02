use std::time::Duration;

use serde_json::Value;

use crate::AgentError;

/// The model the agent uses unless configured otherwise.
pub const DEFAULT_MODEL: &str = "claude-opus-5-5";

const API_VERSION: &str = "2023-06-01";
/// Server-side fallback on safety declines, routed by refusal category.
const FALLBACK_BETA: &str = "server-side-fallback-2026-07-01";

/// A Messages API endpoint: takes a request body, returns the response body.
pub trait Llm: Send + Sync {
    /// Sends one `POST /v1/messages` request.
    fn create(&self, body: &Value) -> Result<Value, AgentError>;
    /// The model id requests are sent with.
    fn model(&self) -> &str;
}

/// Claude over raw HTTP (Rust has no official Anthropic SDK).
pub struct ClaudeHttp {
    api_key: String,
    base_url: String,
    model: String,
    http: reqwest::blocking::Client,
}

impl ClaudeHttp {
    /// A client for `model`. `base_url` defaults to `https://api.anthropic.com`.
    pub fn new(api_key: &str, model: &str, base_url: Option<&str>) -> Result<Self, AgentError> {
        if api_key.trim().is_empty() {
            return Err(AgentError::NoApiKey);
        }
        let http = reqwest::blocking::Client::builder()
            .timeout(Duration::from_secs(600))
            .build()
            .map_err(|e| AgentError::Network(e.to_string()))?;
        Ok(Self {
            api_key: api_key.trim().to_string(),
            base_url: base_url
                .unwrap_or("https://api.anthropic.com")
                .trim_end_matches('/')
                .to_string(),
            model: model.to_string(),
            http,
        })
    }

    fn send_once(&self, body: &Value) -> Result<(u16, Option<u64>, Value), AgentError> {
        let resp = self
            .http
            .post(format!("{}/v1/messages", self.base_url))
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", API_VERSION)
            .header("anthropic-beta", FALLBACK_BETA)
            .header("content-type", "application/json")
            .json(body)
            .send()
            .map_err(|e| AgentError::Network(e.to_string()))?;
        let status = resp.status().as_u16();
        let retry_after = resp
            .headers()
            .get("retry-after")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.parse::<u64>().ok());
        let json: Value = resp
            .json()
            .map_err(|e| AgentError::Protocol(e.to_string()))?;
        Ok((status, retry_after, json))
    }
}

fn error_message(body: &Value) -> String {
    body["error"]["message"]
        .as_str()
        .unwrap_or("unknown error")
        .to_string()
}

impl Llm for ClaudeHttp {
    fn create(&self, body: &Value) -> Result<Value, AgentError> {
        let mut delay = 2u64;
        for attempt in 0..4 {
            let (status, retry_after, json) = match self.send_once(body) {
                Ok(r) => r,
                Err(AgentError::Network(_)) if attempt < 3 => {
                    std::thread::sleep(Duration::from_secs(delay));
                    delay *= 2;
                    continue;
                }
                Err(e) => return Err(e),
            };
            match status {
                200 => return Ok(json),
                401 | 403 => return Err(AgentError::Auth(error_message(&json))),
                429 | 500..=599 if attempt < 3 => {
                    std::thread::sleep(Duration::from_secs(retry_after.unwrap_or(delay).min(60)));
                    delay *= 2;
                }
                _ => {
                    return Err(AgentError::Api {
                        status,
                        message: error_message(&json),
                    })
                }
            }
        }
        Err(AgentError::Network("gave up after retries".into()))
    }

    fn model(&self) -> &str {
        &self.model
    }
}
