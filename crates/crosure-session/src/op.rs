use crosure_recorder::StepKind;
use serde::{Deserialize, Serialize};

/// Where an op came from. Console ops keep the typed command as their label.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Origin {
    Ui,
    Console,
    /// An AI agent's tool call.
    Agent,
}

/// One analysis action. `target` accepts a name (`main`, `strcmp`) or an
/// address (`0x401000`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum Op {
    Info,
    Functions {
        filter: Option<String>,
        /// First line to show the agent (paging); `None` for the start.
        offset: Option<usize>,
    },
    Disasm {
        target: String,
        /// First line to show the agent (paging); `None` for the start.
        offset: Option<usize>,
    },
    /// Pseudo-C of a function (needs rizin + rz-ghidra).
    Decompile {
        target: String,
        /// First line to show the agent (paging); `None` for the start.
        offset: Option<usize>,
    },
    XrefsTo {
        target: String,
        /// First line to show the agent (paging); `None` for the start.
        offset: Option<usize>,
    },
    XrefsFrom {
        target: String,
        /// First line to show the agent (paging); `None` for the start.
        offset: Option<usize>,
    },
    Strings {
        filter: Option<String>,
        min_len: Option<usize>,
        /// First line to show the agent (paging); `None` for the start.
        offset: Option<usize>,
    },
    Imports,
    Hex {
        target: String,
        len: Option<usize>,
    },
    Rename {
        target: String,
        name: String,
    },
    Comment {
        target: String,
        text: String,
    },
    Hypothesis {
        text: String,
    },
    Finding {
        text: String,
    },
    Verdict {
        verdict: String,
        family: Option<String>,
        text: String,
    },
}

impl Op {
    /// The step kind this op is recorded as.
    ///
    /// ```
    /// use crosure_recorder::StepKind;
    /// use crosure_session::Op;
    /// assert_eq!(Op::Imports.kind(), StepKind::Imports);
    /// ```
    pub fn kind(&self) -> StepKind {
        match self {
            Op::Info => StepKind::Recon,
            Op::Functions { .. } => StepKind::Functions,
            Op::Disasm { .. } => StepKind::Disasm,
            Op::Decompile { .. } => StepKind::Decompile,
            Op::XrefsTo { .. } | Op::XrefsFrom { .. } => StepKind::Xref,
            Op::Strings { .. } => StepKind::Strings,
            Op::Imports => StepKind::Imports,
            Op::Hex { .. } => StepKind::Navigate,
            Op::Rename { .. } => StepKind::Rename,
            Op::Comment { .. } => StepKind::Comment,
            Op::Hypothesis { .. } => StepKind::Hypothesis,
            Op::Finding { .. } => StepKind::Finding,
            Op::Verdict { .. } => StepKind::Verdict,
        }
    }

    /// The same op aimed at `target` (ops without a target are returned as-is).
    ///
    /// ```
    /// use crosure_session::Op;
    /// let op = Op::Disasm { target: "0x1189".into(), offset: None }.with_target("decode");
    /// assert_eq!(op.command(), "dis decode");
    /// ```
    pub fn with_target(&self, target: &str) -> Op {
        let t = target.to_string();
        match self.clone() {
            Op::Disasm { offset, .. } => Op::Disasm { target: t, offset },
            Op::Decompile { offset, .. } => Op::Decompile { target: t, offset },
            Op::XrefsTo { offset, .. } => Op::XrefsTo { target: t, offset },
            Op::XrefsFrom { offset, .. } => Op::XrefsFrom { target: t, offset },
            Op::Hex { len, .. } => Op::Hex { target: t, len },
            Op::Rename { name, .. } => Op::Rename { target: t, name },
            Op::Comment { text, .. } => Op::Comment { target: t, text },
            other => other,
        }
    }

    /// The canonical console form of the op, stored as the step's command
    /// whatever the origin, so datasets see one action vocabulary. It parses
    /// back to the same op.
    ///
    /// ```
    /// use crosure_session::{parse_command, Op};
    /// let op = Op::XrefsTo { target: "strcmp".into(), offset: None };
    /// assert_eq!(op.command(), "xt strcmp");
    /// let hex = Op::Hex { target: "0x1000".into(), len: None };
    /// assert_eq!(parse_command(&hex.command())?, hex);
    /// # Ok::<(), crosure_session::SessionError>(())
    /// ```
    pub fn command(&self) -> String {
        crate::console::render(self)
    }

    /// First result line the agent asked to see (0 when not paging).
    ///
    /// ```
    /// use crosure_session::Op;
    /// assert_eq!(Op::Disasm { target: "main".into(), offset: Some(400) }.offset(), 400);
    /// assert_eq!(Op::Imports.offset(), 0);
    /// ```
    pub fn offset(&self) -> usize {
        match self {
            Op::Functions { offset, .. }
            | Op::Disasm { offset, .. }
            | Op::Decompile { offset, .. }
            | Op::XrefsTo { offset, .. }
            | Op::XrefsFrom { offset, .. }
            | Op::Strings { offset, .. } => offset.unwrap_or(0),
            _ => 0,
        }
    }
}
