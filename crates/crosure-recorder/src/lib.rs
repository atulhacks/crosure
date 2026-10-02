//! Crosure recorder: every step an analyst (or agent) takes while reversing
//! a binary is stored here as an append-only, hash-chained record.
//!
//! ```
//! use crosure_recorder::{NewStep, StepKind, Store};
//!
//! let store = Store::open_in_memory()?;
//! let session = store.create_session("crackme", "/tmp/crackme", "ab12")?;
//! let step = store.record(&session.id, NewStep::human(StepKind::Strings, "crosure"))?;
//! assert_eq!(step.seq, 0);
//! assert!(store.verify(&session.id)?.ok);
//! # Ok::<(), crosure_recorder::RecorderError>(())
//! ```

mod chain;
mod error;
mod schema;
mod step;
mod store;
mod verify;

pub use chain::{canonical_json, genesis_hash, step_hash};
pub use error::RecorderError;
pub use step::{
    Actor, ActorKind, Attention, Intent, NewStep, Observation, ParentRef, Relation, Step, StepKind,
    Target, CONTEXT_CHIP,
};
pub use store::{Session, Store};
pub use verify::{VerifyFailure, VerifyReport};
