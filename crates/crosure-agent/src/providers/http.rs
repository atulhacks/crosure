use std::time::Duration;

use serde_json::Value;

use crate::AgentError;

/// Shared blocking HTTP client (no retries here: see [`Http::call`]).
pub(crate) struct Http {
    client: reqwest::blocking::Client,
}

fn message(body: &Value) -> String {
    body["error"]["message"]
        .as_str()
        .or_else(|| body["error"].as_str())
        .or_else(|| body["message"].as_str())
        .unwrap_or("unknown error")
        .to_string()
}

impl Http {
    pub(crate) fn new() -> Result<Self, AgentError> {
        let client = reqwest::blocking::Client::builder()
            .timeout(Duration::from_secs(600))
            .build()
            .map_err(|e| AgentError::Network(e.to_string()))?;
        Ok(Self { client })
    }

    fn send(
        &self,
        req: reqwest::blocking::RequestBuilder,
    ) -> Result<(u16, Option<u64>, Value), AgentError> {
        let resp = req.send().map_err(|e| AgentError::Network(e.to_string()))?;
        let status = resp.status().as_u16();
        let retry_after = resp
            .headers()
            .get("retry-after")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.parse().ok());
        let text = resp
            .text()
            .map_err(|e| AgentError::Network(e.to_string()))?;
        let json = serde_json::from_str(&text).unwrap_or(Value::String(text));
        Ok((status, retry_after, json))
    }

    /// GET or POST (when `body` is set) with headers; returns the JSON body.
    /// One attempt: temporary failures come back as
    /// [`AgentError::Transient`] / [`AgentError::Network`] and the agent loop
    /// decides whether to retry, so it can show the wait and honour Stop.
    pub(crate) fn call(
        &self,
        url: &str,
        headers: &[(&str, String)],
        body: Option<&Value>,
    ) -> Result<Value, AgentError> {
        let mut req = match body {
            Some(b) => self.client.post(url).json(b),
            None => self.client.get(url),
        };
        for (k, v) in headers {
            req = req.header(*k, v);
        }
        let (status, retry_after, json) = self.send(req)?;
        match status {
            200 => Ok(json),
            401 | 403 => Err(AgentError::Auth(message(&json))),
            408 | 429 | 500 | 502 | 503 | 504 | 529 => Err(AgentError::Transient {
                status,
                message: message(&json),
                retry_after,
            }),
            _ => Err(AgentError::Api {
                status,
                message: message(&json),
            }),
        }
    }
}
