use std::time::Duration;

use serde_json::Value;

use crate::AgentError;

/// Shared blocking HTTP with retries on 429, 5xx and network errors.
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
    pub(crate) fn call(
        &self,
        url: &str,
        headers: &[(&str, String)],
        body: Option<&Value>,
    ) -> Result<Value, AgentError> {
        let mut delay = 2u64;
        for attempt in 0..4 {
            let mut req = match body {
                Some(b) => self.client.post(url).json(b),
                None => self.client.get(url),
            };
            for (k, v) in headers {
                req = req.header(*k, v);
            }
            match self.send(req) {
                Ok((200, _, json)) => return Ok(json),
                Ok((401 | 403, _, json)) => return Err(AgentError::Auth(message(&json))),
                Ok((429 | 500..=599, retry_after, _)) if attempt < 3 => {
                    std::thread::sleep(Duration::from_secs(retry_after.unwrap_or(delay).min(60)));
                    delay *= 2;
                }
                Ok((status, _, json)) => {
                    return Err(AgentError::Api {
                        status,
                        message: message(&json),
                    })
                }
                Err(AgentError::Network(_)) if attempt < 3 => {
                    std::thread::sleep(Duration::from_secs(delay));
                    delay *= 2;
                }
                Err(e) => return Err(e),
            }
        }
        Err(AgentError::Network("gave up after retries".into()))
    }
}
