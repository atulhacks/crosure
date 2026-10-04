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

mod cfg;
mod decompile;
mod engine;
mod error;
mod fhash;
mod native;
mod operand;
mod types;

pub use cfg::build_cfg;
pub use decompile::{
    decompile, decompiler_status, parse_pdgj, DecompLine, Decompiled, DecompilerStatus,
    DECOMPILER_HINT,
};
pub use engine::Engine;
pub use error::EngineError;
pub use fhash::{function_hash, FunctionHash, MIN_INSNS};
pub use native::NativeEngine;
pub use operand::{parse_branch_target, parse_rip_relative};
pub use types::{
    BasicBlock, BinaryInfo, BlockEdge, EdgeKind, FunctionInfo, Import, Instruction, SectionInfo,
    StringRef, Xref, XrefKind,
};
