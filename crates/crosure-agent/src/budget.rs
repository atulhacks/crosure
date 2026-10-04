//! How much prompt a request may carry, learned from what the server reports.
//!
//! The limit is the run's own budget (a bound on cost), lowered to the
//! model's window minus its output limit when that is known. Sizes are
//! estimated from characters, then corrected by the ratio between the
//! server's reported prompt tokens and the estimate: disassembly and hex
//! take more tokens per character than prose.
//!
//! Some local servers (Ollama's OpenAI endpoint) silently drop the start of
//! a prompt that does not fit their context, instead of returning an error.
//! A reported prompt far smaller than what was sent reveals it: the analyst
//! is warned once, and later requests are trimmed to what the server kept.

use crate::context::estimate_tokens;
use crate::providers::Provider;
use crate::{AgentEvent, Sink, Transcript};

/// Below this, estimates are too noisy to compare with the server.
const MIN_COMPARE: usize = 4_000;
/// Never trim below this: the system prompt and tools alone take about this much.
const FLOOR: usize = 2_000;
/// Output reserved when the model's output limit is unknown.
const DEFAULT_OUTPUT: u64 = 8_192;

/// Context budget for one run.
pub(crate) struct Budget {
    cap: usize,
    /// Tokens per estimated token, from the last comparable reply.
    ratio: f64,
    /// The prompt size a truncating server kept.
    learned: Option<usize>,
    warned: bool,
}

impl Budget {
    /// A budget bounded by `cap` tokens.
    pub(crate) fn new(cap: usize) -> Self {
        Self {
            cap,
            ratio: 1.0,
            learned: None,
            warned: false,
        }
    }

    /// Prompt tokens a request to `p` may carry.
    pub(crate) fn limit(&self, p: &dyn Provider) -> usize {
        let l = p.limits();
        let mut limit = self.cap;
        if let Some(w) = l.context_window {
            let out = l.max_output.unwrap_or(DEFAULT_OUTPUT.min(w / 4));
            // 5% margin: estimates are approximate.
            let fit = w.saturating_sub(out).saturating_sub(w / 20);
            limit = limit.min(usize::try_from(fit).unwrap_or(usize::MAX));
        }
        if let Some(kept) = self.learned {
            limit = limit.min(kept);
        }
        limit.max(FLOOR)
    }

    /// The estimated size of a request for `t`, corrected by what the
    /// server has reported so far.
    pub(crate) fn estimate(&self, t: &Transcript) -> usize {
        (estimate_tokens(t) as f64 * self.ratio) as usize
    }

    /// Compares the server's reported prompt size with the raw estimate of
    /// what was sent (`sent`, from [`estimate_tokens`]).
    pub(crate) fn observe(&mut self, sent: usize, reported: u64, sink: &dyn Sink) {
        let reported = usize::try_from(reported).unwrap_or(usize::MAX);
        if reported == 0 || sent < MIN_COMPARE {
            return;
        }
        if reported * 2 < sent {
            // Character estimates undercount; half of them cannot be right.
            self.learned = Some(reported * 9 / 10);
            if !self.warned {
                self.warned = true;
                sink.emit(AgentEvent::PromptTruncated {
                    estimated: sent,
                    reported,
                });
            }
            return;
        }
        self.ratio = (reported as f64 / sent as f64).clamp(0.8, 3.0);
    }
}

#[cfg(test)]
mod tests {
    use super::Budget;
    use crate::providers::{ModelLimits, Provider};
    use crate::{AgentError, AgentEvent, Sink, Transcript, Turn};
    use std::sync::Mutex;

    struct P(ModelLimits);
    impl Provider for P {
        fn id(&self) -> &str {
            "p"
        }
        fn model(&self) -> &str {
            "m"
        }
        fn limits(&self) -> ModelLimits {
            self.0
        }
        fn next(&self, _: &Transcript) -> Result<Turn, AgentError> {
            Err(AgentError::Protocol("unused".into()))
        }
    }

    #[derive(Default)]
    struct Events(Mutex<Vec<AgentEvent>>);
    impl Sink for Events {
        fn emit(&self, e: AgentEvent) {
            if let Ok(mut v) = self.0.lock() {
                v.push(e);
            }
        }
        fn should_stop(&self) -> bool {
            false
        }
    }

    #[test]
    fn the_model_window_lowers_but_never_raises_the_cap() {
        let b = Budget::new(100_000);
        let small = P(ModelLimits {
            context_window: Some(32_768),
            max_output: Some(4_096),
        });
        assert_eq!(b.limit(&small), 32_768 - 4_096 - 1_638);
        let huge = P(ModelLimits {
            context_window: Some(1_000_000),
            max_output: None,
        });
        assert_eq!(b.limit(&huge), 100_000);
        assert_eq!(b.limit(&P(ModelLimits::default())), 100_000);
    }

    #[test]
    fn reported_sizes_calibrate_and_truncation_is_learned_once() {
        let sink = Events::default();
        let mut b = Budget::new(100_000);
        let p = P(ModelLimits::default());
        b.observe(10_000, 15_000, &sink);
        assert!((b.ratio - 1.5).abs() < 1e-9);
        // A server that kept only 4096 of ~20000 tokens.
        b.observe(20_000, 4_096, &sink);
        b.observe(20_000, 4_096, &sink);
        assert_eq!(b.limit(&p), 3_686);
        let warnings = sink
            .0
            .lock()
            .map(|v| {
                v.iter()
                    .filter(|e| matches!(e, AgentEvent::PromptTruncated { .. }))
                    .count()
            })
            .unwrap_or(0);
        assert_eq!(warnings, 1);
    }
}
