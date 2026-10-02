use serde_json::Value;

/// One piece of a model turn, in a provider-neutral form.
#[derive(Clone, Debug, PartialEq)]
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
#[derive(Clone, Debug, PartialEq)]
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
#[derive(Clone, Debug)]
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
#[derive(Clone, Debug, PartialEq)]
pub struct ToolResult {
    pub id: String,
    pub content: String,
    pub is_error: bool,
}

/// A conversation entry.
#[derive(Clone, Debug)]
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

/// The whole conversation, rendered per provider on every request.
#[derive(Clone, Debug, Default)]
pub struct Transcript {
    pub entries: Vec<Entry>,
}
