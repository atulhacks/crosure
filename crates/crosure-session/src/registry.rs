//! The one description of every op. The console grammar and help, the
//! agent/MCP tool schemas, and which tools write are all derived from
//! [`OPS`], so they cannot drift apart.

/// How an argument is written and checked.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ArgKind {
    /// A function name or an address, one word.
    Target,
    /// One word with no spaces (a new name).
    Word,
    /// Free text. On the console it takes the rest of the line.
    Text,
    /// A non-negative integer, clamped to `max`.
    Count { max: u64 },
    /// One of a fixed set of words.
    Choice(&'static [&'static str]),
}

/// Where an argument goes on the console line.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Placement {
    /// In order after the command. An absent optional one before a later
    /// argument is written `-`.
    Positional,
    /// `--name value`, anywhere on the line.
    Flag(&'static str),
}

/// One argument of an op.
#[derive(Clone, Copy, Debug)]
pub struct ArgSpec {
    /// Field name in [`crate::Op`] (and in recorded step actions).
    pub field: &'static str,
    /// Field name in the agent tool's input, or `None` if agents cannot set it.
    pub tool_field: Option<&'static str>,
    pub kind: ArgKind,
    pub required: bool,
    pub placement: Placement,
    /// Console placeholder, e.g. `fn|addr`.
    pub hint: &'static str,
    /// Description in the agent schema (empty: none).
    pub desc: &'static str,
}

/// One op: its names on every surface and its arguments.
#[derive(Clone, Copy, Debug)]
pub struct OpSpec {
    /// The `op` tag in [`crate::Op`]'s JSON.
    pub op: &'static str,
    /// Canonical console command, stored as every step's command.
    pub command: &'static str,
    /// Other console spellings (r2 style).
    pub aliases: &'static [&'static str],
    /// Agent / MCP tool name.
    pub tool: &'static str,
    /// Agent tool description.
    pub summary: &'static str,
    /// One-line console help.
    pub help: &'static str,
    /// Changes analysis state others read (renames, comments).
    pub writes: bool,
    pub args: &'static [ArgSpec],
}

pub use crate::table::{OPS, VERDICTS};

/// The op a console command or alias names.
///
/// ```
/// assert_eq!(crosure_session::spec_for_command("axt").map(|s| s.tool), Some("xrefs_to"));
/// ```
pub fn spec_for_command(name: &str) -> Option<&'static OpSpec> {
    OPS.iter()
        .find(|s| s.command == name || s.aliases.contains(&name))
}

/// The op an agent tool name maps to.
///
/// ```
/// assert_eq!(crosure_session::spec_for_tool("read_bytes").map(|s| s.command), Some("hex"));
/// ```
pub fn spec_for_tool(name: &str) -> Option<&'static OpSpec> {
    OPS.iter().find(|s| s.tool == name)
}

/// The spec with `op` tag `tag`.
pub(crate) fn spec_for_tag(tag: &str) -> Option<&'static OpSpec> {
    OPS.iter().find(|s| s.op == tag)
}

/// Whether an agent tool changes analysis state others read.
///
/// ```
/// assert!(crosure_session::is_write_tool("rename_function"));
/// assert!(!crosure_session::is_write_tool("disassemble"));
/// ```
pub fn is_write_tool(name: &str) -> bool {
    spec_for_tool(name).is_some_and(|s| s.writes)
}

/// Console help, one line per op, generated from [`OPS`].
///
/// ```
/// let help = crosure_session::console_help();
/// assert!(help.lines().any(|l| l.starts_with("hex <addr> [len]")));
/// assert!(help.contains("str [--min n] [filter]"));
/// ```
pub fn console_help() -> String {
    OPS.iter()
        .map(|s| {
            let mut usage = s.command.to_string();
            for a in s
                .args
                .iter()
                .filter(|a| matches!(a.placement, Placement::Flag(_)))
            {
                if let Placement::Flag(flag) = a.placement {
                    usage.push_str(&format!(" [{flag} {}]", a.hint));
                }
            }
            for a in s
                .args
                .iter()
                .filter(|a| a.placement == Placement::Positional)
            {
                if a.required {
                    usage.push_str(&format!(" <{}>", a.hint));
                } else {
                    usage.push_str(&format!(" [{}]", a.hint));
                }
            }
            format!("{usage:<28} {}", s.help)
        })
        .collect::<Vec<_>>()
        .join("\n")
}
