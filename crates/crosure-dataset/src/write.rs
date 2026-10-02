use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::Path;

use crosure_recorder::{RecorderError, Store};
use serde::{Deserialize, Serialize};

use crate::dpo::preference_pairs;
use crate::record::{trajectory, Trajectory};
use crate::sft::{sft_examples, SftOptions};

/// What to export.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExportOptions {
    /// Session ids to export; empty means every session in the store.
    pub sessions: Vec<String>,
    pub sft: SftOptions,
    /// Also export sessions whose hash chain does not verify (marked `verified: false`).
    pub allow_unverified: bool,
}

/// One exported (or skipped) session.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionEntry {
    pub session_id: String,
    pub binary: String,
    pub head_hash: String,
    pub verified: bool,
    pub steps: usize,
    pub sft: usize,
    pub dpo: usize,
    /// Why the session was left out, if it was.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub skipped: Option<String>,
}

/// `manifest.json`: what is in the export and where it came from.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Manifest {
    pub format: String,
    pub files: Vec<String>,
    pub options: ExportOptions,
    pub sessions: Vec<SessionEntry>,
    pub trajectories: usize,
    pub sft_examples: usize,
    pub dpo_pairs: usize,
}

/// Errors while exporting.
#[derive(Debug)]
pub enum ExportError {
    Store(RecorderError),
    Io(std::io::Error),
    Json(serde_json::Error),
}

impl std::fmt::Display for ExportError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Store(e) => write!(f, "store: {e}"),
            Self::Io(e) => write!(f, "io: {e}"),
            Self::Json(e) => write!(f, "json: {e}"),
        }
    }
}

impl std::error::Error for ExportError {}

impl From<RecorderError> for ExportError {
    fn from(e: RecorderError) -> Self {
        Self::Store(e)
    }
}
impl From<std::io::Error> for ExportError {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e)
    }
}
impl From<serde_json::Error> for ExportError {
    fn from(e: serde_json::Error) -> Self {
        Self::Json(e)
    }
}

fn jsonl<T: Serialize>(w: &mut impl Write, items: &[T]) -> Result<(), ExportError> {
    for it in items {
        serde_json::to_writer(&mut *w, it)?;
        w.write_all(b"\n")?;
    }
    Ok(())
}

/// Verified trajectories of the chosen sessions, with an entry for each
/// session (including skipped ones).
pub fn collect(
    store: &Store,
    opts: &ExportOptions,
) -> Result<(Vec<Trajectory>, Vec<SessionEntry>), ExportError> {
    let sessions = if opts.sessions.is_empty() {
        store.sessions()?
    } else {
        opts.sessions
            .iter()
            .map(|id| store.session(id))
            .collect::<Result<_, _>>()?
    };
    let mut out = Vec::new();
    let mut entries = Vec::new();
    for session in sessions {
        let report = store.verify(&session.id)?;
        let steps = store.steps(&session.id)?;
        let t = trajectory(&session, &steps, report.ok);
        let skipped = if !report.ok && !opts.allow_unverified {
            Some("hash chain does not verify".to_string())
        } else if !t.steps.iter().any(|s| s.is_action()) {
            Some("no analysis steps".to_string())
        } else {
            None
        };
        entries.push(SessionEntry {
            session_id: t.session_id.clone(),
            binary: t.binary.clone(),
            head_hash: t.head_hash.clone(),
            verified: report.ok,
            steps: t.steps.len(),
            sft: 0,
            dpo: 0,
            skipped: skipped.clone(),
        });
        if skipped.is_none() {
            out.push(t);
        }
    }
    Ok((out, entries))
}

/// Writes `trajectories.jsonl`, `sft.jsonl`, `dpo.jsonl` and `manifest.json` into `dir`.
///
/// ```
/// use crosure_dataset::{export, ExportOptions};
/// use crosure_recorder::{NewStep, StepKind, Store};
/// let store = Store::open_in_memory()?;
/// let s = store.create_session("demo", "/tmp/demo.bin", "00")?;
/// let mut f = NewStep::human(StepKind::Finding, "crosure");
/// f.command = Some("find it decodes a key".into());
/// store.record(&s.id, f)?;
/// let dir = tempfile::tempdir()?;
/// let m = export(&store, dir.path(), &ExportOptions::default())?;
/// assert_eq!((m.trajectories, m.sft_examples), (1, 1));
/// assert!(dir.path().join("sft.jsonl").exists());
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
pub fn export(store: &Store, dir: &Path, opts: &ExportOptions) -> Result<Manifest, ExportError> {
    std::fs::create_dir_all(dir)?;
    let (trajectories, mut entries) = collect(store, opts)?;
    let mut traj = BufWriter::new(File::create(dir.join("trajectories.jsonl"))?);
    let mut sft = BufWriter::new(File::create(dir.join("sft.jsonl"))?);
    let mut dpo = BufWriter::new(File::create(dir.join("dpo.jsonl"))?);
    let (mut n_sft, mut n_dpo) = (0, 0);
    for t in &trajectories {
        let ex = sft_examples(t, &opts.sft);
        let pairs = preference_pairs(t, opts.sft.history);
        jsonl(&mut traj, std::slice::from_ref(t))?;
        jsonl(&mut sft, &ex)?;
        jsonl(&mut dpo, &pairs)?;
        if let Some(e) = entries.iter_mut().find(|e| e.session_id == t.session_id) {
            e.sft = ex.len();
            e.dpo = pairs.len();
        }
        n_sft += ex.len();
        n_dpo += pairs.len();
    }
    traj.flush()?;
    sft.flush()?;
    dpo.flush()?;
    let manifest = Manifest {
        format: "crosure.dataset.v1".into(),
        files: ["trajectories.jsonl", "sft.jsonl", "dpo.jsonl"]
            .map(String::from)
            .to_vec(),
        options: opts.clone(),
        sessions: entries,
        trajectories: trajectories.len(),
        sft_examples: n_sft,
        dpo_pairs: n_dpo,
    };
    std::fs::write(
        dir.join("manifest.json"),
        serde_json::to_vec_pretty(&manifest)?,
    )?;
    Ok(manifest)
}
