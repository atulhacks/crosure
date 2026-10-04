use crosure_session::{check_arg, spec_for_tool, ArgKind, ArgSpec, Op, OPS};
use serde_json::{json, Value};

/// One parsed tool call: the op to run and the model's stated reason.
#[derive(Clone, Debug, PartialEq)]
pub struct ToolCall {
    pub op: Op,
    pub why: String,
}

const WHY: &str = "One sentence: what you expect to learn from this step.";

/// JSON schema of one argument as the agent sees it.
fn arg_schema(a: &ArgSpec) -> Value {
    let base = match a.kind {
        ArgKind::Count { .. } => "integer",
        _ => "string",
    };
    let mut v = if a.required {
        json!({ "type": base })
    } else {
        json!({ "type": [base, "null"] })
    };
    if let ArgKind::Choice(options) = a.kind {
        v["enum"] = json!(options);
    }
    if !a.desc.is_empty() {
        v["description"] = json!(a.desc);
    }
    v
}

/// The tools a model can call, generated from the op registry
/// ([`crosure_session::OPS`]). Each maps to one recorded Crosure op.
///
/// ```
/// let tools = crosure_agent::tool_definitions();
/// assert!(tools.as_array().is_some_and(|t| t.iter().all(|t| t["input_schema"]["required"]
///     .as_array().is_some_and(|r| r.contains(&serde_json::json!("why"))))));
/// ```
pub fn tool_definitions() -> Value {
    Value::Array(
        OPS.iter()
            .map(|spec| {
                let mut properties = serde_json::Map::new();
                let mut required = Vec::new();
                for a in spec.args {
                    if let Some(name) = a.tool_field {
                        properties.insert(name.into(), arg_schema(a));
                        required.push(name);
                    }
                }
                properties.insert(
                    "why".into(),
                    json!({ "type": "string", "description": WHY }),
                );
                required.push("why");
                json!({
                    "name": spec.tool,
                    "description": spec.summary,
                    "strict": true,
                    "input_schema": {
                        "type": "object",
                        "properties": properties,
                        "required": required,
                        "additionalProperties": false,
                    }
                })
            })
            .collect(),
    )
}

/// The tools offered in `profile` (all of them for `Investigate`, none for `Ask`).
///
/// ```
/// use crosure_agent::{tool_definitions_for, Profile};
/// assert_eq!(tool_definitions_for(Profile::Ask).as_array().map(Vec::len), Some(0));
/// let ro = tool_definitions_for(Profile::ReadOnly);
/// assert!(ro.as_array().is_some_and(|t| t.iter().all(|t| t["name"] != "rename_function")));
/// ```
pub fn tool_definitions_for(profile: crate::Profile) -> Value {
    Value::Array(
        tool_definitions()
            .as_array()
            .into_iter()
            .flatten()
            .filter(|t| t["name"].as_str().is_some_and(|n| profile.allows(n)))
            .cloned()
            .collect(),
    )
}

/// Validates a `tool_use` block's input and maps it to an op, using the
/// same argument rules as the console.
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
    let spec = spec_for_tool(name).ok_or_else(|| format!("unknown tool `{name}`"))?;
    let why = input["why"].as_str().unwrap_or_default().trim().to_string();
    let mut fields = serde_json::Map::new();
    for a in spec.args {
        let value = a.tool_field.map_or(&Value::Null, |f| &input[f]);
        fields.insert(a.field.into(), check_arg(a, value)?);
    }
    fields.insert("op".into(), json!(spec.op));
    let op: Op = serde_json::from_value(Value::Object(fields)).map_err(|e| e.to_string())?;
    Ok(ToolCall { op, why })
}
