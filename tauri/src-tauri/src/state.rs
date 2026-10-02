use std::path::PathBuf;
use std::sync::{Mutex, MutexGuard};

use crosure_recorder::Store;
use crosure_session::Workspace;

/// App-wide state. Lock order is always `store` then `workspace`.
pub struct AppState {
    pub store: Mutex<Store>,
    pub workspace: Mutex<Option<Workspace>>,
    pub home: PathBuf,
}

/// `$CROSURE_HOME`, or `~/.crosure`.
pub fn crosure_home() -> PathBuf {
    std::env::var_os("CROSURE_HOME")
        .map(PathBuf::from)
        .or_else(|| dirs::home_dir().map(|h| h.join(".crosure")))
        .unwrap_or_else(|| PathBuf::from(".crosure"))
}

impl AppState {
    /// Opens (or creates) the store under `home`.
    pub fn open(home: PathBuf) -> Result<Self, String> {
        std::fs::create_dir_all(&home).map_err(|e| e.to_string())?;
        let store = Store::open(&home.join("crosure.db")).map_err(|e| e.to_string())?;
        Ok(Self {
            store: Mutex::new(store),
            workspace: Mutex::new(None),
            home,
        })
    }

    /// Locks the store and the (required) open workspace together.
    pub fn both(
        &self,
    ) -> Result<(MutexGuard<'_, Store>, MutexGuard<'_, Option<Workspace>>), String> {
        let store = self
            .store
            .lock()
            .map_err(|_| "store lock poisoned".to_string())?;
        let ws = self
            .workspace
            .lock()
            .map_err(|_| "workspace lock poisoned".to_string())?;
        if ws.is_none() {
            return Err("no binary is open".into());
        }
        Ok((store, ws))
    }
}
