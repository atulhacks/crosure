//! Crosure AI agent.
//!
//! A model reverses a binary by calling Crosure's own analysis ops as tools.
//! Each tool call goes through the same recorder as a human click, so it
//! becomes an `actor: agent` step on the investigation graph, hash-chained,
//! with the model's stated reason (`why`) stored as the step's intent.
//!
//! Models plug in as [`Provider`]s: Anthropic natively, and any
//! OpenAI-compatible server (OpenAI, Gemini, OpenRouter, Groq, DeepSeek,
//! Mistral, Ollama, LM Studio, ...). The loop works on a provider-neutral
//! [`Transcript`], so a run can move to the next provider if one declines.

mod agent;
mod error;
mod events;
mod executor;
mod policy;
mod prompt;
mod providers;
mod render;
mod tools;
mod transcript;

pub use agent::{run_agent, run_agent_turn, AgentConfig, Executed, Executor};
pub use error::AgentError;
pub use events::{AgentEvent, Sink};
pub use executor::SessionExecutor;
pub use policy::{ApprovalRequest, Permission, Permissions, Profile, WRITE_TOOLS};
pub use providers::{
    anthropic_compatible, anthropic_request, build_chain, build_provider, list_models,
    openai_request, presets, AgentSettings, AnthropicProvider, Extras, KeySource, OpenAiProvider,
    Provider, ProviderConfig, ProviderKind, ProviderView, ScriptedProvider, DEFAULT_MODEL,
};
pub use render::render_result;
pub use tools::{parse_tool_call, tool_definitions, tool_definitions_for, ToolCall};
pub use transcript::{Block, Entry, Stop, ToolResult, Transcript, Turn};
