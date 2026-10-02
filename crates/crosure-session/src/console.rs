use crate::{Op, SessionError};

/// Help text for the built-in console.
pub const CONSOLE_HELP: &str = "\
info                         binary info
fns [filter]                 list functions
dis <fn|addr>                disassemble function
dec <fn|addr>                decompile function (rizin + rz-ghidra)
xt <fn|addr>                 xrefs to
xf <fn|addr>                 xrefs from function
str [filter]                 strings (optionally filtered)
imp                          imports
hex <addr> [len]             hex dump
ren <fn|addr> <name>         rename function
cmt <addr> <text>            comment
hyp <text>                   pin a hypothesis
find <text>                  record a finding
verdict <malicious|benign|unknown> <family|-> <text>";

fn arg(parts: &[&str], i: usize, line: &str) -> Result<String, SessionError> {
    parts
        .get(i)
        .map(|s| s.to_string())
        .ok_or_else(|| SessionError::BadCommand(line.into()))
}

fn rest(parts: &[&str], from: usize, line: &str) -> Result<String, SessionError> {
    let text = parts.get(from..).map(|p| p.join(" ")).unwrap_or_default();
    if text.is_empty() {
        return Err(SessionError::BadCommand(line.into()));
    }
    Ok(text)
}

/// Parses one console line into an [`Op`].
///
/// ```
/// use crosure_session::{parse_command, Op};
/// assert_eq!(parse_command("dis main")?, Op::Disasm { target: "main".into() });
/// assert_eq!(parse_command("str http")?, Op::Strings { filter: Some("http".into()), min_len: None });
/// assert!(parse_command("frobnicate").is_err());
/// # Ok::<(), crosure_session::SessionError>(())
/// ```
pub fn parse_command(line: &str) -> Result<Op, SessionError> {
    let parts: Vec<&str> = line.split_whitespace().collect();
    let Some(cmd) = parts.first() else {
        return Err(SessionError::BadCommand(line.into()));
    };
    Ok(match *cmd {
        "info" | "i" => Op::Info,
        "fns" | "afl" => Op::Functions {
            filter: parts
                .get(1..)
                .map(|p| p.join(" "))
                .filter(|s| !s.is_empty()),
        },
        "dis" | "pdf" => Op::Disasm {
            target: arg(&parts, 1, line)?,
        },
        "dec" | "pdg" => Op::Decompile {
            target: arg(&parts, 1, line)?,
        },
        "xt" | "axt" => Op::XrefsTo {
            target: arg(&parts, 1, line)?,
        },
        "xf" | "axf" => Op::XrefsFrom {
            target: arg(&parts, 1, line)?,
        },
        "str" | "iz" => Op::Strings {
            filter: parts
                .get(1..)
                .map(|p| p.join(" "))
                .filter(|s| !s.is_empty()),
            min_len: None,
        },
        "imp" | "ii" => Op::Imports,
        "hex" | "px" => Op::Hex {
            target: arg(&parts, 1, line)?,
            len: match parts.get(2) {
                Some(n) => Some(
                    n.parse()
                        .map_err(|_| SessionError::BadCommand(line.into()))?,
                ),
                None => None,
            },
        },
        "ren" | "afn" => Op::Rename {
            target: arg(&parts, 1, line)?,
            name: arg(&parts, 2, line)?,
        },
        "cmt" | "CC" => Op::Comment {
            target: arg(&parts, 1, line)?,
            text: rest(&parts, 2, line)?,
        },
        "hyp" => Op::Hypothesis {
            text: rest(&parts, 1, line)?,
        },
        "find" => Op::Finding {
            text: rest(&parts, 1, line)?,
        },
        "verdict" => Op::Verdict {
            verdict: arg(&parts, 1, line)?,
            family: arg(&parts, 2, line).ok().filter(|f| f != "-"),
            text: parts.get(3..).map(|p| p.join(" ")).unwrap_or_default(),
        },
        _ => return Err(SessionError::BadCommand(line.into())),
    })
}
