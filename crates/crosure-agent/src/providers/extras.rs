use serde_json::{json, Value};

use super::config::ProviderConfig;

/// Headers Crosure sets itself; custom headers with these names are ignored.
const MANAGED: [&str; 5] = [
    "authorization",
    "x-api-key",
    "anthropic-version",
    "content-type",
    "accept",
];

/// Per-provider request options on top of the wire protocol.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Extras {
    /// Extra HTTP headers (gateways, proxies, observability).
    pub headers: Vec<(String, String)>,
    /// `none`, `low`, `medium`, `high` … sent as the server expects it.
    pub reasoning_effort: Option<String>,
    /// Send the output limit as `max_completion_tokens` (OpenAI reasoning models).
    pub max_completion_tokens: bool,
}

/// Thinking budget an Anthropic-compatible server gets for an effort level
/// (kept under the request's `max_tokens` of 16000).
fn budget(effort: &str) -> Option<u64> {
    match effort {
        "minimal" | "low" => Some(2048),
        "medium" => Some(6144),
        "high" => Some(12288),
        "xhigh" | "max" => Some(15000),
        _ => None,
    }
}

impl Extras {
    /// `base` headers followed by the custom ones that do not override them.
    pub(crate) fn with_headers<'a>(
        &'a self,
        base: Vec<(&'static str, String)>,
    ) -> Vec<(&'a str, String)> {
        let mut out: Vec<(&str, String)> = base;
        out.extend(
            self.headers
                .iter()
                .filter(|(k, _)| {
                    let k = k.trim().to_lowercase();
                    !k.is_empty() && !MANAGED.contains(&k.as_str())
                })
                .map(|(k, v)| (k.trim(), v.clone())),
        );
        out
    }

    /// Applies the options to a Chat Completions body.
    ///
    /// ```
    /// use crosure_agent::Extras;
    /// let x = Extras { max_completion_tokens: true, reasoning_effort: Some("high".into()), ..Default::default() };
    /// let mut body = serde_json::json!({ "model": "gpt-5", "max_tokens": 8192 });
    /// x.apply_openai(&mut body);
    /// assert_eq!(body["max_completion_tokens"], 8192);
    /// assert!(body.get("max_tokens").is_none());
    /// assert_eq!(body["reasoning_effort"], "high");
    /// ```
    pub fn apply_openai(&self, body: &mut Value) {
        let Some(o) = body.as_object_mut() else {
            return;
        };
        if self.max_completion_tokens {
            if let Some(n) = o.remove("max_tokens") {
                o.insert("max_completion_tokens".into(), n);
            }
        }
        if let Some(e) = self.effort() {
            o.insert("reasoning_effort".into(), json!(e));
        }
    }

    /// Applies the options to a Messages API body. On Anthropic's own API the
    /// effort replaces the default (`none` turns thinking off); on a compatible
    /// server it turns on budgeted thinking.
    ///
    /// ```
    /// use crosure_agent::Extras;
    /// let x = Extras { reasoning_effort: Some("medium".into()), ..Default::default() };
    /// let mut body = serde_json::json!({ "model": "kimi-k2", "max_tokens": 16000 });
    /// x.apply_anthropic(&mut body, false);
    /// assert_eq!(body["thinking"]["type"], "enabled");
    /// assert_eq!(body["thinking"]["budget_tokens"], 6144);
    /// ```
    pub fn apply_anthropic(&self, body: &mut Value, claude_api: bool) {
        let Some(e) = self.effort() else {
            return;
        };
        let Some(o) = body.as_object_mut() else {
            return;
        };
        if claude_api {
            if e == "none" {
                o.remove("thinking");
                o.remove("output_config");
            } else {
                o.insert("output_config".into(), json!({ "effort": e }));
            }
        } else if let Some(b) = budget(e) {
            o.insert(
                "thinking".into(),
                json!({ "type": "enabled", "budget_tokens": b }),
            );
        }
    }

    fn effort(&self) -> Option<&str> {
        self.reasoning_effort
            .as_deref()
            .map(str::trim)
            .filter(|e| !e.is_empty())
    }
}

impl ProviderConfig {
    /// Environment variable for the key: the configured one, else derived
    /// from the id (`my-gateway` → `MY_GATEWAY_API_KEY`).
    ///
    /// ```
    /// let mut p = crosure_agent::presets().into_iter().find(|p| p.id == "custom").unwrap_or_else(|| unreachable!());
    /// p.id = "my-gateway".into();
    /// assert_eq!(p.key_env_name(), "MY_GATEWAY_API_KEY");
    /// ```
    pub fn key_env_name(&self) -> String {
        if let Some(v) = self.key_env.as_ref().filter(|v| !v.trim().is_empty()) {
            return v.trim().to_string();
        }
        let id: String = self
            .id
            .chars()
            .map(|c| {
                if c.is_ascii_alphanumeric() {
                    c.to_ascii_uppercase()
                } else {
                    '_'
                }
            })
            .collect();
        format!("{id}_API_KEY")
    }

    /// Request options: custom headers, reasoning effort, output-limit field.
    pub fn extras(&self) -> Extras {
        Extras {
            headers: self
                .headers
                .iter()
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect(),
            reasoning_effort: self.reasoning_effort.clone(),
            max_completion_tokens: self.max_completion_tokens
                || self.base_url.to_lowercase().contains("api.openai.com"),
        }
    }
}
