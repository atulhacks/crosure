//! Crosure as an MCP server.
//!
//! Exposes the same recorded tools the built-in agent uses (disassemble,
//! xrefs, strings, rename, findings, ...) over the Model Context Protocol
//! (JSON-RPC 2.0, one message per line on stdio). Whatever agent connects,
//! every call it makes becomes an `agent` step on the graph, tagged
//! `mcp:<client name>`, with the client's `why` as the step's intent.

mod tools;

use crosure_agent::{parse_tool_call, render_result, Executor};
use serde_json::{json, Value};

pub use tools::tool_list;

/// The newest MCP revision this server speaks.
pub const PROTOCOL_VERSION: &str = "2025-06-18";

/// Revisions this server speaks, newest first. Their differences (batching,
/// structured output, titles) do not affect a tools-only server: older
/// clients ignore fields they do not know.
pub const SUPPORTED_VERSIONS: &[&str] = &["2025-06-18", "2025-03-26", "2024-11-05"];

/// The revision to answer with: the client's if supported, else the newest.
///
/// ```
/// assert_eq!(crosure_mcp::negotiate(Some("2025-03-26")), "2025-03-26");
/// assert_eq!(crosure_mcp::negotiate(Some("2099-01-01")), crosure_mcp::PROTOCOL_VERSION);
/// ```
pub fn negotiate(requested: Option<&str>) -> &'static str {
    SUPPORTED_VERSIONS
        .iter()
        .find(|v| Some(**v) == requested)
        .copied()
        .unwrap_or(PROTOCOL_VERSION)
}

/// One MCP connection over an executor.
pub struct Server<E: Executor> {
    exec: E,
    client: String,
}

fn ok(id: &Value, result: Value) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "result": result })
}

fn err(id: &Value, code: i64, message: &str) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "error": { "code": code, "message": message } })
}

impl<E: Executor> Server<E> {
    /// A server recording through `exec`.
    pub fn new(exec: E) -> Self {
        Self {
            exec,
            client: "mcp-client".into(),
        }
    }

    /// The tag recorded on every step: `mcp:<client name>`.
    pub fn tag(&self) -> String {
        format!("mcp:{}", self.client)
    }

    /// Handles one JSON-RPC message, or a batch (an array of them); returns
    /// the response (none for notifications and client responses).
    ///
    /// ```
    /// use crosure_agent::{Executed, Executor, ToolCall};
    /// struct Nop;
    /// impl Executor for Nop {
    ///     fn execute(&self, _: &ToolCall, _: &str) -> Result<Executed, String> { Err("x".into()) }
    ///     fn context(&self) -> String { String::new() }
    /// }
    /// let mut s = crosure_mcp::Server::new(Nop);
    /// let r = s.handle(&serde_json::json!({"jsonrpc":"2.0","id":1,"method":"tools/list"}));
    /// assert!(r.is_some_and(|r| r["result"]["tools"].as_array().is_some_and(|t| !t.is_empty())));
    /// ```
    pub fn handle(&mut self, msg: &Value) -> Option<Value> {
        if let Some(batch) = msg.as_array() {
            if batch.is_empty() {
                return Some(err(&Value::Null, -32600, "empty batch"));
            }
            let replies: Vec<Value> = batch.iter().filter_map(|m| self.handle(m)).collect();
            return (!replies.is_empty()).then_some(Value::Array(replies));
        }
        let id = msg.get("id")?.clone();
        let Some(method) = msg["method"].as_str() else {
            // A response to us (we send no requests) is ignored; anything
            // else with an id and no method is not a request.
            return (msg.get("result").is_none() && msg.get("error").is_none())
                .then(|| err(&id, -32600, "invalid request: no method"));
        };
        let params = &msg["params"];
        Some(match method {
            "initialize" => {
                if let Some(name) = params["clientInfo"]["name"].as_str() {
                    self.client = name
                        .chars()
                        .filter(|c| c.is_ascii_alphanumeric() || "-_.".contains(*c))
                        .collect();
                }
                ok(
                    &id,
                    json!({
                        "protocolVersion": negotiate(params["protocolVersion"].as_str()),
                        "capabilities": { "tools": { "listChanged": false } },
                        "serverInfo": { "name": "crosure", "version": env!("CARGO_PKG_VERSION") },
                        "instructions": format!(
                            "{} Every tool call is recorded on the analyst's investigation graph; give a short `why` with each call.",
                            self.exec.context()
                        ),
                    }),
                )
            }
            "ping" => ok(&id, json!({})),
            "tools/list" => ok(&id, json!({ "tools": tool_list() })),
            "tools/call" => {
                let name = params["name"].as_str().unwrap_or("");
                if crosure_session::spec_for_tool(name).is_none() {
                    return Some(err(&id, -32602, &format!("unknown tool: {name}")));
                }
                let args = params.get("arguments").cloned().unwrap_or(json!({}));
                let reply = match parse_tool_call(name, &args) {
                    Err(e) => {
                        json!({ "content": [{ "type": "text", "text": format!("Invalid input: {e}") }], "isError": true })
                    }
                    Ok(call) => match self.exec.execute(&call, &self.tag()) {
                        Ok(done) => json!({
                            "content": [{ "type": "text", "text": render_result(
                                &done.kind,
                                &done.summary,
                                &done.result,
                                call.op.offset(),
                            ) }],
                            // Which step on the graph this call became.
                            "structuredContent": { "step_id": done.step_id, "kind": done.kind, "summary": done.summary },
                            "isError": false,
                        }),
                        Err(e) => {
                            json!({ "content": [{ "type": "text", "text": format!("Error: {e}") }], "isError": true })
                        }
                    },
                };
                ok(&id, reply)
            }
            other => err(&id, -32601, &format!("method not found: {other}")),
        })
    }
}
