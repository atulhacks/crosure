use serde_json::Value;

const MAX_CHARS: usize = 20_000;

fn hex(v: &Value) -> String {
    v.as_u64().map_or_else(|| "?".into(), |n| format!("{n:#x}"))
}

fn text(v: &Value) -> &str {
    v.as_str().unwrap_or("")
}

fn rows(items: &Value, line: impl Fn(&Value) -> String) -> Vec<String> {
    items
        .as_array()
        .map(|a| a.iter().map(line).collect())
        .unwrap_or_default()
}

/// The header, then `lines` from `offset`, cut to fit the budget. A cut
/// says exactly where to continue.
fn clip(lines: Vec<String>, header: String, offset: usize) -> String {
    let mut out = header;
    let total = lines.len();
    if offset > 0 {
        out.push_str(&format!("\n[lines {offset}..{total} of {total}]"));
    }
    for (i, l) in lines.iter().enumerate().skip(offset) {
        if out.len() + l.len() + 1 > MAX_CHARS {
            out.push_str(&format!(
                "\n[truncated at line {i} of {total}: call again with offset {i} to continue, or narrow with a filter]"
            ));
            break;
        }
        out.push('\n');
        out.push_str(l);
    }
    out
}

/// Renders an op's result as compact text for the model (not JSON, which
/// costs ~3x the tokens for the same listing), starting at line `offset`.
///
/// ```
/// use serde_json::json;
/// let r = json!({"functions": [{"addr": 4489, "name": "decode", "size": 80, "source": "symbol"}]});
/// let t = crosure_agent::render_result("functions", "1 functions", &r, 0);
/// assert!(t.contains("0x1189 decode (80 bytes)"));
/// ```
pub fn render_result(kind: &str, summary: &str, r: &Value, offset: usize) -> String {
    let header = summary.to_string();
    match kind {
        "functions" => clip(
            rows(&r["functions"], |f| {
                format!(
                    "{} {} ({} bytes)",
                    hex(&f["addr"]),
                    text(&f["name"]),
                    f["size"]
                )
            }),
            header,
            offset,
        ),
        "disasm" => {
            let leaders: Vec<u64> = r["blocks"]
                .as_array()
                .map(|b| {
                    b.iter()
                        .skip(1)
                        .filter_map(|b| b["addr"].as_u64())
                        .collect()
                })
                .unwrap_or_default();
            let comments: Vec<(u64, String)> = r["comments"]
                .as_array()
                .map(|c| {
                    c.iter()
                        .filter_map(|p| Some((p[0].as_u64()?, p[1].as_str()?.to_string())))
                        .collect()
                })
                .unwrap_or_default();
            let mut lines = Vec::new();
            for i in r["instructions"].as_array().into_iter().flatten() {
                let addr = i["addr"].as_u64().unwrap_or(0);
                if leaders.contains(&addr) {
                    lines.push(format!("loc_{addr:x}:"));
                }
                let mut l = format!(
                    "  {:#x}  {} {}",
                    addr,
                    text(&i["mnemonic"]),
                    text(&i["operands"])
                );
                if let Some(c) = i["comment"].as_str() {
                    l.push_str(&format!("  ; {c}"));
                }
                if let Some((_, c)) = comments.iter().find(|(a, _)| *a == addr) {
                    l.push_str(&format!("  ; [note] {c}"));
                }
                lines.push(l);
            }
            clip(
                lines,
                format!(
                    "{header}\n{} @ {}:",
                    text(&r["function"]["name"]),
                    hex(&r["function"]["addr"])
                ),
                offset,
            )
        }
        "decompile" => clip(
            rows(&r["lines"], |l| text(&l["text"]).to_string()),
            format!(
                "{header}\n{} @ {}:",
                text(&r["function"]["name"]),
                hex(&r["function"]["addr"])
            ),
            offset,
        ),
        "xref" => {
            let names = r["names"].as_array();
            clip(
                rows(&r["refs"], |x| match names {
                    Some(_) => format!(
                        "{} {} -> {}",
                        hex(&x["from"]),
                        text(&x["kind"]),
                        hex(&x["to"])
                    ),
                    None => format!(
                        "{} {} from {}",
                        hex(&x["from"]),
                        text(&x["kind"]),
                        x["from_func"].as_str().unwrap_or("?")
                    ),
                }),
                header,
                offset,
            )
        }
        "strings" => clip(
            rows(&r["strings"], |s| {
                format!(
                    "{} {} {:?}",
                    hex(&s["addr"]),
                    s["section"].as_str().unwrap_or("-"),
                    text(&s["value"])
                )
            }),
            header,
            offset,
        ),
        "imports" => clip(
            rows(&r["imports"], |i| {
                format!(
                    "{} ({}) @ {}",
                    text(&i["name"]),
                    text(&i["library"]),
                    hex(&i["addr"])
                )
            }),
            header,
            offset,
        ),
        "navigate" => format!(
            "{header}\n{}",
            text(&r["hex"])
                .as_bytes()
                .chunks(32)
                .map(|c| String::from_utf8_lossy(c).into_owned())
                .collect::<Vec<_>>()
                .join("\n")
        ),
        "recall" => {
            let mut lines = Vec::new();
            if r["ambiguous"].as_bool() == Some(true) {
                lines.push(format!(
                    "ambiguous: this code appears {} time(s) in this binary or under several names; treat names as hints",
                    r["copies_in_binary"]
                ));
            }
            for m in r["matches"].as_array().into_iter().flatten() {
                let list = |k: &str| {
                    m[k].as_array()
                        .map(|a| {
                            a.iter()
                                .filter_map(Value::as_str)
                                .collect::<Vec<_>>()
                                .join("; ")
                        })
                        .unwrap_or_default()
                };
                let mut l = format!(
                    "- {} ({} match, session \"{}\"): seen as {}",
                    text(&m["binary"]),
                    text(&m["level"]),
                    text(&m["session_name"]),
                    list("names")
                );
                for (k, label) in [
                    ("renamed_to", "renamed to"),
                    ("comments", "comments"),
                    ("notes", "conclusions"),
                ] {
                    let v = list(k);
                    if !v.is_empty() {
                        l.push_str(&format!("; {label}: {v}"));
                    }
                }
                lines.push(l);
            }
            clip(lines, header, 0)
        }
        "recon" => {
            let mut v = r.clone();
            if let Some(o) = v.as_object_mut() {
                o.remove("path");
            }
            let mut out = format!("{header}\n{v}");
            if out.len() > MAX_CHARS {
                let cut = (0..=MAX_CHARS)
                    .rev()
                    .find(|i| out.is_char_boundary(*i))
                    .unwrap_or(0);
                out.truncate(cut);
                out.push_str("\n[truncated]");
            }
            out
        }
        _ => header,
    }
}
