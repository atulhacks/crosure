use serde_json::{json, Value};

use super::http::Http;
use super::Provider;
use crate::prompt::SYSTEM_PROMPT;
use crate::tools::tool_definitions;
use crate::{AgentError, Block, Entry, Stop, Transcript, Turn};

/// Makes a strict-mode schema acceptable to lenient servers: `["string","null"]`
/// becomes `"string"` (the tool parser already treats empty as "none").
fn relax(schema: &Value) -> Value {
    match schema {
        Value::Object(map) => {
            let mut out = serde_json::Map::new();
            for (k, v) in map {
                if k == "type" {
                    if let Some(types) = v.as_array() {
                        let first = types
                            .iter()
                            .find(|t| *t != "null")
                            .cloned()
                            .unwrap_or(json!("string"));
                        out.insert(k.clone(), first);
                        continue;
                    }
                }
                out.insert(k.clone(), relax(v));
            }
            Value::Object(out)
        }
        Value::Array(a) => Value::Array(a.iter().map(relax).collect()),
        other => other.clone(),
    }
}

fn tools(strict: bool) -> Vec<Value> {
    tool_definitions()
        .as_array()
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .map(|t| {
            let params = if strict {
                t["input_schema"].clone()
            } else {
                relax(&t["input_schema"])
            };
            let mut function =
                json!({ "name": t["name"], "description": t["description"], "parameters": params });
            if strict {
                function["strict"] = json!(true);
            }
            json!({ "type": "function", "function": function })
        })
        .collect()
}

/// Renders a transcript as an OpenAI-style Chat Completions request (OpenAI,
/// Gemini's compatibility endpoint, OpenRouter, Groq, DeepSeek, Mistral,
/// Ollama, LM Studio, vLLM, llama.cpp).
///
/// ```
/// use crosure_agent::{openai_request, Entry, Transcript};
/// let t = Transcript { entries: vec![Entry::User("hi".into())] };
/// let body = openai_request("qwen2.5-coder:14b", &t, false);
/// assert_eq!(body["messages"][0]["role"], "system");
/// assert_eq!(body["tools"][0]["type"], "function");
/// ```
pub fn openai_request(model: &str, t: &Transcript, strict: bool) -> Value {
    let mut messages = vec![json!({ "role": "system", "content": SYSTEM_PROMPT })];
    for e in &t.entries {
        match e {
            Entry::User(text) => messages.push(json!({ "role": "user", "content": text })),
            Entry::Assistant { turn, .. } => {
                let calls: Vec<Value> = turn
                    .blocks
                    .iter()
                    .filter_map(|b| match b {
                        Block::ToolUse { id, name, input } => Some(json!({
                            "id": id, "type": "function",
                            "function": { "name": name, "arguments": input.to_string() }
                        })),
                        _ => None,
                    })
                    .collect();
                let text = turn.text();
                let mut m = json!({ "role": "assistant", "content": if text.is_empty() { Value::Null } else { json!(text) } });
                if !calls.is_empty() {
                    m["tool_calls"] = json!(calls);
                }
                messages.push(m);
            }
            Entry::Results { results, note } => {
                for r in results {
                    let content = if r.is_error {
                        format!("ERROR: {}", r.content)
                    } else {
                        r.content.clone()
                    };
                    messages
                        .push(json!({ "role": "tool", "tool_call_id": r.id, "content": content }));
                }
                if let Some(n) = note {
                    messages.push(json!({ "role": "user", "content": n }));
                }
            }
        }
    }
    json!({ "model": model, "messages": messages, "tools": tools(strict), "tool_choice": "auto", "max_tokens": 8192 })
}

fn arguments(v: &Value) -> Value {
    match v {
        Value::String(s) => serde_json::from_str(s).unwrap_or_else(|_| json!({ "__unparsed": s })),
        other => other.clone(),
    }
}

/// Parses a Chat Completions response.
pub(crate) fn parse_openai(resp: &Value) -> Turn {
    let choice = &resp["choices"][0];
    let msg = &choice["message"];
    let mut blocks = Vec::new();
    for key in ["reasoning_content", "reasoning"] {
        if let Some(r) = msg[key].as_str().filter(|r| !r.trim().is_empty()) {
            blocks.push(Block::Thinking(r.to_string()));
        }
    }
    if let Some(t) = msg["content"].as_str() {
        blocks.push(Block::Text(t.to_string()));
    }
    let calls = msg["tool_calls"].as_array().cloned().unwrap_or_default();
    for (i, c) in calls.iter().enumerate() {
        blocks.push(Block::ToolUse {
            id: c["id"]
                .as_str()
                .map(str::to_string)
                .unwrap_or_else(|| format!("call_{i}")),
            name: c["function"]["name"].as_str().unwrap_or("").to_string(),
            input: arguments(&c["function"]["arguments"]),
        });
    }
    let refusal = msg["refusal"]
        .as_str()
        .filter(|r| !r.is_empty())
        .map(str::to_string);
    let stop = match (choice["finish_reason"].as_str(), refusal) {
        (_, Some(r)) => Stop::Refusal {
            category: None,
            explanation: Some(r),
        },
        (Some("content_filter"), None) => Stop::Refusal {
            category: Some("content_filter".into()),
            explanation: None,
        },
        (Some("length"), None) => Stop::MaxTokens,
        _ if !calls.is_empty() => Stop::ToolUse,
        _ => Stop::EndTurn,
    };
    let u = &resp["usage"];
    Turn {
        blocks,
        stop,
        input_tokens: u["prompt_tokens"].as_u64().unwrap_or(0),
        output_tokens: u["completion_tokens"].as_u64().unwrap_or(0),
        raw: msg.clone(),
    }
}

/// Any OpenAI-compatible Chat Completions endpoint.
pub struct OpenAiProvider {
    id: String,
    model: String,
    base_url: String,
    api_key: Option<String>,
    strict: bool,
    http: Http,
}

impl OpenAiProvider {
    /// `base_url` ends at the API root, e.g. `https://api.openai.com/v1` or `http://localhost:11434/v1`.
    pub fn new(
        id: &str,
        model: &str,
        base_url: &str,
        api_key: Option<&str>,
        strict: bool,
    ) -> Result<Self, AgentError> {
        Ok(Self {
            id: id.into(),
            model: model.into(),
            base_url: base_url.trim_end_matches('/').into(),
            api_key: api_key
                .map(|k| k.trim().to_string())
                .filter(|k| !k.is_empty()),
            strict,
            http: Http::new()?,
        })
    }

    pub(crate) fn headers(api_key: Option<&str>) -> Vec<(&'static str, String)> {
        let mut h = vec![("X-Title", "Crosure".to_string())];
        if let Some(k) = api_key {
            h.push(("Authorization", format!("Bearer {k}")));
        }
        h
    }
}

impl Provider for OpenAiProvider {
    fn id(&self) -> &str {
        &self.id
    }
    fn model(&self) -> &str {
        &self.model
    }
    fn next(&self, t: &Transcript) -> Result<Turn, AgentError> {
        let body = openai_request(&self.model, t, self.strict);
        let url = format!("{}/chat/completions", self.base_url);
        let resp = self
            .http
            .call(&url, &Self::headers(self.api_key.as_deref()), Some(&body))?;
        if resp["choices"].as_array().is_none_or(|c| c.is_empty()) {
            return Err(AgentError::Protocol(format!(
                "no choices in response: {}",
                resp.to_string().chars().take(300).collect::<String>()
            )));
        }
        Ok(parse_openai(&resp))
    }
}
