//! Crosure analysis engine.
//!
//! One [`Engine`] trait, so the app and the recorder never depend on a
//! specific backend. The default backend is [`NativeEngine`]: pure Rust
//! parsing (`object`) plus Capstone disassembly, no external tools.
//!
//! ```no_run
//! use crosure_engine::{Engine, NativeEngine};
//! let engine = NativeEngine::open(std::path::Path::new("/bin/ls"))?;
//! println!("{} functions", engine.functions()?.len());
//! # Ok::<(), crosure_engine::EngineError>(())
//! ```

mod engine;
mod error;
mod native;
mod operand;
mod types;

pub use engine::Engine;
pub use error::EngineError;
pub use native::NativeEngine;
pub use operand::{parse_branch_target, parse_rip_relative};
pub use types::{
    BinaryInfo, FunctionInfo, Import, Instruction, SectionInfo, StringRef, Xref, XrefKind,
};
