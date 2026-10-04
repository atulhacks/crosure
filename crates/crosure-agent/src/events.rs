use serde::{Deserialize, Serialize};

use crate::ApprovalRequest;

/// What the agent is doing, for the UI. Serialized with a `type` tag.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AgentEvent {
    /// The run began.
    Started {
        prompt: String,
        model: String,
        provider: String,
    },
    /// A summary of the model's reasoning for this turn.
    Thinking { text: String },
    /// Text the model wrote between tool calls.
    Message { text: String },
    /// A tool call, with the recorded step it produced (or why it failed).
    ToolCall {
        tool: String,
        command: String,
        why: String,
        step_id: Option<String>,
        summary: Option<String>,
        error: Option<String>,
    },
    /// A provider's safety system declined.
    Refusal {
        provider: String,
        category: Option<String>,
        explanation: Option<String>,
    },
    /// The run moved to the next provider after a decline (`provider:model` tags).
    Switched { from: String, to: String },
    /// The run finished with a final report.
    Finished {
        report: String,
        turns: u32,
        tool_calls: u32,
        input_tokens: u64,
        output_tokens: u64,
    },
    /// The run stopped on an error.
    Failed { error: String },
    /// The user stopped the run.
    Stopped,
    /// Cumulative tokens for the run so far, and how full the context is.
    Usage {
        input_tokens: u64,
        output_tokens: u64,
        /// Prompt tokens served from the provider's cache, cumulative.
        #[serde(default)]
        cached_tokens: u64,
        /// Size of the last request's prompt, as the server counted it.
        #[serde(default)]
        context_tokens: u64,
        /// Prompt size at which old results are elided (see `AgentConfig`).
        #[serde(default)]
        context_limit: u64,
    },
    /// A tool call is waiting for the analyst (tool permission `confirm`).
    ApprovalRequested { request: ApprovalRequest },
    /// The analyst decided.
    ApprovalResolved { id: String, allowed: bool },
    /// A temporary provider failure; retrying after `delay_ms`.
    Retrying {
        attempt: u32,
        delay_ms: u64,
        error: String,
    },
    /// Old tool results were elided to stay within the context window.
    ContextTrimmed { elided: usize, tokens: usize },
    /// The server counted far fewer prompt tokens than were sent: it most
    /// likely dropped the start of the prompt (a local server's context is
    /// smaller than the conversation). Later requests are trimmed to fit.
    PromptTruncated { estimated: usize, reported: usize },
}

/// The reply being streamed, shown live and never saved: the finished turn
/// arrives as ordinary events.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Live {
    /// Reasoning text so far.
    pub thinking: String,
    /// Reply text so far.
    pub text: String,
    /// The tool call being written, if any.
    pub tool: Option<String>,
}

/// Where events go, and whether the user asked to stop.
pub trait Sink: Send + Sync {
    /// Delivers one event.
    fn emit(&self, event: AgentEvent);
    /// True once the user pressed stop.
    fn should_stop(&self) -> bool;
    /// Blocks until the analyst allows or denies a tool call. Sinks with no
    /// one to ask (tests, the CLI) allow.
    fn approve(&self, _request: &ApprovalRequest) -> bool {
        true
    }
    /// The reply streamed so far; an empty one clears it. Sinks that show
    /// nothing live ignore it.
    fn live(&self, _live: &Live) {}
}
