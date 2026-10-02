//! Model providers. Each one turns a [`Transcript`] into its own wire format
//! and parses the reply back into a neutral [`Turn`].

mod anthropic;
mod config;
mod extras;
mod http;
mod openai;
mod presets;
mod scripted;

pub use anthropic::{
    anthropic_request, compatible as anthropic_compatible, AnthropicProvider, DEFAULT_MODEL,
};
pub use config::{
    build_chain, build_provider, list_models, AgentSettings, KeySource, ProviderConfig,
    ProviderKind, ProviderView,
};
pub use extras::Extras;
pub use openai::{openai_request, OpenAiProvider};
pub use presets::presets;
pub use scripted::ScriptedProvider;

use crate::{AgentError, Transcript, Turn};

/// A model the agent can run on.
pub trait Provider: Send + Sync {
    /// Settings id (e.g. `anthropic`, `ollama`). Identifies whose turns are whose.
    fn id(&self) -> &str;
    /// Model id sent with requests.
    fn model(&self) -> &str;
    /// `id:model`, recorded on every step the provider takes.
    fn tag(&self) -> String {
        format!("{}:{}", self.id(), self.model())
    }
    /// Asks for the next turn.
    fn next(&self, transcript: &Transcript) -> Result<Turn, AgentError>;
}
