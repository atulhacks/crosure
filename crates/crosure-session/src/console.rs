use serde_json::{Map, Value};

use crate::registry::{spec_for_command, spec_for_tag, ArgKind, ArgSpec, OpSpec, Placement};
use crate::{Op, SessionError};

/// Checks one argument value against its spec and normalizes it: strings
/// trimmed of nothing but checked for content, counts clamped to their
/// maximum. `Null` (or an empty string) means absent.
///
/// ```
/// use crosure_session::{check_arg, spec_for_tool};
/// use serde_json::json;
/// let hex = spec_for_tool("read_bytes").unwrap_or_else(|| unreachable!());
/// assert_eq!(check_arg(&hex.args[1], &json!(9000))?, json!(4096));
/// assert!(check_arg(&hex.args[0], &json!("")).is_err());
/// # Ok::<(), String>(())
/// ```
pub fn check_arg(arg: &ArgSpec, value: &Value) -> Result<Value, String> {
    let name = arg.tool_field.unwrap_or(arg.field);
    let present = match value {
        Value::Null => false,
        Value::String(s) => !s.trim().is_empty(),
        _ => true,
    };
    if !present {
        return if arg.required {
            Err(format!("missing or empty `{name}`"))
        } else {
            Ok(Value::Null)
        };
    }
    match (arg.kind, value) {
        (ArgKind::Count { max }, Value::Number(n)) => n
            .as_u64()
            .map(|n| Value::from(n.min(max)))
            .ok_or_else(|| format!("`{name}` must be a non-negative integer")),
        (ArgKind::Count { max }, Value::String(s)) => s
            .trim()
            .parse::<u64>()
            .map(|n| Value::from(n.min(max)))
            .map_err(|_| format!("`{name}` must be a non-negative integer")),
        (ArgKind::Word, Value::String(s)) if s.trim().contains(char::is_whitespace) => {
            Err(format!("`{name}` must not contain spaces"))
        }
        (ArgKind::Choice(options), Value::String(s)) if !options.contains(&s.trim()) => {
            Err(format!("`{name}` must be one of: {}", options.join(", ")))
        }
        (ArgKind::Text, Value::String(s)) => Ok(Value::from(s.as_str())),
        (_, Value::String(s)) => Ok(Value::from(s.trim())),
        _ => Err(format!("`{name}` has the wrong type")),
    }
}

/// Builds an [`Op`] from checked fields (`field` names, not tool names).
pub(crate) fn build(spec: &OpSpec, mut fields: Map<String, Value>) -> Result<Op, String> {
    fields.insert("op".into(), Value::from(spec.op));
    serde_json::from_value(Value::Object(fields)).map_err(|e| e.to_string())
}

/// Parses one console line into an [`Op`], using the op registry.
///
/// ```
/// use crosure_session::{parse_command, Op};
/// assert_eq!(parse_command("dis main")?, Op::Disasm { target: "main".into() });
/// assert_eq!(parse_command("str http")?, Op::Strings { filter: Some("http".into()), min_len: None });
/// assert_eq!(parse_command("iz --min 8")?, Op::Strings { filter: None, min_len: Some(8) });
/// assert!(parse_command("frobnicate").is_err());
/// assert!(parse_command("verdict evil - why").is_err(), "verdicts are checked");
/// # Ok::<(), crosure_session::SessionError>(())
/// ```
pub fn parse_command(line: &str) -> Result<Op, SessionError> {
    let bad = |why: String| SessionError::BadCommand(format!("{line}: {why}"));
    let mut words: Vec<&str> = line.split_whitespace().collect();
    if words.is_empty() {
        return Err(SessionError::BadCommand(line.into()));
    }
    let cmd = words.remove(0);
    let spec = spec_for_command(cmd).ok_or_else(|| SessionError::BadCommand(line.into()))?;
    let mut fields = Map::new();

    for a in spec.args {
        if let Placement::Flag(flag) = a.placement {
            if let Some(i) = words.iter().position(|w| *w == flag) {
                let v = words
                    .get(i + 1)
                    .copied()
                    .ok_or_else(|| bad(format!("{flag} needs a value")))?;
                fields.insert(a.field.into(), check_arg(a, &Value::from(v)).map_err(bad)?);
                words.drain(i..=i + 1);
            }
        }
    }
    let mut rest = words.into_iter();
    for a in spec
        .args
        .iter()
        .filter(|a| a.placement == Placement::Positional)
    {
        let raw = if a.kind == ArgKind::Text {
            rest.by_ref().collect::<Vec<_>>().join(" ")
        } else {
            match rest.next() {
                Some("-") if !a.required => String::new(),
                Some(w) => w.to_string(),
                None => String::new(),
            }
        };
        fields.insert(
            a.field.into(),
            check_arg(a, &Value::from(raw)).map_err(bad)?,
        );
    }
    if let Some(extra) = rest.next() {
        return Err(bad(format!("unexpected `{extra}`")));
    }
    build(spec, fields).map_err(bad)
}

/// The canonical console form of `op`: parses back to the same op.
pub(crate) fn render(op: &Op) -> String {
    let v = serde_json::to_value(op).unwrap_or(Value::Null);
    let Some(spec) = v["op"].as_str().and_then(spec_for_tag) else {
        return String::new();
    };
    let word = |a: &ArgSpec| match &v[a.field] {
        Value::String(s) => Some(s.clone()),
        Value::Number(n) => Some(n.to_string()),
        _ => None,
    };
    let mut out = vec![spec.command.to_string()];
    for a in spec.args {
        if let (Placement::Flag(flag), Some(w)) = (a.placement, word(a)) {
            out.push(flag.into());
            out.push(w);
        }
    }
    let positional: Vec<&ArgSpec> = spec
        .args
        .iter()
        .filter(|a| a.placement == Placement::Positional)
        .collect();
    let last = positional.iter().rposition(|a| word(a).is_some());
    for a in positional.iter().take(last.map_or(0, |i| i + 1)) {
        out.push(word(a).unwrap_or_else(|| "-".into()));
    }
    out.join(" ")
}
