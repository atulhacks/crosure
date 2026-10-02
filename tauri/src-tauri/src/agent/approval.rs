//! A tool call waiting for the analyst: the agent thread blocks here until
//! the UI answers (or the run is stopped, which denies).

use std::sync::{Condvar, Mutex};
use std::time::Duration;

/// One pending decision at a time (the agent runs calls in order).
#[derive(Default)]
pub struct Gate {
    slot: Mutex<Option<(String, Option<bool>)>>,
    changed: Condvar,
}

impl Gate {
    /// Waits for a decision on `id`; `stopped` is polled so stop never hangs.
    pub fn wait(&self, id: &str, stopped: impl Fn() -> bool) -> bool {
        let Ok(mut slot) = self.slot.lock() else {
            return false;
        };
        *slot = Some((id.to_string(), None));
        loop {
            if let Some((_, Some(decision))) = slot.as_ref() {
                let d = *decision;
                *slot = None;
                return d;
            }
            if stopped() {
                *slot = None;
                return false;
            }
            match self.changed.wait_timeout(slot, Duration::from_millis(250)) {
                Ok((s, _)) => slot = s,
                Err(_) => return false,
            }
        }
    }

    /// Records the analyst's answer for `id`. Returns false if nothing is waiting on it.
    pub fn decide(&self, id: &str, allow: bool) -> bool {
        let Ok(mut slot) = self.slot.lock() else {
            return false;
        };
        match slot.as_mut() {
            Some((pending, decision)) if pending == id => {
                *decision = Some(allow);
                self.changed.notify_all();
                true
            }
            _ => false,
        }
    }
}
