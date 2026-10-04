//! The tool list as MCP describes it: schema, a display title, and
//! behaviour hints derived from the op registry.

use crosure_agent::tool_definitions;
use serde_json::{json, Value};

/// `xrefs_to` -> `Xrefs to`.
fn title(name: &str) -> String {
    let mut words = name.split('_');
    let first: String = words
        .next()
        .map(|w| {
            let mut c = w.chars();
            c.next()
                .map(|f| f.to_uppercase().chain(c).collect())
                .unwrap_or_default()
        })
        .unwrap_or_default();
    std::iter::once(first)
        .chain(words.map(str::to_string))
        .collect::<Vec<_>>()
        .join(" ")
}

/// Every tool, for `tools/list`.
///
/// Hints, from the registry: analysis tools only read (`readOnlyHint`);
/// renames and comments change how the binary reads but keep the old value
/// on the graph (not destructive); records (`record_*`) add to it. No tool
/// reaches outside the session (`openWorldHint: false`).
///
/// ```
/// let tools = crosure_mcp::tool_list();
/// let dis = tools.iter().find(|t| t["name"] == "disassemble").unwrap_or_else(|| unreachable!());
/// assert_eq!(dis["title"], "Disassemble");
/// assert_eq!(dis["annotations"]["readOnlyHint"], true);
/// let ren = tools.iter().find(|t| t["name"] == "rename_function").unwrap_or_else(|| unreachable!());
/// assert_eq!(ren["annotations"]["readOnlyHint"], false);
/// assert_eq!(ren["annotations"]["destructiveHint"], false);
/// ```
pub fn tool_list() -> Vec<Value> {
    tool_definitions()
        .as_array()
        .into_iter()
        .flatten()
        .map(|t| {
            let name = t["name"].as_str().unwrap_or("");
            let reads = !crosure_session::is_write_tool(name) && !name.starts_with("record_");
            json!({
                "name": name,
                "title": title(name),
                "description": t["description"],
                "inputSchema": t["input_schema"],
                "annotations": {
                    "title": title(name),
                    "readOnlyHint": reads,
                    "destructiveHint": false,
                    "idempotentHint": reads,
                    "openWorldHint": false,
                },
            })
        })
        .collect()
}
