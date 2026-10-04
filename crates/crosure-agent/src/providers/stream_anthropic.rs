//! Rebuilds a Messages API response from its stream events, so the reply is
//! parsed, stored and echoed back exactly as a non-streamed one would be
//! (thinking signatures included).

use serde_json::{json, Value};

use crate::{AgentError, Live};

/// Accumulates one streamed message.
#[derive(Default)]
pub(crate) struct AnthropicStream {
    message: Value,
    blocks: Vec<Value>,
    /// Tool input JSON as it arrives, per block.
    partial: Vec<String>,
    done: bool,
    pub(crate) live: Live,
}

fn append(slot: &mut Value, text: &str) {
    let mut s = slot.as_str().unwrap_or("").to_string();
    s.push_str(text);
    *slot = Value::String(s);
}

fn stream_error(e: &Value) -> AgentError {
    let message = e["message"].as_str().unwrap_or("stream error").to_string();
    let status = match e["type"].as_str() {
        Some("overloaded_error") => 529,
        Some("rate_limit_error") => 429,
        Some("api_error") => 500,
        _ => {
            return AgentError::Api {
                status: 400,
                message,
            }
        }
    };
    AgentError::Transient {
        status,
        message,
        retry_after: None,
    }
}

impl AnthropicStream {
    /// Feeds one event's `data:` payload. Returns whether the live view changed.
    pub(crate) fn feed(&mut self, data: &str) -> Result<bool, AgentError> {
        let Ok(v) = serde_json::from_str::<Value>(data) else {
            return Ok(false);
        };
        let i = v["index"].as_u64().and_then(|i| usize::try_from(i).ok());
        match v["type"].as_str().unwrap_or("") {
            "message_start" => self.message = v["message"].clone(),
            "content_block_start" => {
                let Some(i) = i else { return Ok(false) };
                if self.blocks.len() <= i {
                    self.blocks.resize(i + 1, Value::Null);
                    self.partial.resize(i + 1, String::new());
                }
                let block = v["content_block"].clone();
                if block["type"] == "tool_use" {
                    self.live.tool = block["name"].as_str().map(str::to_string);
                }
                self.blocks[i] = block;
                return Ok(true);
            }
            "content_block_delta" => {
                let (Some(i), d) = (i, &v["delta"]) else {
                    return Ok(false);
                };
                let Some(block) = self.blocks.get_mut(i) else {
                    return Ok(false);
                };
                let text = |k: &str| d[k].as_str().unwrap_or("");
                match d["type"].as_str().unwrap_or("") {
                    "text_delta" => {
                        append(&mut block["text"], text("text"));
                        self.live.text.push_str(text("text"));
                    }
                    "thinking_delta" => {
                        append(&mut block["thinking"], text("thinking"));
                        self.live.thinking.push_str(text("thinking"));
                    }
                    "signature_delta" => append(&mut block["signature"], text("signature")),
                    "input_json_delta" => {
                        if let Some(p) = self.partial.get_mut(i) {
                            p.push_str(text("partial_json"));
                        }
                        return Ok(false);
                    }
                    _ => return Ok(false),
                }
                return Ok(true);
            }
            "content_block_stop" => {
                let Some(i) = i else { return Ok(false) };
                if let (Some(block), Some(p)) = (self.blocks.get_mut(i), self.partial.get(i)) {
                    if block["type"] == "tool_use" {
                        let p = if p.trim().is_empty() { "{}" } else { p };
                        block["input"] =
                            serde_json::from_str(p).unwrap_or_else(|_| json!({ "__unparsed": p }));
                    }
                }
            }
            "message_delta" => {
                for k in ["stop_reason", "stop_sequence", "stop_details"] {
                    if !v["delta"][k].is_null() {
                        self.message[k] = v["delta"][k].clone();
                    }
                }
                if let Some(u) = v["usage"].as_object() {
                    for (k, n) in u.iter().filter(|(_, n)| !n.is_null()) {
                        self.message["usage"][k] = n.clone();
                    }
                }
            }
            "message_stop" => self.done = true,
            "error" => return Err(stream_error(&v["error"])),
            _ => {}
        }
        Ok(false)
    }

    /// The response a non-streamed request would have returned. A stream
    /// cut off before `message_stop` is a network error (retried), never
    /// taken as a complete turn.
    pub(crate) fn finish(mut self) -> Result<Value, AgentError> {
        if !self.done {
            return Err(AgentError::Network(
                "the stream ended before the message was complete".into(),
            ));
        }
        self.message["content"] =
            Value::Array(self.blocks.into_iter().filter(|b| !b.is_null()).collect());
        Ok(self.message)
    }
}

#[cfg(test)]
mod tests {
    use super::AnthropicStream;

    #[test]
    fn deltas_rebuild_the_message() -> Result<(), crate::AgentError> {
        let mut s = AnthropicStream::default();
        for e in [
            r#"{"type":"message_start","message":{"role":"assistant","content":[],"usage":{"input_tokens":10,"output_tokens":1}}}"#,
            r#"{"type":"content_block_start","index":0,"content_block":{"type":"tool_use","id":"t1","name":"disassemble","input":{}}}"#,
            r#"{"type":"content_block_delta","index":0,"delta":{"type":"input_json_delta","partial_json":"{\"target\": \"ma"}}"#,
            r#"{"type":"content_block_delta","index":0,"delta":{"type":"input_json_delta","partial_json":"in\"}"}}"#,
            r#"{"type":"content_block_stop","index":0}"#,
            r#"{"type":"message_delta","delta":{"stop_reason":"tool_use"},"usage":{"output_tokens":30}}"#,
            r#"{"type":"message_stop"}"#,
        ] {
            s.feed(e)?;
        }
        assert_eq!(s.live.tool.as_deref(), Some("disassemble"));
        let m = s.finish()?;
        assert_eq!(m["content"][0]["input"]["target"], "main");
        assert_eq!(m["stop_reason"], "tool_use");
        assert_eq!(m["usage"]["input_tokens"], 10);
        assert_eq!(m["usage"]["output_tokens"], 30);
        Ok(())
    }

    #[test]
    fn an_overload_mid_stream_is_temporary() {
        let mut s = AnthropicStream::default();
        let e = s
            .feed(r#"{"type":"error","error":{"type":"overloaded_error","message":"Overloaded"}}"#);
        assert!(e.is_err_and(|e| e.is_transient()));
    }
}
