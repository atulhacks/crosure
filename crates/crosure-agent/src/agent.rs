use serde_json::{json, Value};

use crate::render::render_result;
use crate::request::build_request;
use crate::tools::{parse_tool_call, ToolCall};
use crate::{AgentError, AgentEvent, Llm, Sink};

/// Limits for one run.
#[derive(Clone, Debug)]
pub struct AgentConfig {
    /// Model turns before the agent is told to wrap up.
    pub max_turns: u32,
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self { max_turns: 40 }
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
    /// Executes and records one call (as an agent step with `why` as its intent).
    fn execute(&self, call: &ToolCall) -> Result<Executed, String>;
    /// One paragraph describing the open binary, given to the model up front.
    fn context(&self) -> String;
}

#[derive(Default)]
struct Usage {
    input: u64,
    output: u64,
}

impl Usage {
    fn add(&mut self, u: &Value) {
        for k in [
            "input_tokens",
            "cache_read_input_tokens",
            "cache_creation_input_tokens",
        ] {
            self.input += u[k].as_u64().unwrap_or(0);
        }
        self.output += u["output_tokens"].as_u64().unwrap_or(0);
    }
}

fn texts(content: &[Value], kind: &str, field: &str) -> Vec<String> {
    content
        .iter()
        .filter(|b| b["type"] == kind)
        .filter_map(|b| b[field].as_str())
        .map(str::trim)
        .filter(|t| !t.is_empty())
        .map(str::to_string)
        .collect()
}

/// Runs the agent until it writes a final report, is stopped, or fails.
/// Returns the report. Every tool call is executed through `exec`, so it is
/// recorded on the investigation graph.
pub fn run_agent(
    llm: &dyn Llm,
    exec: &dyn Executor,
    sink: &dyn Sink,
    prompt: &str,
    cfg: &AgentConfig,
) -> Result<String, AgentError> {
    sink.emit(AgentEvent::Started {
        prompt: prompt.into(),
        model: llm.model().into(),
    });
    let opening = format!("{}\n\nTask: {prompt}", exec.context());
    let mut messages: Vec<Value> = vec![json!({ "role": "user", "content": opening })];
    let mut usage = Usage::default();
    let mut tool_calls = 0u32;
    let hard_limit = cfg.max_turns + 3;

    for turn in 1..=hard_limit {
        if sink.should_stop() {
            sink.emit(AgentEvent::Stopped);
            return Ok(String::new());
        }
        let resp = match llm.create(&build_request(llm.model(), &messages)) {
            Ok(r) => r,
            Err(e) => {
                sink.emit(AgentEvent::Failed {
                    error: e.to_string(),
                });
                return Err(e);
            }
        };
        usage.add(&resp["usage"]);
        let stop = resp["stop_reason"].as_str().unwrap_or("");
        if stop == "refusal" {
            sink.emit(AgentEvent::Refusal {
                category: resp["stop_details"]["category"]
                    .as_str()
                    .map(str::to_string),
                explanation: resp["stop_details"]["explanation"]
                    .as_str()
                    .map(str::to_string),
            });
            return Ok(String::new());
        }
        let content: Vec<Value> = resp["content"].as_array().cloned().unwrap_or_default();
        for t in texts(&content, "thinking", "thinking") {
            sink.emit(AgentEvent::Thinking { text: t });
        }
        // The assistant turn goes back verbatim (thinking blocks included).
        messages.push(json!({ "role": "assistant", "content": content.clone() }));

        match stop {
            "tool_use" => {
                for t in texts(&content, "text", "text") {
                    sink.emit(AgentEvent::Message { text: t });
                }
                let mut results = Vec::new();
                for block in content.iter().filter(|b| b["type"] == "tool_use") {
                    let name = block["name"].as_str().unwrap_or("");
                    let id = block["id"].clone();
                    let (text, is_error) = run_one(exec, sink, name, &block["input"]);
                    tool_calls += 1;
                    results.push(json!({ "type": "tool_result", "tool_use_id": id, "content": text, "is_error": is_error }));
                }
                if turn == cfg.max_turns {
                    results.push(json!({ "type": "text", "text": "Step limit reached. Call record_verdict if you have not, then write the final report without further analysis." }));
                }
                messages.push(json!({ "role": "user", "content": results }));
            }
            "pause_turn" => continue,
            "max_tokens" => {
                let e = AgentError::Protocol("the response hit max_tokens".into());
                sink.emit(AgentEvent::Failed {
                    error: e.to_string(),
                });
                return Err(e);
            }
            _ => {
                let report = texts(&content, "text", "text").join("\n\n");
                sink.emit(AgentEvent::Finished {
                    report: report.clone(),
                    turns: turn,
                    tool_calls,
                    input_tokens: usage.input,
                    output_tokens: usage.output,
                });
                return Ok(report);
            }
        }
    }
    let e = AgentError::Protocol("the agent did not finish within its step limit".into());
    sink.emit(AgentEvent::Failed {
        error: e.to_string(),
    });
    Err(e)
}

fn run_one(exec: &dyn Executor, sink: &dyn Sink, name: &str, input: &Value) -> (String, bool) {
    let call = match parse_tool_call(name, input) {
        Ok(c) => c,
        Err(e) => {
            sink.emit(AgentEvent::ToolCall {
                tool: name.into(),
                command: name.into(),
                why: String::new(),
                step_id: None,
                summary: None,
                error: Some(e.clone()),
            });
            return (format!("Invalid input: {e}"), true);
        }
    };
    match exec.execute(&call) {
        Ok(done) => {
            sink.emit(AgentEvent::ToolCall {
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
        Err(e) => {
            sink.emit(AgentEvent::ToolCall {
                tool: name.into(),
                command: call.op.command(),
                why: call.why,
                step_id: None,
                summary: None,
                error: Some(e.clone()),
            });
            (format!("Error: {e}"), true)
        }
    }
}
