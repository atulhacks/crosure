//! What a model can take and give: its context window and output limit.
//!
//! Servers that report limits in their model list fill them in when models
//! are fetched (Anthropic `max_input_tokens`/`max_tokens`, OpenRouter
//! `context_length`/`top_provider.max_completion_tokens`, llama.cpp
//! `meta.n_ctx`); the analyst can override them per provider. There is no
//! built-in table of vendor models: it would go stale.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::anthropic::{first_party, AnthropicProvider};
use super::config::{ProviderConfig, ProviderKind};
use super::http::Http;
use super::openai::OpenAiProvider;
use crate::AgentError;

/// Token limits of one model; `None` where unknown.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModelLimits {
    /// Prompt plus output, in tokens.
    pub context_window: Option<u64>,
    /// Longest reply, in tokens; sent as the request's `max_tokens`.
    pub max_output: Option<u64>,
}

/// One entry of a server's model list.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModelInfo {
    pub id: String,
    #[serde(flatten)]
    pub limits: ModelLimits,
}

fn first_u64(m: &Value, paths: &[&[&str]]) -> Option<u64> {
    paths.iter().find_map(|path| {
        path.iter()
            .try_fold(m, |v, k| v.get(*k))
            .and_then(Value::as_u64)
            .filter(|n| *n > 0)
    })
}

/// Reads a `/models` response (`data` or `models` array) into ids and
/// whatever limits the server reports, sorted by id.
///
/// ```
/// let resp = serde_json::json!({ "data": [
///     { "id": "claude-x", "max_input_tokens": 200000, "max_tokens": 64000 },
///     { "id": "models/gemini-2.5-pro" },
///     { "id": "qwen", "context_length": 32768, "top_provider": { "max_completion_tokens": 8192 } }
/// ] });
/// let models = crosure_agent::parse_model_list(&resp);
/// assert_eq!(models[0].id, "claude-x");
/// assert_eq!(models[0].limits.context_window, Some(200000));
/// assert_eq!(models[1].id, "gemini-2.5-pro");
/// assert_eq!(models[1].limits.max_output, None);
/// assert_eq!(models[2].limits.max_output, Some(8192));
/// ```
pub fn parse_model_list(resp: &Value) -> Vec<ModelInfo> {
    let mut out: Vec<ModelInfo> = resp["data"]
        .as_array()
        .or_else(|| resp["models"].as_array())
        .map(|a| {
            a.iter()
                .filter_map(|m| {
                    let id = m["id"].as_str().or_else(|| m["name"].as_str())?;
                    // Gemini lists `models/gemini-…`; requests take the bare id.
                    let id = id.strip_prefix("models/").unwrap_or(id).to_string();
                    let limits = ModelLimits {
                        context_window: first_u64(
                            m,
                            &[
                                &["max_input_tokens"],
                                &["context_window"],
                                &["context_length"],
                                &["inputTokenLimit"],
                                &["meta", "n_ctx"],
                            ],
                        ),
                        max_output: first_u64(
                            m,
                            &[
                                &["max_tokens"],
                                &["max_output_tokens"],
                                &["outputTokenLimit"],
                                &["top_provider", "max_completion_tokens"],
                            ],
                        ),
                    };
                    Some(ModelInfo { id, limits })
                })
                .collect()
        })
        .unwrap_or_default();
    out.sort_by(|a, b| a.id.cmp(&b.id));
    out
}

/// Lists the models the provider's server offers, with any limits it reports
/// (also a connection test).
pub fn list_model_info(cfg: &ProviderConfig) -> Result<Vec<ModelInfo>, AgentError> {
    let http = Http::new()?;
    let (key, _) = cfg.key();
    let base = cfg.base_url.trim_end_matches('/');
    let extras = cfg.extras();
    let (url, headers) = match cfg.kind {
        ProviderKind::Anthropic => (
            format!("{base}/v1/models?limit=100"),
            AnthropicProvider::headers(key.as_deref().unwrap_or(""), first_party(&cfg.base_url)),
        ),
        ProviderKind::OpenaiCompatible => (
            format!("{base}/models"),
            OpenAiProvider::headers(key.as_deref()),
        ),
    };
    let resp = http.call(&url, &extras.with_headers(headers), None)?;
    Ok(parse_model_list(&resp))
}

/// Model ids the provider's server offers (also a connection test).
pub fn list_models(cfg: &ProviderConfig) -> Result<Vec<String>, AgentError> {
    Ok(list_model_info(cfg)?.into_iter().map(|m| m.id).collect())
}
