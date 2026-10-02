//! `crosure-mcp <binary>`: serve one binary to an MCP client over stdio.
//!
//! Register it with your client, e.g. Claude Code:
//! `claude mcp add crosure -- crosure-mcp ./sample`
//! The session is stored in `~/.crosure` (or `$CROSURE_HOME`) and opens in
//! the Crosure app afterwards with every step on the graph.

use std::io::{BufRead, Write};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use crosure_agent::SessionExecutor;
use crosure_mcp::Server;
use crosure_recorder::Store;
use crosure_session::Workspace;

fn home() -> PathBuf {
    std::env::var_os("CROSURE_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".crosure")))
        .unwrap_or_else(|| PathBuf::from(".crosure"))
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let Some(binary) = std::env::args().nth(1) else {
        eprintln!("usage: crosure-mcp <binary>");
        std::process::exit(2);
    };
    let dir = home();
    std::fs::create_dir_all(&dir)?;
    let store = Store::open(&dir.join("crosure.db"))?;
    let (ws, _) = Workspace::open(&store, std::path::Path::new(&binary), None)?;
    ws.record_task(&store, "external agent session (MCP)")?;
    eprintln!(
        "crosure-mcp: recording session {} for {binary}",
        ws.session.id
    );
    let exec = SessionExecutor {
        store: Arc::new(Mutex::new(store)),
        workspace: Arc::new(Mutex::new(Some(ws))),
    };
    let mut server = Server::new(exec);
    let stdin = std::io::stdin();
    let mut out = std::io::stdout().lock();
    for line in stdin.lock().lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        let reply = match serde_json::from_str(&line) {
            Ok(msg) => server.handle(&msg),
            Err(e) => Some(
                serde_json::json!({ "jsonrpc": "2.0", "id": null, "error": { "code": -32700, "message": e.to_string() } }),
            ),
        };
        if let Some(r) = reply {
            writeln!(out, "{r}")?;
            out.flush()?;
        }
    }
    Ok(())
}
