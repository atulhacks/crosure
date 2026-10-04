//! First-run setup: which providers already have a key in the environment.

use crosure_agent::{presets, AgentSettings, KeySource, ProviderConfig};
use serde::Serialize;

/// A provider whose key was found in an environment variable.
#[derive(Serialize)]
pub struct Detected {
    pub id: String,
    pub label: String,
    /// Empty when the analyst still has to pick one.
    pub model: String,
    pub key_env: String,
}

/// Configured providers, then presets, whose key comes from the environment.
pub fn detected(s: &AgentSettings) -> Vec<Detected> {
    let extra = presets()
        .into_iter()
        .filter(|p| !s.providers.iter().any(|c| c.id == p.id));
    s.providers
        .iter()
        .cloned()
        .chain(extra)
        .filter(|p| p.enabled && p.key().1 == KeySource::Env)
        .map(|p| Detected {
            key_env: p.key_env_name(),
            id: p.id,
            label: p.label,
            model: p.model,
        })
        .collect()
}

/// Adds the preset `id` to the provider list if it is not there yet.
pub fn adopt(s: &mut AgentSettings, id: &str) -> bool {
    if s.providers.iter().any(|p| p.id == id) {
        return true;
    }
    let preset: Option<ProviderConfig> = presets().into_iter().find(|p| p.id == id);
    preset.map(|p| s.providers.push(p)).is_some()
}
