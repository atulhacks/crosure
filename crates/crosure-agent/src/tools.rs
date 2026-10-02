use crosure_session::Op;
use serde_json::{json, Value};

/// One parsed tool call: the op to run and the model's stated reason.
#[derive(Clone, Debug, PartialEq)]
pub struct ToolCall {
    pub op: Op,
    pub why: String,
}

fn tool(name: &str, description: &str, props: Value, required: &[&str]) -> Value {
    let mut properties = props;
    properties["why"] = json!({ "type": "string", "description": "One sentence: what you expect to learn from this step." });
    let mut req: Vec<&str> = required.to_vec();
    req.push("why");
    json!({
        "name": name,
        "description": description,
        "strict": true,
        "input_schema": {
            "type": "object",
            "properties": properties,
            "required": req,
            "additionalProperties": false,
        }
    })
}

fn nullable_string(desc: &str) -> Value {
    json!({ "type": ["string", "null"], "description": desc })
}

fn string(desc: &str) -> Value {
    json!({ "type": "string", "description": desc })
}

/// The tools Claude can call. Each maps to one recorded Crosure op.
///
/// ```
/// let tools = crosure_agent::tool_definitions();
/// assert!(tools.as_array().is_some_and(|t| t.iter().all(|t| t["input_schema"]["required"]
///     .as_array().is_some_and(|r| r.contains(&serde_json::json!("why"))))));
/// ```
pub fn tool_definitions() -> Value {
    let target = "Function name (e.g. main, strcmp@plt) or address (0x...).";
    json!([
        tool("binary_info", "Format, architecture, entry point, hashes and sections of the binary.", json!({}), &[]),
        tool("list_functions", "List discovered functions with addresses and sizes. Filter by substring to narrow.",
            json!({ "filter": nullable_string("Substring to match, or null for all.") }), &["filter"]),
        tool("disassemble", "Disassemble a whole function. Calls are annotated with callee names, data references with string literals.",
            json!({ "target": string(target) }), &["target"]),
        tool("xrefs_to", "Who references an address: callers of a function or import, users of a string or global.",
            json!({ "target": string("Function, import, or address (string addresses come from search_strings).") }), &["target"]),
        tool("xrefs_from", "Everything a function calls or references.",
            json!({ "function": string(target) }), &["function"]),
        tool("search_strings", "Printable strings (ASCII and UTF-16) with addresses and sections. Filter by case-insensitive substring.",
            json!({ "filter": nullable_string("Substring such as http, .exe, password; null for all.") }), &["filter"]),
        tool("list_imports", "Imported functions grouped by library, with the address code uses to call each.", json!({}), &[]),
        tool("read_bytes", "Hex dump of raw bytes at an address (max 4096).",
            json!({ "address": string("Address (0x...) or symbol."), "length": { "type": ["integer", "null"], "description": "Bytes to read; null for 256." } }),
            &["address", "length"]),
        tool("rename_function", "Give a function a descriptive name once its purpose is clear.",
            json!({ "target": string(target), "new_name": string("snake_case name, no spaces.") }), &["target", "new_name"]),
        tool("add_comment", "Attach a note to an address.",
            json!({ "address": string("Address (0x...)."), "text": string("The comment.") }), &["address", "text"]),
        tool("record_hypothesis", "Pin a hypothesis you are about to test.",
            json!({ "text": string("The hypothesis.") }), &["text"]),
        tool("record_finding", "Record a confirmed finding with its evidence.",
            json!({ "text": string("The finding and the evidence for it.") }), &["text"]),
        tool("record_verdict", "Final classification of the binary. Call once, at the end.",
            json!({
                "verdict": { "type": "string", "enum": ["malicious", "suspicious", "benign", "unknown"] },
                "family": nullable_string("Malware family if known, else null."),
                "summary": string("One or two sentences justifying the verdict."),
            }), &["verdict", "family", "summary"]),
    ])
}

fn s(input: &Value, key: &str) -> Result<String, String> {
    input[key]
        .as_str()
        .map(str::to_string)
        .filter(|v| !v.trim().is_empty())
        .ok_or_else(|| format!("missing or empty `{key}`"))
}

fn opt(input: &Value, key: &str) -> Option<String> {
    input[key]
        .as_str()
        .map(str::to_string)
        .filter(|v| !v.trim().is_empty())
}

/// Validates a `tool_use` block's input and maps it to an op.
///
/// ```
/// use serde_json::json;
/// let call = crosure_agent::parse_tool_call("disassemble", &json!({"target": "main", "why": "see the flow"}))?;
/// assert_eq!(call.why, "see the flow");
/// assert!(crosure_agent::parse_tool_call("disassemble", &json!({"why": "x"})).is_err());
/// # Ok::<(), String>(())
/// ```
pub fn parse_tool_call(name: &str, input: &Value) -> Result<ToolCall, String> {
    if input.get("__unparsed").is_some() {
        return Err("tool arguments were not valid JSON".into());
    }
    let why = opt(input, "why").unwrap_or_default();
    let op = match name {
        "binary_info" => Op::Info,
        "list_functions" => Op::Functions {
            filter: opt(input, "filter"),
        },
        "disassemble" => Op::Disasm {
            target: s(input, "target")?,
        },
        "xrefs_to" => Op::XrefsTo {
            target: s(input, "target")?,
        },
        "xrefs_from" => Op::XrefsFrom {
            target: s(input, "function")?,
        },
        "search_strings" => Op::Strings {
            filter: opt(input, "filter"),
            min_len: None,
        },
        "list_imports" => Op::Imports,
        "read_bytes" => Op::Hex {
            target: s(input, "address")?,
            len: input["length"].as_u64().map(|n| n.min(4096) as usize),
        },
        "rename_function" => {
            let new_name = s(input, "new_name")?;
            if new_name.contains(char::is_whitespace) {
                return Err("`new_name` must not contain spaces".into());
            }
            Op::Rename {
                target: s(input, "target")?,
                name: new_name,
            }
        }
        "add_comment" => Op::Comment {
            target: s(input, "address")?,
            text: s(input, "text")?,
        },
        "record_hypothesis" => Op::Hypothesis {
            text: s(input, "text")?,
        },
        "record_finding" => Op::Finding {
            text: s(input, "text")?,
        },
        "record_verdict" => {
            let verdict = s(input, "verdict")?;
            if !["malicious", "suspicious", "benign", "unknown"].contains(&verdict.as_str()) {
                return Err(format!("unknown verdict `{verdict}`"));
            }
            Op::Verdict {
                verdict,
                family: opt(input, "family"),
                text: s(input, "summary")?,
            }
        }
        other => return Err(format!("unknown tool `{other}`")),
    };
    Ok(ToolCall { op, why })
}
