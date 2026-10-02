use rusqlite::params;
use serde::{Deserialize, Serialize};

use crate::chain::{genesis_hash, step_hash};
use crate::{RecorderError, Step, Store};

/// Where and why a chain failed verification.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct VerifyFailure {
    pub seq: u64,
    pub step_id: Option<String>,
    pub reason: String,
}

/// Result of [`Store::verify`].
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct VerifyReport {
    pub ok: bool,
    /// Number of steps checked before stopping.
    pub checked: u64,
    /// Hash of the last valid step (the session's root of trust).
    pub head_hash: String,
    pub failure: Option<VerifyFailure>,
}

impl Store {
    /// Re-derives every hash in the session's chain from the stored data.
    ///
    /// Any edit to a stored step (its JSON, its order, or its hash) makes
    /// verification fail at that step. Exporting `head_hash` somewhere else
    /// (a report, a ticket, a signature) anchors the whole chain.
    pub fn verify(&self, session_id: &str) -> Result<VerifyReport, RecorderError> {
        let session = self.session(session_id)?;
        let mut stmt = self
            .conn()
            .prepare("SELECT seq, json, hash FROM steps WHERE session_id = ?1 ORDER BY seq")?;
        let rows = stmt.query_map(params![session_id], |r| {
            Ok((
                r.get::<_, i64>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
            ))
        })?;
        let mut expected_prev = genesis_hash(session_id);
        let mut checked = 0u64;
        for row in rows {
            let (seq, json, stored_hash) = row?;
            let fail = |id: Option<String>, reason: &str| VerifyReport {
                ok: false,
                checked,
                head_hash: expected_prev.clone(),
                failure: Some(VerifyFailure {
                    seq: seq as u64,
                    step_id: id,
                    reason: reason.into(),
                }),
            };
            let step: Step = match serde_json::from_str(&json) {
                Ok(s) => s,
                Err(_) => return Ok(fail(None, "step data is not valid JSON")),
            };
            let id = Some(step.id.clone());
            if step.seq != checked || seq as u64 != checked {
                return Ok(fail(id, "sequence gap or reordering"));
            }
            if step.session_id != session_id {
                return Ok(fail(id, "step belongs to another session"));
            }
            if step.prev_hash != expected_prev {
                return Ok(fail(id, "prev_hash does not link to the previous step"));
            }
            let recomputed = step_hash(&step)?;
            if recomputed != step.hash || recomputed != stored_hash {
                return Ok(fail(id, "content was modified after recording"));
            }
            expected_prev = recomputed;
            checked += 1;
        }
        if session.head_hash != expected_prev {
            return Ok(VerifyReport {
                ok: false,
                checked,
                head_hash: expected_prev,
                failure: Some(VerifyFailure {
                    seq: checked,
                    step_id: None,
                    reason: "session head does not match the last step (steps removed?)".into(),
                }),
            });
        }
        Ok(VerifyReport {
            ok: true,
            checked,
            head_hash: expected_prev,
            failure: None,
        })
    }
}
