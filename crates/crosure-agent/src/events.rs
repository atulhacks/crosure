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
    /// Cumulative tokens for the run so far.
    Usage {
        input_tokens: u64,
        output_tokens: u64,
    },
    /// A tool call is waiting for the analyst (tool permission `confirm`).
    ApprovalRequested { request: ApprovalRequest },
    /// The analyst decided.
    ApprovalResolved { id: String, allowed: bool },
    /// Old tool results were elided to stay within the context window.
    ContextTrimmed { elided: usize, tokens: usize },
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
}
