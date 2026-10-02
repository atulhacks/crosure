use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// Which tools a thread may use (what Zed calls an agent profile).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Profile {
    /// Every tool: read, rename, comment, record findings and a verdict.
    #[default]
    Investigate,
    /// Analysis and records, but nothing that changes how the binary reads
    /// (no renames, no comments).
    ReadOnly,
    /// No tools: answer from the conversation and attached context only.
    Ask,
}

/// Tools that change the analysis state others will read.
pub const WRITE_TOOLS: [&str; 2] = ["rename_function", "add_comment"];

impl Profile {
    /// Whether `tool` is offered to the model in this profile.
    ///
    /// ```
    /// use crosure_agent::Profile;
    /// assert!(Profile::Investigate.allows("rename_function"));
    /// assert!(!Profile::ReadOnly.allows("rename_function"));
    /// assert!(Profile::ReadOnly.allows("disassemble"));
    /// assert!(!Profile::Ask.allows("disassemble"));
    /// ```
    pub fn allows(self, tool: &str) -> bool {
        match self {
            Profile::Investigate => true,
            Profile::ReadOnly => !WRITE_TOOLS.contains(&tool),
            Profile::Ask => false,
        }
    }
}

/// What happens when the model calls a tool (Zed's tool permissions).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Permission {
    #[default]
    Allow,
    /// Ask the analyst first.
    Confirm,
    Deny,
}

/// Per-tool permissions; unlisted tools are allowed.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Permissions(pub BTreeMap<String, Permission>);

impl Permissions {
    /// The permission for `tool`.
    ///
    /// ```
    /// use crosure_agent::{Permission, Permissions};
    /// let p = Permissions::confirm_changes();
    /// assert_eq!(p.get("rename_function"), Permission::Confirm);
    /// assert_eq!(p.get("disassemble"), Permission::Allow);
    /// ```
    pub fn get(&self, tool: &str) -> Permission {
        self.0.get(tool).copied().unwrap_or_default()
    }

    /// A preset: confirm renames, comments and the verdict; allow the rest.
    pub fn confirm_changes() -> Self {
        Self(
            WRITE_TOOLS
                .iter()
                .chain(["record_verdict"].iter())
                .map(|t| (t.to_string(), Permission::Confirm))
                .collect(),
        )
    }
}

/// A tool call waiting for the analyst's decision.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ApprovalRequest {
    pub id: String,
    pub tool: String,
    /// The command the call would record, e.g. `ren sub_1189 xor_decode`.
    pub command: String,
    pub why: String,
}
