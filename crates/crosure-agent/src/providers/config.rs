use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::anthropic::{first_party, AnthropicProvider};
use super::http::Http;
use super::openai::OpenAiProvider;
use super::presets::presets;
use super::Provider;
use crate::AgentError;

/// Wire protocol a provider speaks.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProviderKind {
    Anthropic,
    OpenaiCompatible,
}

/// One configured model provider.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ProviderConfig {
    /// Stable id, recorded on every step this provider takes (`ollama:qwen…`).
    pub id: String,
    pub kind: ProviderKind,
    pub label: String,
    pub base_url: String,
    pub model: String,
    /// Saved key. Never sent to the UI.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub api_key: Option<String>,
    /// Environment variable read when no key is saved.
    #[serde(default)]
    pub key_env: Option<String>,
    /// Send `strict: true` tool schemas (only servers that support it).
    #[serde(default)]
    pub strict_tools: bool,
    #[serde(default = "yes")]
    pub enabled: bool,
    /// Send the output limit as `max_completion_tokens` (always on for
    /// api.openai.com, whose reasoning models reject `max_tokens`).
    #[serde(default)]
    pub max_completion_tokens: bool,
    /// Reasoning effort (`none`, `low`, `medium`, `high`); unset keeps the server default.
    #[serde(default)]
    pub reasoning_effort: Option<String>,
    /// Extra HTTP headers sent with every request.
    #[serde(default)]
    pub headers: BTreeMap<String, String>,
}

fn yes() -> bool {
    true
}

/// Where a provider's key comes from.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum KeySource {
    Saved,
    Env,
    /// Local servers need no key.
    NotNeeded,
    Missing,
}

impl ProviderConfig {
    fn local(&self) -> bool {
        let u = self.base_url.to_lowercase();
        u.contains("://localhost") || u.contains("://127.0.0.1") || u.contains("://[::1]")
    }

    /// The key to use and where it came from.
    pub fn key(&self) -> (Option<String>, KeySource) {
        if let Some(k) = self.api_key.clone().filter(|k| !k.trim().is_empty()) {
            return (Some(k), KeySource::Saved);
        }
        if let Some(k) = std::env::var(self.key_env_name())
            .ok()
            .filter(|k| !k.trim().is_empty())
        {
            return (Some(k), KeySource::Env);
        }
        let src = if self.local() {
            KeySource::NotNeeded
        } else {
            KeySource::Missing
        };
        (None, src)
    }

    /// Enabled, has an endpoint and a model, and has a key unless it is local.
    pub fn ready(&self) -> bool {
        self.enabled
            && !self.base_url.trim().is_empty()
            && !self.model.trim().is_empty()
            && self.key().1 != KeySource::Missing
    }
}

/// All agent settings, saved as `~/.crosure/agent.json` (mode 0600).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AgentSettings {
    /// In fallback order.
    pub providers: Vec<ProviderConfig>,
    /// Provider id runs start on.
    pub active: String,
    /// When a model declines, continue the same run on the next ready provider.
    #[serde(default = "yes")]
    pub auto_fallback: bool,
    /// Analyst instructions appended to the system prompt of every thread.
    #[serde(default)]
    pub instructions: String,
    /// Allow / confirm / deny per tool.
    #[serde(default)]
    pub permissions: crate::Permissions,
    /// Profile new threads start with.
    #[serde(default)]
    pub default_profile: crate::Profile,
}

impl Default for AgentSettings {
    fn default() -> Self {
        Self {
            providers: presets().into_iter().take(1).collect(),
            active: "anthropic".into(),
            auto_fallback: true,
            instructions: String::new(),
            permissions: crate::Permissions::default(),
            default_profile: crate::Profile::default(),
        }
    }
}

impl AgentSettings {
    /// Parses saved settings, migrating the first format (`{api_key, model}`).
    ///
    /// ```
    /// let s = crosure_agent::AgentSettings::from_json(br#"{"api_key":"k","model":"claude-opus-5-5"}"#);
    /// assert_eq!(s.providers[0].api_key.as_deref(), Some("k"));
    /// ```
    pub fn from_json(bytes: &[u8]) -> Self {
        if let Ok(s) = serde_json::from_slice::<AgentSettings>(bytes) {
            return s;
        }
        let mut s = Self::default();
        if let Ok(old) = serde_json::from_slice::<Value>(bytes) {
            if let Some(p) = s.providers.first_mut() {
                p.api_key = old["api_key"].as_str().map(str::to_string);
                if let Some(m) = old["model"].as_str() {
                    p.model = m.into();
                }
            }
        }
        s
    }

    /// Settings as the UI sees them: no keys, plus where each key comes from.
    pub fn views(&self) -> Vec<ProviderView> {
        self.providers
            .iter()
            .map(|p| {
                let mut config = p.clone();
                config.api_key = None;
                config.key_env = Some(p.key_env_name());
                ProviderView {
                    ready: p.ready(),
                    key_source: p.key().1,
                    config,
                }
            })
            .collect()
    }

    /// Replaces the provider list from the UI. A provider sent without a key
    /// keeps its saved one; an empty string clears it.
    pub fn merge(&mut self, incoming: AgentSettings) {
        let old = std::mem::take(&mut self.providers);
        self.providers = incoming
            .providers
            .into_iter()
            .map(|mut p| {
                match &p.api_key {
                    None => {
                        p.api_key = old
                            .iter()
                            .find(|o| o.id == p.id)
                            .and_then(|o| o.api_key.clone())
                    }
                    Some(k) if k.trim().is_empty() => p.api_key = None,
                    Some(_) => {}
                }
                p
            })
            .collect();
        self.active = incoming.active;
        self.auto_fallback = incoming.auto_fallback;
        self.instructions = incoming.instructions;
        self.permissions = incoming.permissions;
        self.default_profile = incoming.default_profile;
    }
}

/// A provider as shown in settings (key redacted).
#[derive(Clone, Debug, Serialize)]
pub struct ProviderView {
    #[serde(flatten)]
    pub config: ProviderConfig,
    pub key_source: KeySource,
    pub ready: bool,
}

/// Builds a runnable provider from its settings.
pub fn build_provider(cfg: &ProviderConfig) -> Result<Box<dyn Provider>, AgentError> {
    let (key, _) = cfg.key();
    Ok(match cfg.kind {
        ProviderKind::Anthropic => Box::new(
            AnthropicProvider::new(
                &cfg.id,
                &cfg.model,
                &cfg.base_url,
                key.as_deref().unwrap_or(""),
            )?
            .with_extras(cfg.extras()),
        ),
        ProviderKind::OpenaiCompatible => Box::new(
            OpenAiProvider::new(
                &cfg.id,
                &cfg.model,
                &cfg.base_url,
                key.as_deref(),
                cfg.strict_tools,
            )?
            .with_extras(cfg.extras()),
        ),
    })
}

/// The active provider first, then (with auto-fallback) every other ready one, in order.
pub fn build_chain(s: &AgentSettings) -> Result<Vec<Box<dyn Provider>>, AgentError> {
    let mut order: Vec<&ProviderConfig> = s.providers.iter().filter(|p| p.id == s.active).collect();
    if s.auto_fallback {
        order.extend(s.providers.iter().filter(|p| p.id != s.active));
    }
    let chain: Vec<Box<dyn Provider>> = order
        .into_iter()
        .filter(|p| p.ready())
        .map(build_provider)
        .collect::<Result<_, _>>()?;
    if chain.is_empty() {
        return Err(AgentError::NoApiKey);
    }
    Ok(chain)
}

/// Lists model ids the provider's server offers (also a connection test).
pub fn list_models(cfg: &ProviderConfig) -> Result<Vec<String>, AgentError> {
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
    let mut ids: Vec<String> = resp["data"]
        .as_array()
        .or_else(|| resp["models"].as_array())
        .map(|a| {
            a.iter()
                .filter_map(|m| m["id"].as_str().or_else(|| m["name"].as_str()))
                // Gemini lists `models/gemini-…`; requests take the bare id.
                .map(|id| id.strip_prefix("models/").unwrap_or(id).to_string())
                .collect()
        })
        .unwrap_or_default();
    ids.sort();
    Ok(ids)
}
