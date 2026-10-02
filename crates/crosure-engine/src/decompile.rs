//! Decompilation through [rizin](https://rizin.re) and its
//! [rz-ghidra](https://github.com/rizinorg/rz-ghidra) plugin, an optional
//! external tool. Everything else in the engine works without it.
//!
//! The executable is `$CROSURE_RIZIN` if set, else `rizin` on `PATH`.

use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::OnceLock;
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::EngineError;

/// How to get a decompiler, shown when none is available.
pub const DECOMPILER_HINT: &str = "Install rizin (https://rizin.re) and the rz-ghidra plugin \
(https://github.com/rizinorg/rz-ghidra), or set CROSURE_RIZIN to the rizin executable.";

/// One line of pseudo-C, with the lowest address it was generated from.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DecompLine {
    pub text: String,
    pub addr: Option<u64>,
}

/// A decompiled function.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Decompiled {
    /// e.g. `rz-ghidra (rizin 0.8.2)`.
    pub backend: String,
    pub lines: Vec<DecompLine>,
}

/// Whether decompilation is available, and with what.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DecompilerStatus {
    pub available: bool,
    /// `rizin 0.8.2`, when rizin was found.
    pub rizin: Option<String>,
    pub ghidra: bool,
    /// What to install when something is missing.
    pub hint: Option<String>,
}

fn exe() -> PathBuf {
    std::env::var_os("CROSURE_RIZIN")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("rizin"))
}

/// Runs `cmd`, killing it after `timeout`. Returns stdout.
fn run(mut cmd: Command, timeout: Duration) -> Result<String, EngineError> {
    let mut child = cmd
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| EngineError::Decompiler(format!("could not start rizin: {e}")))?;
    let mut out = child
        .stdout
        .take()
        .ok_or_else(|| EngineError::Decompiler("no output pipe".into()))?;
    let reader = std::thread::spawn(move || {
        let mut s = String::new();
        let _ = out.read_to_string(&mut s);
        s
    });
    let deadline = Instant::now() + timeout;
    loop {
        if child.try_wait()?.is_some() {
            break;
        }
        if Instant::now() > deadline {
            let _ = child.kill();
            let _ = child.wait();
            return Err(EngineError::Decompiler(format!(
                "rizin did not finish within {}s",
                timeout.as_secs()
            )));
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    reader
        .join()
        .map_err(|_| EngineError::Decompiler("output reader failed".into()))
}

fn probe() -> DecompilerStatus {
    let short = Duration::from_secs(10);
    let mut v = Command::new(exe());
    v.arg("-v");
    let rizin = run(v, short).ok().and_then(|s| {
        s.lines()
            .next()
            .and_then(|l| l.split(" @ ").next())
            .map(str::to_string)
    });
    let ghidra = rizin.is_some() && {
        let mut l = Command::new(exe());
        l.args(["-q", "-c", "Lc", "--"]);
        run(l, short).is_ok_and(|s| s.to_lowercase().contains("ghidra"))
    };
    let hint = (!ghidra).then(|| DECOMPILER_HINT.to_string());
    DecompilerStatus {
        available: ghidra,
        rizin,
        ghidra,
        hint,
    }
}

/// Finds rizin and rz-ghidra (checked once per process).
pub fn decompiler_status() -> DecompilerStatus {
    static STATUS: OnceLock<DecompilerStatus> = OnceLock::new();
    STATUS.get_or_init(probe).clone()
}

/// Splits rz-ghidra's `pdgj` output into lines, each with the lowest
/// address any of its tokens came from.
///
/// ```
/// let j = serde_json::json!({
///     "code": "\nint f(void)\n{\n    return 1;\n}\n",
///     "annotations": [{ "start": 21, "end": 30, "type": "offset", "offset": 4096 }]
/// });
/// let lines = crosure_engine::parse_pdgj(&j)?;
/// assert_eq!(lines.len(), 4);
/// assert_eq!(lines[2].text, "    return 1;");
/// assert_eq!(lines[2].addr, Some(4096));
/// assert_eq!(lines[0].addr, None);
/// # Ok::<(), crosure_engine::EngineError>(())
/// ```
pub fn parse_pdgj(j: &Value) -> Result<Vec<DecompLine>, EngineError> {
    let code = j["code"]
        .as_str()
        .ok_or_else(|| EngineError::Decompiler("decompiler returned no code".into()))?;
    let mut starts = Vec::new();
    let mut pos = 0usize;
    for line in code.split('\n') {
        starts.push((pos, line));
        pos += line.len() + 1;
    }
    if code.ends_with('\n') {
        starts.pop();
    }
    let mut lines: Vec<DecompLine> = starts
        .iter()
        .map(|(_, t)| DecompLine {
            text: t.trim_end().to_string(),
            addr: None,
        })
        .collect();
    for a in j["annotations"].as_array().into_iter().flatten() {
        if a["type"] != "offset" {
            continue;
        }
        let (Some(start), Some(off)) = (a["start"].as_u64(), a["offset"].as_u64()) else {
            continue;
        };
        let idx = starts.partition_point(|(s, _)| *s as u64 <= start);
        if let Some(l) = idx.checked_sub(1).and_then(|i| lines.get_mut(i)) {
            l.addr = Some(l.addr.map_or(off, |a| a.min(off)));
        }
    }
    let lead = lines.iter().take_while(|l| l.text.is_empty()).count();
    lines.drain(..lead);
    Ok(lines)
}

/// Decompiles the function at `addr` in `binary` with rz-ghidra.
///
/// Only that function is analysed, so this stays fast on large binaries.
pub fn decompile(binary: &Path, addr: u64) -> Result<Decompiled, EngineError> {
    let status = decompiler_status();
    if !status.available {
        return Err(EngineError::Decompiler(format!(
            "no decompiler available. {DECOMPILER_HINT}"
        )));
    }
    let mut cmd = Command::new(exe());
    cmd.args([
        "-2",
        "-q",
        "-e",
        "scr.color=0",
        "-c",
        &format!("af @ {addr:#x}; pdgj @ {addr:#x}"),
    ])
    .arg(binary);
    let out = run(cmd, Duration::from_secs(60))?;
    let json = out
        .lines()
        .find(|l| l.trim_start().starts_with('{'))
        .ok_or_else(|| EngineError::Decompiler(format!("nothing to decompile at {addr:#x}")))?;
    let j: Value = serde_json::from_str(json)
        .map_err(|e| EngineError::Decompiler(format!("unreadable decompiler output: {e}")))?;
    Ok(Decompiled {
        backend: format!(
            "rz-ghidra ({})",
            status.rizin.unwrap_or_else(|| "rizin".into())
        ),
        lines: parse_pdgj(&j)?,
    })
}
