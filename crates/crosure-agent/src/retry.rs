//! Retries for temporary provider failures, visible and interruptible.

use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use crate::providers::Provider;
use crate::{AgentConfig, AgentError, AgentEvent, Live, Sink, Transcript, Turn};

/// Longest single wait, whatever the server asks for.
const MAX_WAIT: Duration = Duration::from_secs(60);
/// How often a wait checks whether the analyst pressed Stop.
const SLICE: Duration = Duration::from_millis(100);

/// `base * 2^attempt`, or the server's `retry-after`, capped, with ±10% jitter.
fn delay(cfg: &AgentConfig, attempt: u32, e: &AgentError) -> Duration {
    let base = match e.retry_after() {
        Some(secs) => Duration::from_secs(secs),
        None => Duration::from_millis(cfg.retry_base_ms.saturating_mul(1 << attempt.min(16))),
    }
    .min(MAX_WAIT);
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.subsec_nanos());
    // 0.9 ..= 1.1 of the base, from the clock: enough to spread clients out.
    let factor = 0.9 + f64::from(nanos % 1000) / 5000.0;
    base.mul_f64(factor)
}

/// Sleeps for `d` in short slices; returns false if Stop was pressed.
fn wait(sink: &dyn Sink, d: Duration) -> bool {
    let end = Instant::now() + d;
    while Instant::now() < end {
        if sink.should_stop() {
            return false;
        }
        std::thread::sleep(SLICE.min(end.saturating_duration_since(Instant::now())));
    }
    !sink.should_stop()
}

/// Asks `provider` for the next turn, retrying temporary failures up to
/// `cfg.max_retries` times. Each wait is announced with
/// [`AgentEvent::Retrying`]. Returns `Ok(None)` if the analyst stopped
/// during a wait or while the reply was streaming.
pub(crate) fn next_turn(
    provider: &dyn Provider,
    t: &Transcript,
    sink: &dyn Sink,
    cfg: &AgentConfig,
) -> Result<Option<Turn>, AgentError> {
    let mut attempt = 0;
    loop {
        let reply = provider.next_live(t, sink);
        // The finished turn arrives as events; the live view is cleared
        // also when the attempt failed and will be retried.
        sink.live(&Live::default());
        match reply {
            Ok(turn) => return Ok(Some(turn)),
            Err(AgentError::Stopped) => return Ok(None),
            Err(e) if e.is_transient() && attempt < cfg.max_retries => {
                let d = delay(cfg, attempt, &e);
                attempt += 1;
                sink.emit(AgentEvent::Retrying {
                    attempt,
                    delay_ms: d.as_millis().try_into().unwrap_or(u64::MAX),
                    error: format!("{}: {e}", provider.id()),
                });
                if !wait(sink, d) {
                    return Ok(None);
                }
            }
            Err(e) => return Err(e),
        }
    }
}
