//! Every op in the registry round-trips through every surface.

use crosure_session::{parse_command, ArgKind, Op, Placement, OPS};
use serde_json::{Map, Value};

/// A sample value for an argument, as the console would write it.
fn sample(kind: ArgKind) -> Value {
    match kind {
        ArgKind::Target => Value::from("check_password"),
        ArgKind::Word => Value::from("xor_decode"),
        ArgKind::Text => Value::from("decodes the secret with key 0x43"),
        ArgKind::Count { .. } => Value::from(64),
        ArgKind::Choice(options) => Value::from(options[0]),
    }
}

/// Every op, once with all arguments and once with only the required ones.
fn samples() -> Vec<Op> {
    let mut out = Vec::new();
    for spec in OPS {
        for all in [true, false] {
            let mut m = Map::new();
            m.insert("op".into(), Value::from(spec.op));
            for a in spec.args {
                let v = if a.required || all {
                    sample(a.kind)
                } else {
                    Value::Null
                };
                m.insert(a.field.into(), v);
            }
            if let Ok(op) = serde_json::from_value(Value::Object(m)) {
                out.push(op);
            }
        }
    }
    out
}

#[test]
fn every_op_round_trips_through_the_console() -> Result<(), Box<dyn std::error::Error>> {
    let ops = samples();
    assert_eq!(ops.len(), OPS.len() * 2, "every spec builds an Op");
    for op in ops {
        let line = op.command();
        assert_eq!(parse_command(&line)?, op, "{line}");
    }
    Ok(())
}

#[test]
fn aliases_are_unique_and_parse() -> Result<(), Box<dyn std::error::Error>> {
    let mut seen = std::collections::BTreeSet::new();
    for spec in OPS {
        for name in std::iter::once(&spec.command).chain(spec.aliases) {
            assert!(seen.insert(*name), "`{name}` used twice");
        }
        assert!(
            seen.insert(spec.tool) || spec.tool == spec.command,
            "tool `{}`",
            spec.tool
        );
        // Only the last positional argument may take the rest of the line.
        let pos: Vec<_> = spec
            .args
            .iter()
            .filter(|a| a.placement == Placement::Positional)
            .collect();
        for a in pos.iter().rev().skip(1) {
            assert_ne!(a.kind, ArgKind::Text, "{}: text must be last", spec.op);
        }
    }
    Ok(())
}

#[test]
fn frontend_op_type_names_every_op() -> Result<(), Box<dyn std::error::Error>> {
    let ts = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tauri/src/types.ts"),
    )?;
    let start = ts.find("export type Op =").ok_or("no Op type")?;
    let block = &ts[start..ts[start..].find(";\n\n").map_or(ts.len(), |e| start + e)];
    let mut names: Vec<&str> = block
        .split("op: \"")
        .skip(1)
        .filter_map(|s| s.split('"').next())
        .collect();
    names.sort_unstable();
    let mut expected: Vec<&str> = OPS.iter().map(|s| s.op).collect();
    expected.sort_unstable();
    assert_eq!(names, expected);
    Ok(())
}
