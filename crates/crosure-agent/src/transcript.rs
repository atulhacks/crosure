use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::policy::Profile;
use crate::prompt::SYSTEM_PROMPT;

/// One piece of a model turn, in a provider-neutral form.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Block {
    Text(String),
    /// A reasoning summary (shown to the analyst, never sent to another provider).
    Thinking(String),
    ToolUse {
        id: String,
        name: String,
        input: Value,
    },
}

/// Why a model turn ended.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Stop {
    EndTurn,
    ToolUse,
    /// Server-side work paused; send the transcript again to resume.
    PauseTurn,
    MaxTokens,
    /// A safety system declined the request.
    Refusal {
        category: Option<String>,
        explanation: Option<String>,
    },
}

/// One model turn.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Turn {
    pub blocks: Vec<Block>,
    pub stop: Stop,
    pub input_tokens: u64,
    pub output_tokens: u64,
    /// The provider-native assistant message, echoed back verbatim to the same
    /// provider (Anthropic requires thinking blocks to come back unchanged).
    pub raw: Value,
}

impl Turn {
    /// Text blocks joined.
    pub fn text(&self) -> String {
        self.blocks
            .iter()
            .filter_map(|b| match b {
                Block::Text(t) if !t.trim().is_empty() => Some(t.trim().to_string()),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join("\n\n")
    }
}

/// The result of one tool call, sent back to the model.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ToolResult {
    pub id: String,
    pub content: String,
    pub is_error: bool,
}

/// A conversation entry.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Entry {
    User(String),
    /// A turn, tagged with the provider id that produced it.
    Assistant {
        provider: String,
        turn: Turn,
    },
    /// Tool results, plus an optional instruction appended after them.
    Results {
        results: Vec<ToolResult>,
        note: Option<String>,
    },
}

/// The whole conversation (a thread), rendered per provider on every request.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Transcript {
    pub entries: Vec<Entry>,
    /// Analyst instructions appended to the system prompt for every request.
    #[serde(default)]
    pub instructions: String,
    /// Which tools the model may use in this thread.
    #[serde(default)]
    pub profile: Profile,
}

impl Transcript {
    /// The system prompt with the analyst's instructions appended.
    ///
    /// ```
    /// let mut t = crosure_agent::Transcript::default();
    /// t.instructions = "Map behaviour to MITRE ATT&CK.".into();
    /// assert!(t.system_prompt().ends_with("Map behaviour to MITRE ATT&CK."));
    /// ```
    pub fn system_prompt(&self) -> String {
        let extra = self.instructions.trim();
        if extra.is_empty() {
            SYSTEM_PROMPT.to_string()
        } else {
            format!("{SYSTEM_PROMPT}\n\nAnalyst instructions (follow these):\n{extra}")
        }
    }

    /// Adds a user message, folding it into a trailing user-side entry so
    /// roles keep alternating (e.g. after a stopped or failed run).
    ///
    /// ```
    /// use crosure_agent::{Entry, Transcript};
    /// let mut t = Transcript::default();
    /// t.push_user("first");
    /// t.push_user("second");
    /// assert_eq!(t.entries.len(), 1);
    /// ```
    pub fn push_user(&mut self, text: &str) {
        match self.entries.last_mut() {
            Some(Entry::User(prev)) => {
                prev.push_str("\n\n");
                prev.push_str(text);
            }
            Some(Entry::Results { note, .. }) => match note {
                Some(n) => {
                    n.push_str("\n\n");
                    n.push_str(text);
                }
                None => *note = Some(text.to_string()),
            },
            _ => self.entries.push(Entry::User(text.to_string())),
        }
    }
}
