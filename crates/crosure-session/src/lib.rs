//! Crosure session: the single choke point between "the analyst did
//! something" and "it is recorded".
//!
//! Every analysis op (from the UI, the console, or an agent) goes through
//! [`Workspace::run`], which executes it on the engine, stores the full
//! result as a blob, and appends a hash-chained step.
//!
//! ```no_run
//! use crosure_recorder::Store;
//! use crosure_session::{Op, Origin, Workspace};
//!
//! let store = Store::open_in_memory()?;
//! let (mut ws, _load) = Workspace::open(&store, std::path::Path::new("./crackme"), None)?;
//! let out = ws.run(&store, Op::Strings { filter: Some("http".into()), min_len: None }, None, Origin::Ui)?;
//! println!("{}", out.step.observation.summary);
//! # Ok::<(), crosure_session::SessionError>(())
//! ```

mod console;
mod decomp;
mod error;
mod op;
mod run;
mod summary;
mod workspace;

pub use console::{parse_command, CONSOLE_HELP};
pub use error::SessionError;
pub use op::{Op, Origin};
pub use run::{Author, Outcome};
pub use workspace::Workspace;
