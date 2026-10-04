//! Model providers. Each one turns a [`Transcript`] into its own wire format
//! and parses the reply back into a neutral [`Turn`].

mod anthropic;
mod config;
mod extras;
mod http;
mod limits;
mod openai;
mod presets;
mod scripted;
mod sse;
mod stream_anthropic;
mod stream_openai;

pub use anthropic::{
    anthropic_request, compatible as anthropic_compatible, AnthropicProvider, DEFAULT_MODEL,
};
pub use config::{
    build_chain, build_provider, AgentSettings, KeySource, ProviderConfig, ProviderKind,
    ProviderView,
};
pub use extras::Extras;
pub use limits::{list_model_info, list_models, parse_model_list, ModelInfo, ModelLimits};
pub use openai::{openai_request, OpenAiProvider};
pub use presets::presets;
pub use scripted::ScriptedProvider;

use crate::{AgentError, Sink, Transcript, Turn};

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
    /// The model's context window and output limit, where known.
    fn limits(&self) -> ModelLimits {
        ModelLimits::default()
    }
    /// Asks for the next turn.
    fn next(&self, transcript: &Transcript) -> Result<Turn, AgentError>;
    /// Asks for the next turn, streaming it: the reply so far goes to
    /// [`Sink::live`], and Stop ends the request at once
    /// ([`AgentError::Stopped`]). The finished [`Turn`] is the one
    /// [`Provider::next`] would return. By default, does not stream.
    fn next_live(&self, transcript: &Transcript, _sink: &dyn Sink) -> Result<Turn, AgentError> {
        self.next(transcript)
    }
}
