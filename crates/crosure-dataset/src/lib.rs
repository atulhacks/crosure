//! Turns recorded investigations into training data.
//!
//! Every session is verified first: a session whose hash chain does not
//! verify is skipped unless asked otherwise. An export directory holds:
//!
//! - `trajectories.jsonl`: one investigation per line, every step with its
//!   command, target, reason, summary, tags, key-path flag and hash;
//! - `sft.jsonl`: chat-format examples (context → next step and why);
//! - `dpo.jsonl`: preference pairs from branches and dead ends;
//! - `manifest.json`: options, counts, and each session's head hash.
//!
//! Local paths are reduced to file names and actor ids are already
//! anonymous, so exports can be shared.

mod dpo;
mod record;
mod sft;
mod write;

pub use dpo::{preference_pairs, PairMeta, PairRule, PreferencePair};
pub use record::{trajectory, Link, TrajStep, Trajectory, TRAJECTORY_FORMAT};
pub use sft::{
    prompt_at, sft_examples, ExampleMeta, Message, SftExample, SftOptions, SYSTEM_PROMPT,
};
pub use write::{collect, export, ExportError, ExportOptions, Manifest, SessionEntry};
