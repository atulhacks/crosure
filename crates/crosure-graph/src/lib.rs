//! Crosure investigation graph.
//!
//! Turns the flat, hash-chained list of recorded steps into the graph the
//! canvas draws: one node per step, typed edges, annotations folded into
//! the step they describe, and the path that led to each finding.
//!
//! ```
//! use crosure_graph::build;
//! use crosure_recorder::{NewStep, StepKind, Store};
//!
//! let store = Store::open_in_memory()?;
//! let s = store.create_session("demo", "/bin/true", "00")?;
//! store.record(&s.id, NewStep::human(StepKind::Load, "crosure"))?;
//! store.record(&s.id, NewStep::human(StepKind::Strings, "crosure"))?;
//! let graph = build(&store.steps(&s.id)?);
//! assert_eq!(graph.nodes.len(), 2);
//! assert_eq!(graph.edges.len(), 1);
//! # Ok::<(), crosure_recorder::RecorderError>(())
//! ```

mod build;
mod model;
mod paths;

pub use build::{build, label};
pub use model::{GraphEdge, GraphNode, GraphStats, InvestigationGraph};
pub use paths::replay;
