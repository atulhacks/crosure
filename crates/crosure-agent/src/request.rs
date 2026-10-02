use serde_json::{json, Value};

use crate::prompt::SYSTEM_PROMPT;
use crate::tools::tool_definitions;

/// Builds a Messages API request for one agent turn.
///
/// - adaptive thinking with summarized display, so the UI can show reasoning
/// - effort `high` set explicitly (Claude Opus 5.5 defaults to `medium`)
/// - `fallbacks: "default"`: a safety decline is retried server-side
/// - top-level `cache_control`: the growing prefix is cached turn to turn
///
/// ```
/// use serde_json::json;
/// let body = crosure_agent::build_request("claude-opus-5-5", &[json!({"role": "user", "content": "hi"})]);
/// assert_eq!(body["fallbacks"], "default");
/// assert_eq!(body["tool_choice"]["type"], "auto");
/// ```
pub fn build_request(model: &str, messages: &[Value]) -> Value {
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
