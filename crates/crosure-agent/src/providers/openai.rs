use std::sync::atomic::{AtomicBool, Ordering};

use serde_json::{json, Value};

use super::extras::Extras;
use super::http::Http;
use super::limits::ModelLimits;
use super::sse::{rejects_streaming, stream};
use super::stream_openai::OpenAiStream;
use super::Provider;
use crate::tools::tool_definitions_for;
use crate::Profile;
use crate::{AgentError, Block, Entry, Sink, Stop, Transcript, Turn};

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

fn tools(profile: Profile, strict: bool) -> Vec<Value> {
    tool_definitions_for(profile)
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

/// What a reasoning model needs back on its own tool-call turns: the
/// reasoning text (Kimi, DeepSeek and GLM thinking modes reject the request
/// without it) and each call's `extra_content` (Gemini thought signatures).
/// Only echoed to the provider that produced it, in the field it used.
fn echo_own(m: &mut Value, raw: &Value) {
    for key in ["reasoning_content", "reasoning"] {
        if raw[key].as_str().is_some_and(|r| !r.is_empty()) {
            m[key] = raw[key].clone();
        }
    }
    // OpenRouter's structured reasoning (encrypted blocks, signatures).
    if raw["reasoning_details"]
        .as_array()
        .is_some_and(|d| !d.is_empty())
    {
        m["reasoning_details"] = raw["reasoning_details"].clone();
    }
    let raw_calls = raw["tool_calls"].as_array().cloned().unwrap_or_default();
    if let Some(calls) = m["tool_calls"].as_array_mut() {
        for c in calls {
            let extra = raw_calls
                .iter()
                .find(|r| r["id"] == c["id"])
                .map(|r| r["extra_content"].clone())
                .filter(|e| !e.is_null());
            if let Some(e) = extra {
                c["extra_content"] = e;
            }
        }
    }
}

/// Renders a transcript as an OpenAI-style Chat Completions request for
/// provider `self_id` (OpenAI, Gemini's compatibility endpoint, DeepSeek,
/// Z.ai, Moonshot, xAI, Qwen, OpenRouter, Groq, Mistral, Ollama, LM Studio,
/// vLLM, llama.cpp …).
///
/// ```
/// use crosure_agent::{openai_request, Entry, Transcript};
/// let t = Transcript { entries: vec![Entry::User("hi".into())], ..Default::default() };
/// let body = openai_request("ollama", "qwen2.5-coder:14b", &t, false);
/// assert_eq!(body["messages"][0]["role"], "system");
/// assert_eq!(body["tools"][0]["type"], "function");
/// ```
pub fn openai_request(self_id: &str, model: &str, t: &Transcript, strict: bool) -> Value {
    let mut messages = vec![json!({ "role": "system", "content": t.system_prompt() })];
    for e in &t.entries {
        match e {
            Entry::User(text) => messages.push(json!({ "role": "user", "content": text })),
            Entry::Assistant { provider, turn } => {
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
                    if provider == self_id {
                        echo_own(&mut m, &turn.raw);
                    }
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
    let mut body = json!({ "model": model, "messages": messages, "max_tokens": 8192 });
    let tools = tools(t.profile, strict);
    if !tools.is_empty() {
        body["tools"] = json!(tools);
        body["tool_choice"] = json!("auto");
    }
    body
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
        // `prompt_tokens` already includes the cached part.
        input_tokens: u["prompt_tokens"].as_u64().unwrap_or(0),
        output_tokens: u["completion_tokens"].as_u64().unwrap_or(0),
        cache_read_tokens: u["prompt_tokens_details"]["cached_tokens"]
            .as_u64()
            .or_else(|| u["prompt_cache_hit_tokens"].as_u64())
            .unwrap_or(0),
        cache_write_tokens: 0,
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
    extras: Extras,
    limits: ModelLimits,
    stream: bool,
    /// Set once the server refused a streaming request.
    no_stream: AtomicBool,
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
            extras: Extras::default(),
            limits: ModelLimits::default(),
            stream: true,
            no_stream: AtomicBool::new(false),
            http: Http::new()?,
        })
    }

    /// Sets custom headers, reasoning effort and the output-limit field.
    pub fn with_extras(mut self, extras: Extras) -> Self {
        self.extras = extras;
        self
    }

    /// Sets the model's limits; `max_output` becomes the output limit.
    pub fn with_limits(mut self, limits: ModelLimits) -> Self {
        self.limits = limits;
        self
    }

    /// Streams replies (on by default); off, each reply arrives whole.
    pub fn with_stream(mut self, on: bool) -> Self {
        self.stream = on;
        self
    }

    fn request(&self, t: &Transcript) -> (String, Vec<(&str, String)>, Value) {
        let mut body = openai_request(&self.id, &self.model, t, self.strict);
        if let Some(n) = self.limits.max_output {
            body["max_tokens"] = json!(n);
        }
        self.extras.apply_openai(&mut body);
        let url = format!("{}/chat/completions", self.base_url);
        let headers = self
            .extras
            .with_headers(Self::headers(self.api_key.as_deref()));
        (url, headers, body)
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
    fn limits(&self) -> ModelLimits {
        self.limits
    }
    fn next_live(&self, t: &Transcript, sink: &dyn Sink) -> Result<Turn, AgentError> {
        if !self.stream || self.no_stream.load(Ordering::Relaxed) {
            return self.next(t);
        }
        let (url, headers, mut body) = self.request(t);
        body["stream"] = json!(true);
        body["stream_options"] = json!({ "include_usage": true });
        let mut acc = OpenAiStream::default();
        let read = stream(self.http.post(&url, &headers, &body), sink, &mut |d| {
            if acc.feed(d)? {
                sink.live(&acc.live);
            }
            Ok(())
        });
        match read {
            Err(e) if rejects_streaming(&e) => {
                self.no_stream.store(true, Ordering::Relaxed);
                self.next(t)
            }
            Err(e) => Err(e),
            Ok(()) => Ok(parse_openai(&acc.finish()?)),
        }
    }
    fn next(&self, t: &Transcript) -> Result<Turn, AgentError> {
        let (url, headers, body) = self.request(t);
        let resp = self.http.call(&url, &headers, Some(&body))?;
        if resp["choices"].as_array().is_none_or(|c| c.is_empty()) {
            return Err(AgentError::Protocol(format!(
                "no choices in response: {}",
                resp.to_string().chars().take(300).collect::<String>()
            )));
        }
        Ok(parse_openai(&resp))
    }
}
