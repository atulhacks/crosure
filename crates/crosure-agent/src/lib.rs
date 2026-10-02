//! Crosure AI agent.
//!
//! Claude reverses a binary by calling Crosure's own analysis ops as tools.
//! Each tool call goes through the same recorder as a human click, so it
//! becomes an `actor: agent` step on the investigation graph, hash-chained,
//! with the model's stated reason (`why`) stored as the step's intent.
//!
//! The model client is a trait ([`Llm`]) so the loop runs against Claude
//! ([`ClaudeHttp`]) in the app and against a [`ScriptedLlm`] in tests.

mod agent;
mod error;
mod events;
mod executor;
mod llm;
mod prompt;
mod render;
mod request;
mod scripted;
mod tools;

pub use agent::{run_agent, AgentConfig, Executed, Executor};
pub use error::AgentError;
pub use events::{AgentEvent, Sink};
pub use executor::SessionExecutor;
pub use llm::{ClaudeHttp, Llm, DEFAULT_MODEL};
pub use render::render_result;
pub use request::build_request;
pub use scripted::ScriptedLlm;
pub use tools::{parse_tool_call, tool_definitions, ToolCall};
