use serde_json::Value;

use crate::budget::Budget;
use crate::call::{run_one, CallCtx};
use crate::context::{elide_old_results, estimate_tokens, is_context_overflow};
use crate::policy::Permissions;
use crate::prompt::wrap_up;
use crate::providers::Provider;
use crate::retry::next_turn;
use crate::tools::ToolCall;
use crate::{AgentError, AgentEvent, Block, Entry, Sink, Stop, ToolResult, Transcript};

/// Limits and permissions for one run.
#[derive(Clone, Debug)]
pub struct AgentConfig {
    /// Model turns before the agent is told to wrap up.
    pub max_turns: u32,
    /// Allow / confirm / deny per tool.
    pub permissions: Permissions,
    /// Estimated request size (tokens) above which old tool results are
    /// elided, down to two thirds of it (in one go, so prompt caching keeps
    /// working). A bound on cost: a model with a smaller window lowers it
    /// (see [`crate::ModelLimits`]). Providers that reject a long prompt
    /// trigger the same.
    pub context_tokens: usize,
    /// Retries of a temporary provider failure (429, overload, 5xx, network).
    pub max_retries: u32,
    /// First retry delay; doubles each time (the server's `retry-after` wins).
    pub retry_base_ms: u64,
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            max_turns: 40,
            permissions: Permissions::default(),
            context_tokens: 100_000,
            max_retries: 3,
            retry_base_ms: 2_000,
        }
    }
}

/// A tool call that ran and was recorded.
#[derive(Clone, Debug)]
pub struct Executed {
    /// Id of the recorded step.
    pub step_id: String,
    /// Step kind (`disasm`, `xref`, ...), selects how the result is rendered.
    pub kind: String,
    pub command: String,
    pub summary: String,
    pub result: Value,
}

/// Runs a parsed tool call against the open binary and records it.
pub trait Executor: Send + Sync {
    /// Executes and records one call as an agent step: `by` is the
    /// `provider:model` tag, `call.why` becomes the step's intent.
    fn execute(&self, call: &ToolCall, by: &str) -> Result<Executed, String>;
    /// One paragraph describing the open binary, given to the model up front.
    fn context(&self) -> String;
}

/// Sent when a reply was cut off by the output limit.
const CUT_OFF: &str = "Your last reply hit the output limit and was cut off. Continue from where you stopped, more briefly.";

/// Runs a fresh conversation. See [`run_agent_turn`].
pub fn run_agent(
    chain: &[Box<dyn Provider>],
    exec: &dyn Executor,
    sink: &dyn Sink,
    prompt: &str,
    cfg: &AgentConfig,
) -> Result<String, AgentError> {
    run_agent_turn(chain, exec, sink, &mut Transcript::default(), prompt, cfg)
}

/// Adds `prompt` to a thread and runs until the model answers, is stopped,
/// or fails. Follow-ups keep the whole conversation (the thread).
///
/// `chain` is the active provider followed by fallbacks: if a model declines,
/// the same conversation continues on the next one (the switch is reported).
/// Every tool call goes through `exec`, so it is recorded on the graph.
pub fn run_agent_turn(
    chain: &[Box<dyn Provider>],
    exec: &dyn Executor,
    sink: &dyn Sink,
    t: &mut Transcript,
    prompt: &str,
    cfg: &AgentConfig,
) -> Result<String, AgentError> {
    let mut cur = 0usize;
    let Some(first) = chain.first() else {
        return Err(AgentError::NoApiKey);
    };
    sink.emit(AgentEvent::Started {
        prompt: prompt.into(),
        model: first.model().into(),
        provider: first.id().into(),
    });
    if t.entries.is_empty() {
        t.push_user(&format!("{}\n\nTask: {prompt}", exec.context()));
    } else {
        t.push_user(prompt);
    }
    let (mut input, mut output, mut cached, mut tool_calls) = (0u64, 0u64, 0u64, 0u32);
    let mut budget = Budget::new(cfg.context_tokens);
    let mut overflow_retried = false;

    for turn_no in 1..=cfg.max_turns + 3 {
        if sink.should_stop() {
            sink.emit(AgentEvent::Stopped);
            return Ok(String::new());
        }
        let provider = &chain[cur];
        let limit = budget.limit(provider.as_ref());
        if budget.estimate(t) > limit {
            let ratio = budget.estimate(t) as f64 / estimate_tokens(t).max(1) as f64;
            trim(t, sink, (limit as f64 * 2.0 / 3.0 / ratio) as usize);
        }
        let sent = estimate_tokens(t);
        let turn = match next_turn(provider.as_ref(), t, sink, cfg) {
            Ok(Some(r)) => {
                overflow_retried = false;
                r
            }
            Ok(None) => {
                sink.emit(AgentEvent::Stopped);
                return Ok(String::new());
            }
            Err(e) if !overflow_retried && is_context_overflow(&e.to_string()) => {
                // Too long for this model: elide harder and try once more.
                overflow_retried = true;
                if trim(t, sink, estimate_tokens(t) / 2) > 0 {
                    continue;
                }
                sink.emit(AgentEvent::Failed {
                    error: format!("{}: {e}", provider.id()),
                });
                return Err(e);
            }
            Err(e) => {
                sink.emit(AgentEvent::Failed {
                    error: format!("{}: {e}", provider.id()),
                });
                return Err(e);
            }
        };
        budget.observe(sent, turn.input_tokens, sink);
        input += turn.input_tokens;
        output += turn.output_tokens;
        cached += turn.cache_read_tokens;
        sink.emit(AgentEvent::Usage {
            input_tokens: input,
            output_tokens: output,
            cached_tokens: cached,
            context_tokens: turn.input_tokens,
            context_limit: limit as u64,
        });

        if let Stop::Refusal {
            category,
            explanation,
        } = &turn.stop
        {
            sink.emit(AgentEvent::Refusal {
                provider: provider.id().into(),
                category: category.clone(),
                explanation: explanation.clone(),
            });
            if cur + 1 < chain.len() {
                cur += 1;
                sink.emit(AgentEvent::Switched {
                    from: provider.tag(),
                    to: chain[cur].tag(),
                });
                continue;
            }
            return Ok(String::new());
        }
        for b in &turn.blocks {
            if let Block::Thinking(text) = b {
                sink.emit(AgentEvent::Thinking { text: text.clone() });
            }
        }
        let stop = turn.stop.clone();
        let tool_uses: Vec<(String, String, Value)> = turn
            .blocks
            .iter()
            .filter_map(|b| match b {
                Block::ToolUse { id, name, input } => {
                    Some((id.clone(), name.clone(), input.clone()))
                }
                _ => None,
            })
            .collect();
        let text = turn.text();
        t.entries.push(Entry::Assistant {
            provider: provider.id().into(),
            turn,
        });

        match stop {
            Stop::ToolUse => {
                if !text.is_empty() {
                    sink.emit(AgentEvent::Message { text });
                }
                let ctx = CallCtx {
                    exec,
                    sink,
                    by: provider.tag(),
                    profile: t.profile,
                    permissions: &cfg.permissions,
                };
                let results = tool_uses
                    .into_iter()
                    .map(|(id, name, args)| {
                        // Every call needs a result, even the ones Stop skipped.
                        if sink.should_stop() {
                            return ToolResult {
                                id,
                                content: "Not run: the analyst stopped the run.".into(),
                                is_error: true,
                            };
                        }
                        let (content, is_error) = run_one(&ctx, &id, &name, &args);
                        tool_calls += 1;
                        ToolResult {
                            id,
                            content,
                            is_error,
                        }
                    })
                    .collect();
                let note = (turn_no >= cfg.max_turns).then(|| wrap_up(t.profile).to_string());
                t.entries.push(Entry::Results { results, note });
            }
            Stop::PauseTurn => continue,
            Stop::MaxTokens => {
                // The turn is already in the transcript: answer every tool
                // call in it, or the next request is rejected (tool_use
                // without tool_result) and the thread can never continue.
                let results: Vec<ToolResult> = tool_uses
                    .into_iter()
                    .map(|(id, _, _)| ToolResult {
                        id,
                        content: "Not run: your reply hit the output limit before this call \
                                  was complete. Issue it again if you still need it."
                            .into(),
                        is_error: true,
                    })
                    .collect();
                if results.is_empty() {
                    t.push_user(CUT_OFF);
                } else {
                    t.entries.push(Entry::Results {
                        results,
                        note: Some(CUT_OFF.into()),
                    });
                }
            }
            Stop::EndTurn | Stop::Refusal { .. } => {
                sink.emit(AgentEvent::Finished {
                    report: text.clone(),
                    turns: turn_no,
                    tool_calls,
                    input_tokens: input,
                    output_tokens: output,
                });
                return Ok(text);
            }
        }
    }
    let e = AgentError::Protocol("the agent did not finish within its step limit".into());
    sink.emit(AgentEvent::Failed {
        error: e.to_string(),
    });
    Err(e)
}

/// Elides old tool results down to `target` tokens and reports it.
fn trim(t: &mut Transcript, sink: &dyn Sink, target: usize) -> usize {
    let elided = elide_old_results(t, target);
    if elided > 0 {
        sink.emit(AgentEvent::ContextTrimmed {
            elided,
            tokens: estimate_tokens(t),
        });
    }
    elided
}
