//! Rebuilds a Chat Completions response from its stream chunks, so it is
//! parsed and echoed back like a non-streamed one (reasoning text, tool-call
//! `extra_content`, `reasoning_details`).

use serde_json::{json, Map, Value};

use crate::{AgentError, Live};

/// Accumulates one streamed choice.
#[derive(Default)]
pub(crate) struct OpenAiStream {
    message: Map<String, Value>,
    calls: Vec<Value>,
    details: Vec<Value>,
    finish: Option<Value>,
    usage: Value,
    done: bool,
    pub(crate) live: Live,
}

fn append(m: &mut Map<String, Value>, key: &str, text: &str) {
    let slot = m.entry(key).or_insert_with(|| json!(""));
    let mut s = slot.as_str().unwrap_or("").to_string();
    s.push_str(text);
    *slot = Value::String(s);
}

impl OpenAiStream {
    /// Feeds one chunk's `data:` payload. Returns whether the live view changed.
    pub(crate) fn feed(&mut self, data: &str) -> Result<bool, AgentError> {
        if data == "[DONE]" {
            self.done = true;
            return Ok(false);
        }
        let Ok(v) = serde_json::from_str::<Value>(data) else {
            return Ok(false);
        };
        if !v["error"].is_null() {
            let message = v["error"]["message"]
                .as_str()
                .or_else(|| v["error"].as_str())
                .unwrap_or("stream error")
                .to_string();
            // A provider failing mid-stream (gateways report it this way).
            return Err(AgentError::Transient {
                status: 502,
                message,
                retry_after: None,
            });
        }
        if v["usage"].is_object() {
            self.usage = v["usage"].clone();
        }
        let choice = &v["choices"][0];
        if !choice["finish_reason"].is_null() {
            self.finish = Some(choice["finish_reason"].clone());
        }
        let d = &choice["delta"];
        let mut changed = false;
        for key in ["content", "reasoning_content", "reasoning", "refusal"] {
            if let Some(t) = d[key].as_str().filter(|t| !t.is_empty()) {
                append(&mut self.message, key, t);
                if key == "content" {
                    self.live.text.push_str(t);
                } else if key != "refusal" {
                    self.live.thinking.push_str(t);
                }
                changed = true;
            }
        }
        if let Some(details) = d["reasoning_details"].as_array() {
            self.details.extend(details.iter().cloned());
        }
        for (n, tc) in d["tool_calls"].as_array().into_iter().flatten().enumerate() {
            let i = tc["index"]
                .as_u64()
                .and_then(|i| usize::try_from(i).ok())
                .unwrap_or(n);
            if self.calls.len() <= i {
                self.calls.resize(
                    i + 1,
                    json!({ "type": "function", "function": { "name": "", "arguments": "" } }),
                );
            }
            let call = &mut self.calls[i];
            for (k, val) in tc.as_object().into_iter().flatten() {
                match k.as_str() {
                    "index" => {}
                    "function" => {
                        if let Some(name) = val["name"].as_str().filter(|s| !s.is_empty()) {
                            if call["function"]["name"].as_str().is_none_or(str::is_empty) {
                                call["function"]["name"] = json!(name);
                                self.live.tool = Some(name.to_string());
                                changed = true;
                            }
                        }
                        if let Some(a) = val["arguments"].as_str() {
                            let mut s = call["function"]["arguments"]
                                .as_str()
                                .unwrap_or("")
                                .to_string();
                            s.push_str(a);
                            call["function"]["arguments"] = json!(s);
                        }
                    }
                    _ if !val.is_null() => call[k] = val.clone(),
                    _ => {}
                }
            }
        }
        Ok(changed)
    }

    /// The response a non-streamed request would have returned. A stream
    /// that ends with neither a finish reason nor `[DONE]` is a network
    /// error (retried), never taken as a complete turn.
    pub(crate) fn finish(mut self) -> Result<Value, AgentError> {
        if !self.done && self.finish.is_none() {
            return Err(AgentError::Network(
                "the stream ended before the reply was complete".into(),
            ));
        }
        for c in &mut self.calls {
            if c["function"]["arguments"]
                .as_str()
                .is_some_and(|a| a.trim().is_empty())
            {
                c["function"]["arguments"] = json!("{}");
            }
        }
        let mut msg = self.message;
        msg.insert("role".into(), json!("assistant"));
        if !msg.contains_key("content") {
            msg.insert("content".into(), Value::Null);
        }
        if !self.calls.is_empty() {
            msg.insert("tool_calls".into(), Value::Array(self.calls));
        }
        if !self.details.is_empty() {
            msg.insert("reasoning_details".into(), Value::Array(self.details));
        }
        let finish = self.finish.unwrap_or(Value::Null);
        Ok(json!({
            "choices": [{ "message": Value::Object(msg), "finish_reason": finish }],
            "usage": self.usage,
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::OpenAiStream;

    #[test]
    fn chunks_rebuild_the_message() -> Result<(), crate::AgentError> {
        let mut s = OpenAiStream::default();
        for c in [
            r#"{"choices":[{"delta":{"role":"assistant","reasoning_content":"Look at "}}]}"#,
            r#"{"choices":[{"delta":{"reasoning_content":"main."}}]}"#,
            r#"{"choices":[{"delta":{"tool_calls":[{"index":0,"id":"c1","type":"function","function":{"name":"disassemble","arguments":"{\"tar"}}]}}]}"#,
            r#"{"choices":[{"delta":{"tool_calls":[{"index":0,"function":{"arguments":"get\":\"main\"}"}}]}}]}"#,
            r#"{"choices":[{"delta":{},"finish_reason":"tool_calls"}]}"#,
            r#"{"choices":[],"usage":{"prompt_tokens":50,"completion_tokens":9}}"#,
            "[DONE]",
        ] {
            s.feed(c)?;
        }
        assert_eq!(s.live.thinking, "Look at main.");
        let r = s.finish()?;
        let m = &r["choices"][0]["message"];
        assert_eq!(m["reasoning_content"], "Look at main.");
        assert_eq!(
            m["tool_calls"][0]["function"]["arguments"],
            "{\"target\":\"main\"}"
        );
        assert_eq!(m["tool_calls"][0]["id"], "c1");
        assert_eq!(r["choices"][0]["finish_reason"], "tool_calls");
        assert_eq!(r["usage"]["prompt_tokens"], 50);
        Ok(())
    }

    #[test]
    fn a_cut_off_stream_is_not_a_reply() -> Result<(), crate::AgentError> {
        let mut s = OpenAiStream::default();
        s.feed(r#"{"choices":[{"delta":{"content":"partial"}}]}"#)?;
        assert!(s.finish().is_err_and(|e| e.is_transient()));
        Ok(())
    }
}
