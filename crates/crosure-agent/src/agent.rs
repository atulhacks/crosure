use serde_json::Value;

use crate::policy::{ApprovalRequest, Permission, Permissions};
use crate::providers::Provider;
use crate::render::render_result;
use crate::tools::{parse_tool_call, ToolCall};
use crate::{AgentError, AgentEvent, Block, Entry, Profile, Sink, Stop, ToolResult, Transcript};

/// Limits and permissions for one run.
#[derive(Clone, Debug)]
pub struct AgentConfig {
    /// Model turns before the agent is told to wrap up.
    pub max_turns: u32,
    /// Allow / confirm / deny per tool.
    pub permissions: Permissions,
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            max_turns: 40,
            permissions: Permissions::default(),
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

const WRAP_UP: &str = "Step limit reached. Call record_verdict if you have not, then write the final report without further analysis.";

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
    let (mut input, mut output, mut tool_calls) = (0u64, 0u64, 0u32);

    for turn_no in 1..=cfg.max_turns + 3 {
        if sink.should_stop() {
            sink.emit(AgentEvent::Stopped);
            return Ok(String::new());
        }
        let provider = &chain[cur];
        let turn = match provider.next(t) {
            Ok(r) => r,
            Err(e) => {
                sink.emit(AgentEvent::Failed {
                    error: format!("{}: {e}", provider.id()),
                });
                return Err(e);
            }
        };
        input += turn.input_tokens;
        output += turn.output_tokens;
        sink.emit(AgentEvent::Usage {
            input_tokens: input,
            output_tokens: output,
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
                        let (content, is_error) = run_one(&ctx, &id, &name, &args);
                        tool_calls += 1;
                        ToolResult {
                            id,
                            content,
                            is_error,
                        }
                    })
                    .collect();
                let note = (turn_no == cfg.max_turns).then(|| WRAP_UP.to_string());
                t.entries.push(Entry::Results { results, note });
            }
            Stop::PauseTurn => continue,
            Stop::MaxTokens => {
                let e = AgentError::Protocol("the response hit the output limit".into());
                sink.emit(AgentEvent::Failed {
                    error: e.to_string(),
                });
                return Err(e);
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

struct CallCtx<'a> {
    exec: &'a dyn Executor,
    sink: &'a dyn Sink,
    by: String,
    profile: Profile,
    permissions: &'a Permissions,
}

fn failed(
    ctx: &CallCtx<'_>,
    name: &str,
    command: String,
    why: String,
    error: String,
) -> (String, bool) {
    ctx.sink.emit(AgentEvent::ToolCall {
        tool: name.into(),
        command,
        why,
        step_id: None,
        summary: None,
        error: Some(error.clone()),
    });
    (format!("Error: {error}"), true)
}

fn run_one(ctx: &CallCtx<'_>, id: &str, name: &str, input: &Value) -> (String, bool) {
    if !ctx.profile.allows(name) {
        return failed(
            ctx,
            name,
            name.into(),
            String::new(),
            format!("`{name}` is not available in this profile"),
        );
    }
    let call = match parse_tool_call(name, input) {
        Ok(c) => c,
        Err(e) => {
            return failed(
                ctx,
                name,
                name.into(),
                String::new(),
                format!("invalid input: {e}"),
            )
        }
    };
    let command = call.op.command();
    match ctx.permissions.get(name) {
        Permission::Allow => {}
        Permission::Deny => {
            return failed(
                ctx,
                name,
                command,
                call.why,
                "blocked by tool permissions".into(),
            )
        }
        Permission::Confirm => {
            let request = ApprovalRequest {
                id: id.into(),
                tool: name.into(),
                command: command.clone(),
                why: call.why.clone(),
            };
            ctx.sink.emit(AgentEvent::ApprovalRequested {
                request: request.clone(),
            });
            let allowed = ctx.sink.approve(&request);
            ctx.sink.emit(AgentEvent::ApprovalResolved {
                id: id.into(),
                allowed,
            });
            if !allowed {
                return failed(
                    ctx,
                    name,
                    command,
                    call.why,
                    "the analyst declined this action".into(),
                );
            }
        }
    }
    match ctx.exec.execute(&call, &ctx.by) {
        Ok(done) => {
            ctx.sink.emit(AgentEvent::ToolCall {
                tool: name.into(),
                command: done.command.clone(),
                why: call.why.clone(),
                step_id: Some(done.step_id.clone()),
                summary: Some(done.summary.clone()),
                error: None,
            });
            (
                render_result(&done.kind, &done.summary, &done.result),
                false,
            )
        }
        Err(e) => failed(ctx, name, command, call.why, e),
    }
}
