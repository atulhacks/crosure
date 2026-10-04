//! The op table itself: one entry per op, in listing order.

use crate::registry::{ArgKind, ArgSpec, OpSpec, Placement};

const TARGET_DESC: &str = "Function name (e.g. main, strcmp@plt) or address (0x...).";
/// Verdicts the recorder accepts.
pub const VERDICTS: &[&str] = &["malicious", "suspicious", "benign", "unknown"];

const fn pos(
    field: &'static str,
    tool_field: &'static str,
    kind: ArgKind,
    required: bool,
    hint: &'static str,
    desc: &'static str,
) -> ArgSpec {
    ArgSpec {
        field,
        tool_field: Some(tool_field),
        kind,
        required,
        placement: Placement::Positional,
        hint,
        desc,
    }
}

const fn target(tool_field: &'static str, hint: &'static str, desc: &'static str) -> ArgSpec {
    pos("target", tool_field, ArgKind::Target, true, hint, desc)
}

const fn text(tool_field: &'static str, desc: &'static str) -> ArgSpec {
    pos("text", tool_field, ArgKind::Text, true, "text", desc)
}

/// Every op, in the order tools and help are listed.
pub static OPS: &[OpSpec] = &[
    OpSpec {
        op: "info",
        command: "info",
        aliases: &["i"],
        tool: "binary_info",
        summary: "Format, architecture, entry point, hashes and sections of the binary.",
        help: "binary info",
        writes: false,
        args: &[],
    },
    OpSpec {
        op: "functions",
        command: "fns",
        aliases: &["afl"],
        tool: "list_functions",
        summary: "List discovered functions with addresses and sizes. Filter by substring to narrow.",
        help: "list functions",
        writes: false,
        args: &[pos("filter", "filter", ArgKind::Text, false, "filter", "Substring to match, or null for all.")],
    },
    OpSpec {
        op: "disasm",
        command: "dis",
        aliases: &["pdf"],
        tool: "disassemble",
        summary: "Disassemble a whole function. Calls are annotated with callee names, data references with string literals.",
        help: "disassemble function",
        writes: false,
        args: &[target("target", "fn|addr", TARGET_DESC)],
    },
    OpSpec {
        op: "decompile",
        command: "dec",
        aliases: &["pdg"],
        tool: "decompile",
        summary: "Pseudo-C of a whole function (rz-ghidra), using the current names. Faster to read than disassembly; may be unavailable on this machine, in which case use disassemble.",
        help: "decompile function (rizin + rz-ghidra)",
        writes: false,
        args: &[target("target", "fn|addr", TARGET_DESC)],
    },
    OpSpec {
        op: "xrefs_to",
        command: "xt",
        aliases: &["axt"],
        tool: "xrefs_to",
        summary: "Who references an address: callers of a function or import, users of a string or global.",
        help: "xrefs to",
        writes: false,
        args: &[target("target", "fn|addr", "Function, import, or address (string addresses come from search_strings).")],
    },
    OpSpec {
        op: "xrefs_from",
        command: "xf",
        aliases: &["axf"],
        tool: "xrefs_from",
        summary: "Everything a function calls or references.",
        help: "xrefs from function",
        writes: false,
        args: &[target("function", "fn|addr", TARGET_DESC)],
    },
    OpSpec {
        op: "strings",
        command: "str",
        aliases: &["iz"],
        tool: "search_strings",
        summary: "Printable strings (ASCII and UTF-16) with addresses and sections. Filter by case-insensitive substring.",
        help: "strings (optionally filtered; --min sets the minimum length, default 4)",
        writes: false,
        args: &[
            ArgSpec {
                field: "min_len",
                tool_field: None,
                kind: ArgKind::Count { max: 4096 },
                required: false,
                placement: Placement::Flag("--min"),
                hint: "n",
                desc: "",
            },
            pos("filter", "filter", ArgKind::Text, false, "filter", "Substring such as http, .exe, password; null for all."),
        ],
    },
    OpSpec {
        op: "imports",
        command: "imp",
        aliases: &["ii"],
        tool: "list_imports",
        summary: "Imported functions grouped by library, with the address code uses to call each.",
        help: "imports",
        writes: false,
        args: &[],
    },
    OpSpec {
        op: "hex",
        command: "hex",
        aliases: &["px"],
        tool: "read_bytes",
        summary: "Hex dump of raw bytes at an address (max 4096).",
        help: "hex dump (default 256 bytes, max 4096)",
        writes: false,
        args: &[
            target("address", "addr", "Address (0x...) or symbol."),
            pos("len", "length", ArgKind::Count { max: 4096 }, false, "len", "Bytes to read; null for 256."),
        ],
    },
    OpSpec {
        op: "rename",
        command: "ren",
        aliases: &["afn"],
        tool: "rename_function",
        summary: "Give a function a descriptive name once its purpose is clear.",
        help: "rename function",
        writes: true,
        args: &[
            target("target", "fn|addr", TARGET_DESC),
            pos("name", "new_name", ArgKind::Word, true, "name", "snake_case name, no spaces."),
        ],
    },
    OpSpec {
        op: "comment",
        command: "cmt",
        aliases: &["CC"],
        tool: "add_comment",
        summary: "Attach a note to an address.",
        help: "comment",
        writes: true,
        args: &[target("address", "addr", "Address (0x...)."), text("text", "The comment.")],
    },
    OpSpec {
        op: "hypothesis",
        command: "hyp",
        aliases: &[],
        tool: "record_hypothesis",
        summary: "Pin a hypothesis you are about to test.",
        help: "pin a hypothesis",
        writes: false,
        args: &[text("text", "The hypothesis.")],
    },
    OpSpec {
        op: "finding",
        command: "find",
        aliases: &[],
        tool: "record_finding",
        summary: "Record a confirmed finding with its evidence.",
        help: "record a finding",
        writes: false,
        args: &[text("text", "The finding and the evidence for it.")],
    },
    OpSpec {
        op: "verdict",
        command: "verdict",
        aliases: &[],
        tool: "record_verdict",
        summary: "Final classification of the binary. Call once, at the end.",
        help: "final verdict: malicious, suspicious, benign or unknown",
        writes: false,
        args: &[
            pos("verdict", "verdict", ArgKind::Choice(VERDICTS), true, "verdict", ""),
            pos("family", "family", ArgKind::Word, false, "family|-", "Malware family if known, else null."),
            text("summary", "One or two sentences justifying the verdict."),
        ],
    },
];
