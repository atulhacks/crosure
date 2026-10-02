use crosure_recorder::StepKind;
use serde::{Deserialize, Serialize};

/// Where an op came from. Console ops keep the typed command as their label.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Origin {
    Ui,
    Console,
}

/// One analysis action. `target` accepts a name (`main`, `strcmp`) or an
/// address (`0x401000`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum Op {
    Info,
    Disasm {
        target: String,
    },
    XrefsTo {
        target: String,
    },
    XrefsFrom {
        target: String,
    },
    Strings {
        filter: Option<String>,
        min_len: Option<usize>,
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
            Op::Disasm { .. } => StepKind::Disasm,
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

    /// The canonical console form of the op, stored as the step's command
    /// whatever the origin, so datasets see one action vocabulary.
    ///
    /// ```
    /// use crosure_session::Op;
    /// let op = Op::XrefsTo { target: "strcmp".into() };
    /// assert_eq!(op.command(), "xt strcmp");
    /// ```
    pub fn command(&self) -> String {
        match self {
            Op::Info => "info".into(),
            Op::Disasm { target } => format!("dis {target}"),
            Op::XrefsTo { target } => format!("xt {target}"),
            Op::XrefsFrom { target } => format!("xf {target}"),
            Op::Strings { filter, .. } => match filter {
                Some(f) => format!("str {f}"),
                None => "str".into(),
            },
            Op::Imports => "imp".into(),
            Op::Hex { target, len } => format!("hex {target} {}", len.unwrap_or(256)),
            Op::Rename { target, name } => format!("ren {target} {name}"),
            Op::Comment { target, text } => format!("cmt {target} {text}"),
            Op::Hypothesis { text } => format!("hyp {text}"),
            Op::Finding { text } => format!("find {text}"),
            Op::Verdict {
                verdict,
                family,
                text,
            } => match family {
                Some(f) => format!("verdict {verdict} {f} {text}"),
                None => format!("verdict {verdict} - {text}"),
            },
        }
    }
}
