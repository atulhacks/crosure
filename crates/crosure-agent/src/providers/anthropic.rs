use serde_json::{json, Value};

use super::http::Http;
use super::Provider;
use crate::prompt::SYSTEM_PROMPT;
use crate::tools::tool_definitions;
use crate::{AgentError, Block, Entry, Stop, Transcript, Turn};

/// The default Claude model.
pub const DEFAULT_MODEL: &str = "claude-opus-5-5";
const API_VERSION: &str = "2023-06-01";
/// Server-side fallback on safety declines, routed by refusal category.
const FALLBACK_BETA: &str = "server-side-fallback-2026-07-01";

fn assistant_blocks(blocks: &[Block]) -> Vec<Value> {
    blocks
        .iter()
        .filter_map(|b| match b {
            Block::Text(t) => Some(json!({ "type": "text", "text": t })),
            Block::ToolUse { id, name, input } => {
                Some(json!({ "type": "tool_use", "id": id, "name": name, "input": input }))
            }
            Block::Thinking(_) => None,
        })
        .collect()
}

/// Renders a transcript as a Messages API request.
///
/// Turns this provider produced go back verbatim (thinking blocks included);
/// turns from another provider are rebuilt from their text and tool calls.
///
/// ```
/// use crosure_agent::{anthropic_request, Entry, Transcript};
/// let t = Transcript { entries: vec![Entry::User("hi".into())] };
/// let body = anthropic_request("anthropic", "claude-opus-5-5", &t);
/// assert_eq!(body["fallbacks"], "default");
/// assert_eq!(body["output_config"]["effort"], "high");
/// ```
pub fn anthropic_request(self_id: &str, model: &str, t: &Transcript) -> Value {
    let messages: Vec<Value> = t
        .entries
        .iter()
        .map(|e| match e {
            Entry::User(text) => json!({ "role": "user", "content": text }),
            Entry::Assistant { provider, turn } if provider == self_id && turn.raw.is_array() => {
                json!({ "role": "assistant", "content": turn.raw })
            }
            Entry::Assistant { turn, .. } => json!({ "role": "assistant", "content": assistant_blocks(&turn.blocks) }),
            Entry::Results { results, note } => {
                let mut content: Vec<Value> = results
                    .iter()
                    .map(|r| json!({ "type": "tool_result", "tool_use_id": r.id, "content": r.content, "is_error": r.is_error }))
                    .collect();
                if let Some(n) = note {
                    content.push(json!({ "type": "text", "text": n }));
                }
                json!({ "role": "user", "content": content })
            }
        })
        .collect();
    json!({
        "model": model,
        "max_tokens": 16000,
        "system": SYSTEM_PROMPT,
        "tools": tool_definitions(),
        "tool_choice": { "type": "auto" },
        "thinking": { "type": "adaptive", "display": "summarized" },
        "output_config": { "effort": "high" },
        "fallbacks": "default",
        "cache_control": { "type": "ephemeral" },
        "messages": messages,
    })
}

/// Parses a Messages API response.
pub(crate) fn parse_anthropic(resp: &Value) -> Turn {
    let content = resp["content"].as_array().cloned().unwrap_or_default();
    let blocks = content
        .iter()
        .filter_map(|b| match b["type"].as_str() {
            Some("text") => Some(Block::Text(b["text"].as_str().unwrap_or("").to_string())),
            Some("thinking") => b["thinking"]
                .as_str()
                .filter(|t| !t.trim().is_empty())
                .map(|t| Block::Thinking(t.to_string())),
            Some("tool_use") => Some(Block::ToolUse {
                id: b["id"].as_str().unwrap_or("").to_string(),
                name: b["name"].as_str().unwrap_or("").to_string(),
                input: b["input"].clone(),
            }),
            _ => None,
        })
        .collect();
    let stop = match resp["stop_reason"].as_str() {
        Some("tool_use") => Stop::ToolUse,
        Some("pause_turn") => Stop::PauseTurn,
        Some("max_tokens") => Stop::MaxTokens,
        Some("refusal") => Stop::Refusal {
            category: resp["stop_details"]["category"]
                .as_str()
                .map(str::to_string),
            explanation: resp["stop_details"]["explanation"]
                .as_str()
                .map(str::to_string),
        },
        _ => Stop::EndTurn,
    };
    let u = &resp["usage"];
    let input = [
        "input_tokens",
        "cache_read_input_tokens",
        "cache_creation_input_tokens",
    ]
    .iter()
    .map(|k| u[*k].as_u64().unwrap_or(0))
    .sum();
    Turn {
        blocks,
        stop,
        input_tokens: input,
        output_tokens: u["output_tokens"].as_u64().unwrap_or(0),
        raw: Value::Array(content),
    }
}

/// Claude over the Messages API (raw HTTP: Rust has no official Anthropic SDK).
pub struct AnthropicProvider {
    id: String,
    model: String,
    base_url: String,
    api_key: String,
    http: Http,
}

impl AnthropicProvider {
    /// A provider for `model`; `base_url` like `https://api.anthropic.com`.
    pub fn new(id: &str, model: &str, base_url: &str, api_key: &str) -> Result<Self, AgentError> {
        if api_key.trim().is_empty() {
            return Err(AgentError::NoApiKey);
        }
        Ok(Self {
            id: id.into(),
            model: model.into(),
            base_url: base_url.trim_end_matches('/').into(),
            api_key: api_key.trim().into(),
            http: Http::new()?,
        })
    }

    pub(crate) fn headers(api_key: &str) -> Vec<(&'static str, String)> {
        vec![
            ("x-api-key", api_key.to_string()),
            ("anthropic-version", API_VERSION.to_string()),
            ("anthropic-beta", FALLBACK_BETA.to_string()),
        ]
    }
}

impl Provider for AnthropicProvider {
    fn id(&self) -> &str {
        &self.id
    }
    fn model(&self) -> &str {
        &self.model
    }
    fn next(&self, t: &Transcript) -> Result<Turn, AgentError> {
        let body = anthropic_request(&self.id, &self.model, t);
        let url = format!("{}/v1/messages", self.base_url);
        let resp = self
            .http
            .call(&url, &Self::headers(&self.api_key), Some(&body))?;
        Ok(parse_anthropic(&resp))
    }
}
