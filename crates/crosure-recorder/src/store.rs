use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};

use crate::chain::{genesis_hash, hex_digest, step_hash};
use crate::schema::{FUNC_FP_KEY, MIGRATIONS, SCHEMA_SQL, SCHEMA_VERSION};
use crate::{NewStep, RecorderError, Relation, Step};

/// One investigation of one binary.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Session {
    pub id: String,
    pub name: String,
    pub binary_path: String,
    pub binary_sha256: String,
    pub created_ms: i64,
    /// Hash of the latest step (or the genesis hash when empty).
    pub head_hash: String,
}

/// Append-only step store backed by SQLite.
pub struct Store {
    conn: Connection,
}

pub(crate) fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

impl Store {
    /// Opens (or creates) a store at `path`.
    pub fn open(path: &Path) -> Result<Self, RecorderError> {
        Self::init(Connection::open(path)?)
    }

    /// An in-memory store, for tests and previews.
    ///
    /// ```
    /// let store = crosure_recorder::Store::open_in_memory()?;
    /// assert!(store.sessions()?.is_empty());
    /// # Ok::<(), crosure_recorder::RecorderError>(())
    /// ```
    pub fn open_in_memory() -> Result<Self, RecorderError> {
        Self::init(Connection::open_in_memory()?)
    }

    /// Creates the tables, then brings an older store up to the current
    /// format. A store from a newer Crosure is refused, not misread.
    fn init(mut conn: Connection) -> Result<Self, RecorderError> {
        let found: i64 = conn.query_row("PRAGMA user_version", [], |r| r.get(0))?;
        if found > SCHEMA_VERSION {
            return Err(RecorderError::NewerFormat {
                found,
                supported: SCHEMA_VERSION,
            });
        }
        conn.execute_batch(SCHEMA_SQL)?;
        for version in found.max(1)..SCHEMA_VERSION {
            let tx = conn.transaction()?;
            if let Some(sql) = usize::try_from(version - 1)
                .ok()
                .and_then(|i| MIGRATIONS.get(i))
            {
                tx.execute_batch(sql)?;
            }
            tx.pragma_update(None, "user_version", version + 1)?;
            tx.commit()?;
        }
        conn.pragma_update(None, "user_version", SCHEMA_VERSION)?;
        Ok(Self { conn })
    }

    /// The store format version (`PRAGMA user_version`).
    ///
    /// ```
    /// let store = crosure_recorder::Store::open_in_memory()?;
    /// assert_eq!(store.format_version()?, 2);
    /// # Ok::<(), crosure_recorder::RecorderError>(())
    /// ```
    pub fn format_version(&self) -> Result<i64, RecorderError> {
        Ok(self
            .conn
            .query_row("PRAGMA user_version", [], |r| r.get(0))?)
    }

    /// Starts a new session for a binary.
    pub fn create_session(
        &self,
        name: &str,
        binary_path: &str,
        binary_sha256: &str,
    ) -> Result<Session, RecorderError> {
        let id = format!("ses_{}", ulid::Ulid::new());
        let session = Session {
            head_hash: genesis_hash(&id),
            id,
            name: name.into(),
            binary_path: binary_path.into(),
            binary_sha256: binary_sha256.into(),
            created_ms: now_ms(),
        };
        self.conn.execute(
            "INSERT INTO sessions (id, name, binary_path, binary_sha256, created_ms, head_hash)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                session.id,
                session.name,
                session.binary_path,
                session.binary_sha256,
                session.created_ms,
                session.head_hash
            ],
        )?;
        Ok(session)
    }

    /// Looks up a session by id.
    pub fn session(&self, id: &str) -> Result<Session, RecorderError> {
        self.conn
            .query_row(
                "SELECT id, name, binary_path, binary_sha256, created_ms, head_hash
                 FROM sessions WHERE id = ?1",
                params![id],
                row_to_session,
            )
            .optional()?
            .ok_or_else(|| RecorderError::UnknownSession(id.into()))
    }

    /// All sessions, newest first.
    pub fn sessions(&self) -> Result<Vec<Session>, RecorderError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, binary_path, binary_sha256, created_ms, head_hash
             FROM sessions ORDER BY created_ms DESC, id DESC",
        )?;
        let rows = stmt.query_map([], row_to_session)?;
        Ok(rows.collect::<Result<_, _>>()?)
    }

    /// Appends a step to the session's chain and returns the stored step.
    ///
    /// When the caller gives no `next`/`branch` parent, a `next` edge to the
    /// previous step is added automatically.
    pub fn record(&self, session_id: &str, new: NewStep) -> Result<Step, RecorderError> {
        let session = self.session(session_id)?;
        let last = self.last_step(session_id)?;
        let mut parents = new.parents;
        for p in &parents {
            if !self.step_exists(session_id, &p.id)? {
                return Err(RecorderError::UnknownParent(p.id.clone()));
            }
        }
        let has_order = parents
            .iter()
            .any(|p| matches!(p.rel, Relation::Next | Relation::Branch));
        if let (false, Some(prev)) = (has_order, &last) {
            parents.insert(
                0,
                crate::ParentRef {
                    id: prev.id.clone(),
                    rel: Relation::Next,
                },
            );
        }
        let mut step = Step {
            id: format!("stp_{}", ulid::Ulid::new()),
            session_id: session_id.into(),
            seq: last.as_ref().map_or(0, |s| s.seq + 1),
            ts_ms: now_ms(),
            parents,
            actor: new.actor,
            kind: new.kind,
            tool: new.tool,
            command: new.command,
            action: new.action,
            target: new.target,
            observation: new.observation,
            intent: new.intent,
            tags: new.tags,
            attention: new.attention,
            prev_hash: session.head_hash,
            hash: String::new(),
        };
        step.hash = step_hash(&step)?;
        let kind = serde_json::to_value(step.kind)?;
        self.conn.execute(
            "INSERT INTO steps (session_id, seq, id, kind, json, hash) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                step.session_id,
                step.seq as i64,
                step.id,
                kind.as_str().unwrap_or_default(),
                serde_json::to_string(&step)?,
                step.hash
            ],
        )?;
        self.conn.execute(
            "UPDATE sessions SET head_hash = ?1 WHERE id = ?2",
            params![step.hash, session_id],
        )?;
        Ok(step)
    }

    /// All steps of a session in chain order.
    pub fn steps(&self, session_id: &str) -> Result<Vec<Step>, RecorderError> {
        self.session(session_id)?;
        let mut stmt = self
            .conn
            .prepare("SELECT json FROM steps WHERE session_id = ?1 ORDER BY seq")?;
        let rows = stmt.query_map(params![session_id], |r| r.get::<_, String>(0))?;
        let mut out = Vec::new();
        for json in rows {
            out.push(serde_json::from_str(&json?)?);
        }
        Ok(out)
    }

    /// Steps, in any session, whose target function has a fingerprint
    /// starting with `key` (the 22-char structural key, `fid1:<hash>/`).
    pub fn steps_with_fingerprint(&self, key: &str) -> Result<Vec<Step>, RecorderError> {
        let sql =
            format!("SELECT json FROM steps WHERE {FUNC_FP_KEY} = ?1 ORDER BY session_id, seq");
        let mut stmt = self.conn.prepare(&sql)?;
        let rows = stmt.query_map(params![key], |r| r.get::<_, String>(0))?;
        let mut out = Vec::new();
        for json in rows {
            out.push(serde_json::from_str(&json?)?);
        }
        Ok(out)
    }

    fn last_step(&self, session_id: &str) -> Result<Option<Step>, RecorderError> {
        let json: Option<String> = self
            .conn
            .query_row(
                "SELECT json FROM steps WHERE session_id = ?1 ORDER BY seq DESC LIMIT 1",
                params![session_id],
                |r| r.get(0),
            )
            .optional()?;
        Ok(match json {
            Some(j) => Some(serde_json::from_str(&j)?),
            None => None,
        })
    }

    fn step_exists(&self, session_id: &str, id: &str) -> Result<bool, RecorderError> {
        let n: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM steps WHERE session_id = ?1 AND id = ?2",
            params![session_id, id],
            |r| r.get(0),
        )?;
        Ok(n > 0)
    }

    /// Stores bytes by content and returns their `sha256:` key.
    ///
    /// ```
    /// let store = crosure_recorder::Store::open_in_memory()?;
    /// let key = store.put_blob(b"hello")?;
    /// assert_eq!(store.get_blob(&key)?.as_deref(), Some(&b"hello"[..]));
    /// # Ok::<(), crosure_recorder::RecorderError>(())
    /// ```
    pub fn put_blob(&self, data: &[u8]) -> Result<String, RecorderError> {
        let key = hex_digest(data);
        self.conn.execute(
            "INSERT OR IGNORE INTO blobs (key, data) VALUES (?1, ?2)",
            params![key, data],
        )?;
        Ok(key)
    }

    /// Fetches bytes stored with [`Store::put_blob`].
    pub fn get_blob(&self, key: &str) -> Result<Option<Vec<u8>>, RecorderError> {
        Ok(self
            .conn
            .query_row("SELECT data FROM blobs WHERE key = ?1", params![key], |r| {
                r.get(0)
            })
            .optional()?)
    }

    /// Raw connection, for verification and tamper tests inside this crate.
    pub(crate) fn conn(&self) -> &Connection {
        &self.conn
    }
}

fn row_to_session(r: &rusqlite::Row<'_>) -> rusqlite::Result<Session> {
    Ok(Session {
        id: r.get(0)?,
        name: r.get(1)?,
        binary_path: r.get(2)?,
        binary_sha256: r.get(3)?,
        created_ms: r.get(4)?,
        head_hash: r.get(5)?,
    })
}
