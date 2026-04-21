//! Per-host breaker state machine.
//!
//! Three states: **Closed** (traffic flows), **Open** (fail
//! fast), **HalfOpen** (a probe is allowed to test recovery).
//! Transitions are driven by outcome observations + time
//! (elapsed since entering Open).

use std::time::{Duration, Instant};

use crate::api::breaker_config::BreakerConfig;

/// Breaker state for a single host. Protected by an async
/// `Mutex` inside [`BreakerLayer`](crate::api::breaker_layer::BreakerLayer)
/// — the mutex serializes state transitions so concurrent
/// requests to the same host see coherent state.
#[derive(Debug)]
pub(crate) struct HostBreaker {
    state: State,
    /// Consecutive failures in Closed state. Trips at
    /// `config.failure_threshold`.
    consecutive_failures: u32,
    /// Consecutive successes in HalfOpen state. Closes the
    /// breaker at `config.reset_after_successes`.
    consecutive_successes: u32,
}

/// Concrete state of a breaker.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum State {
    /// Traffic flows normally. On enough consecutive failures,
    /// transitions to Open.
    Closed,
    /// All requests fail fast. After `half_open_after_seconds`
    /// elapses since this moment, the NEXT request promotes to
    /// HalfOpen.
    Open { since: Instant },
    /// A probe request is in flight. Outcome decides next
    /// state: success → count up; failure → back to Open.
    HalfOpen,
}

/// Outcome of an outbound request, as seen by the breaker.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Outcome {
    Success,
    Failure,
}

/// Decision returned when a new request is admitted through
/// the breaker.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Admission {
    /// Pass through — record the outcome afterward via
    /// [`HostBreaker::record`].
    Proceed,
    /// Breaker is open — fail fast without calling the
    /// upstream.
    RejectOpen,
}

impl HostBreaker {
    pub(crate) fn new() -> Self {
        Self {
            state: State::Closed,
            consecutive_failures: 0,
            consecutive_successes: 0,
        }
    }

    /// Observability hook — expose current state for
    /// diagnostics / tests.
    #[allow(dead_code)]
    pub(crate) fn state(&self) -> State {
        self.state
    }

    /// Called BEFORE dispatching a request. Tells the caller
    /// whether to proceed or fail fast. Mutates the state when
    /// Open → HalfOpen promotion is due.
    pub(crate) fn admit(&mut self, config: &BreakerConfig) -> Admission {
        match self.state {
            State::Closed => Admission::Proceed,
            State::Open { since } => {
                let elapsed = since.elapsed();
                let wait = Duration::from_secs(config.half_open_after_seconds);
                if elapsed >= wait {
                    // Promote: let the next request probe.
                    self.state = State::HalfOpen;
                    self.consecutive_successes = 0;
                    Admission::Proceed
                } else {
                    Admission::RejectOpen
                }
            }
            State::HalfOpen => Admission::Proceed,
        }
    }

    /// Called AFTER dispatching a request that `admit` approved.
    /// Updates state based on outcome.
    pub(crate) fn record(&mut self, config: &BreakerConfig, outcome: Outcome) {
        match (self.state, outcome) {
            (State::Closed, Outcome::Success) => {
                self.consecutive_failures = 0;
            }
            (State::Closed, Outcome::Failure) => {
                self.consecutive_failures =
                    self.consecutive_failures.saturating_add(1);
                if self.consecutive_failures >= config.failure_threshold {
                    self.state = State::Open {
                        since: Instant::now(),
                    };
                }
            }
            (State::HalfOpen, Outcome::Success) => {
                self.consecutive_successes =
                    self.consecutive_successes.saturating_add(1);
                if self.consecutive_successes >= config.reset_after_successes {
                    self.state = State::Closed;
                    self.consecutive_failures = 0;
                    self.consecutive_successes = 0;
                }
            }
            (State::HalfOpen, Outcome::Failure) => {
                self.state = State::Open {
                    since: Instant::now(),
                };
                self.consecutive_successes = 0;
            }
            (State::Open { .. }, _) => {
                // `record` called while Open — caller should
                // not dispatch in this state. Ignore.
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_config() -> BreakerConfig {
        BreakerConfig::from_config(
            r#"
                failure_threshold = 3
                half_open_after_seconds = 1
                reset_after_successes = 2
                failure_statuses = [500, 502, 503, 504]
            "#,
        )
        .unwrap()
    }

    /// @covers: HostBreaker::new
    #[test]
    fn test_new_starts_closed() {
        let b = HostBreaker::new();
        assert_eq!(b.state(), State::Closed);
    }

    /// @covers: HostBreaker::admit
    #[test]
    fn test_closed_admits_traffic() {
        let cfg = test_config();
        let mut b = HostBreaker::new();
        assert_eq!(b.admit(&cfg), Admission::Proceed);
    }

    /// @covers: HostBreaker::record
    #[test]
    fn test_failures_below_threshold_stay_closed() {
        let cfg = test_config();
        let mut b = HostBreaker::new();
        b.record(&cfg, Outcome::Failure);
        b.record(&cfg, Outcome::Failure);
        // 2 failures, threshold is 3 — still Closed.
        assert_eq!(b.state(), State::Closed);
    }

    /// @covers: HostBreaker::record
    #[test]
    fn test_failures_at_threshold_trip_to_open() {
        let cfg = test_config();
        let mut b = HostBreaker::new();
        for _ in 0..3 {
            b.record(&cfg, Outcome::Failure);
        }
        assert!(matches!(b.state(), State::Open { .. }));
    }

    /// @covers: HostBreaker::record
    #[test]
    fn test_success_in_closed_resets_failure_counter() {
        let cfg = test_config();
        let mut b = HostBreaker::new();
        b.record(&cfg, Outcome::Failure);
        b.record(&cfg, Outcome::Failure);
        b.record(&cfg, Outcome::Success); // resets
        b.record(&cfg, Outcome::Failure); // now starts from 0+1
        b.record(&cfg, Outcome::Failure); // 2
        // Still below threshold (3) — Closed.
        assert_eq!(b.state(), State::Closed);
    }

    /// @covers: HostBreaker::admit
    #[test]
    fn test_open_rejects_before_wait_elapsed() {
        let cfg = test_config();
        let mut b = HostBreaker::new();
        for _ in 0..3 {
            b.record(&cfg, Outcome::Failure);
        }
        // Immediately after tripping — still within the wait.
        assert_eq!(b.admit(&cfg), Admission::RejectOpen);
    }

    /// @covers: HostBreaker::admit
    #[test]
    fn test_open_promotes_to_half_open_after_wait() {
        let cfg = test_config(); // 1s wait
        let mut b = HostBreaker::new();
        for _ in 0..3 {
            b.record(&cfg, Outcome::Failure);
        }
        // Fast-forward by replacing the state with an older Instant.
        b.state = State::Open {
            since: Instant::now() - Duration::from_secs(2),
        };
        assert_eq!(b.admit(&cfg), Admission::Proceed);
        assert_eq!(b.state(), State::HalfOpen);
    }

    /// @covers: HostBreaker::record
    #[test]
    fn test_half_open_success_counts_toward_reset() {
        let cfg = test_config();
        let mut b = HostBreaker::new();
        b.state = State::HalfOpen;
        b.record(&cfg, Outcome::Success);
        assert_eq!(b.state(), State::HalfOpen); // need 2, have 1
        b.record(&cfg, Outcome::Success);
        assert_eq!(b.state(), State::Closed);
    }

    /// @covers: HostBreaker::record
    #[test]
    fn test_half_open_failure_returns_to_open() {
        let cfg = test_config();
        let mut b = HostBreaker::new();
        b.state = State::HalfOpen;
        b.consecutive_successes = 1; // partially recovered
        b.record(&cfg, Outcome::Failure);
        assert!(matches!(b.state(), State::Open { .. }));
    }
}
