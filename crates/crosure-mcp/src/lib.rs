//! Crosure as an MCP server.
//!
//! Exposes the same recorded tools the built-in agent uses (disassemble,
//! xrefs, strings, rename, findings, ...) over the Model Context Protocol
//! (JSON-RPC 2.0, one message per line on stdio). Whatever agent connects,
//! every call it makes becomes an `agent` step on the graph, tagged
//! `mcp:<client name>`, with the client's `why` as the step's intent.

use crosure_agent::{parse_tool_call, render_result, tool_definitions, Executor};
use serde_json::{json, Value};

/// MCP protocol revision this server speaks when the client does not ask for one.
pub const PROTOCOL_VERSION: &str = "2025-06-18";

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

    /// Handles one JSON-RPC message; returns the response (none for notifications).
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
        let id = msg.get("id")?.clone();
        let params = &msg["params"];
        Some(match msg["method"].as_str().unwrap_or("") {
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
                        "protocolVersion": params["protocolVersion"].as_str().unwrap_or(PROTOCOL_VERSION),
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
            "tools/list" => {
                let tools: Vec<Value> = tool_definitions()
                    .as_array()
                    .into_iter()
                    .flatten()
                    .map(|t| json!({ "name": t["name"], "description": t["description"], "inputSchema": t["input_schema"] }))
                    .collect();
                ok(&id, json!({ "tools": tools }))
            }
            "tools/call" => {
                let name = params["name"].as_str().unwrap_or("");
                let args = params.get("arguments").cloned().unwrap_or(json!({}));
                let (text, is_error) = match parse_tool_call(name, &args) {
                    Err(e) => (format!("Invalid input: {e}"), true),
                    Ok(call) => match self.exec.execute(&call, &self.tag()) {
                        Ok(done) => (
                            render_result(
                                &done.kind,
                                &done.summary,
                                &done.result,
                                call.op.offset(),
                            ),
                            false,
                        ),
                        Err(e) => (format!("Error: {e}"), true),
                    },
                };
                ok(
                    &id,
                    json!({ "content": [{ "type": "text", "text": text }], "isError": is_error }),
                )
            }
            other => err(&id, -32601, &format!("method not found: {other}")),
        })
    }
}
