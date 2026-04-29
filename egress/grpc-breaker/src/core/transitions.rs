//! Pure state-transition logic.
//!
//! No I/O, no clock except `Instant::now()` at the moment of
//! state change.  The decorator in `core::breaker_client`
//! takes the lock, calls these helpers, then drops the lock —
//! keeping the critical section tight.

use std::time::Instant;

use tracing::{debug, info, warn};

use crate::api::breaker_client::BreakerNode;
use crate::api::breaker_config::GrpcBreakerConfig;
use crate::api::breaker_state::{Admission, BreakerState, Outcome};

/// Decide whether to admit a new request.  May promote
/// Open → HalfOpen if the cool-down has elapsed.
pub(crate) fn admit(
    node:   &mut BreakerNode,
    config: &GrpcBreakerConfig,
) -> Admission {
    match node.state {
        BreakerState::Closed   => Admission::Proceed,
        BreakerState::HalfOpen => Admission::Proceed,
        BreakerState::Open { since } => {
            if since.elapsed() >= config.cool_down() {
                debug!(
                    cool_down_seconds = config.cool_down_seconds,
                    "grpc-breaker: cool-down elapsed, promoting to HalfOpen",
                );
                node.state = BreakerState::HalfOpen;
                node.consecutive_successes = 0;
                Admission::Proceed
            } else {
                Admission::RejectOpen
            }
        }
    }
}

/// Record the outcome of a dispatched request and update state.
///
/// Called only when [`admit`] returned [`Admission::Proceed`] —
/// i.e. we actually called the inner client.
pub(crate) fn record(
    node:    &mut BreakerNode,
    config:  &GrpcBreakerConfig,
    outcome: Outcome,
) {
    match (node.state, outcome) {
        (BreakerState::Closed, Outcome::Success) => {
            node.consecutive_failures = 0;
        }
        (BreakerState::Closed, Outcome::Failure) => {
            node.consecutive_failures = node.consecutive_failures.saturating_add(1);
            if node.consecutive_failures >= config.failure_threshold {
                warn!(
                    failures = node.consecutive_failures,
                    threshold = config.failure_threshold,
                    "grpc-breaker: failure threshold reached, opening",
                );
                node.state = BreakerState::Open { since: Instant::now() };
            }
        }
        (BreakerState::HalfOpen, Outcome::Success) => {
            node.consecutive_successes = node.consecutive_successes.saturating_add(1);
            if node.consecutive_successes >= config.half_open_probe_count {
                info!(
                    probe_successes = node.consecutive_successes,
                    "grpc-breaker: probe successful, closing",
                );
                node.state = BreakerState::Closed;
                node.consecutive_failures  = 0;
                node.consecutive_successes = 0;
            }
        }
        (BreakerState::HalfOpen, Outcome::Failure) => {
            warn!("grpc-breaker: probe failed, returning to Open");
            node.state = BreakerState::Open { since: Instant::now() };
            node.consecutive_successes = 0;
        }
        (BreakerState::Open { .. }, _) => {
            // record() called while Open — caller should not
            // dispatch in this state.  Ignore defensively.
        }
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;

    fn cfg() -> GrpcBreakerConfig {
        GrpcBreakerConfig::from_config(
            r#"
                failure_threshold = 3
                cool_down_seconds = 1
                half_open_probe_count = 2
            "#,
        )
        .unwrap()
    }

    /// @covers: admit — Closed admits.
    #[test]
    fn test_admit_closed_proceeds() {
        let mut n = BreakerNode::new();
        assert_eq!(admit(&mut n, &cfg()), Admission::Proceed);
    }

    /// @covers: record — failure under threshold stays Closed.
    #[test]
    fn test_record_failure_below_threshold_stays_closed() {
        let c = cfg();
        let mut n = BreakerNode::new();
        record(&mut n, &c, Outcome::Failure);
        record(&mut n, &c, Outcome::Failure);
        assert_eq!(n.state, BreakerState::Closed);
    }

    /// @covers: record — N consecutive failures trip Open.
    #[test]
    fn test_record_failures_at_threshold_open() {
        let c = cfg();
        let mut n = BreakerNode::new();
        for _ in 0..3 {
            record(&mut n, &c, Outcome::Failure);
        }
        assert!(matches!(n.state, BreakerState::Open { .. }));
    }

    /// @covers: record — success resets failure counter.
    #[test]
    fn test_record_success_in_closed_resets_counter() {
        let c = cfg();
        let mut n = BreakerNode::new();
        record(&mut n, &c, Outcome::Failure);
        record(&mut n, &c, Outcome::Failure);
        record(&mut n, &c, Outcome::Success);
        record(&mut n, &c, Outcome::Failure);
        record(&mut n, &c, Outcome::Failure);
        assert_eq!(n.state, BreakerState::Closed);
    }

    /// @covers: admit — Open within cool-down rejects.
    #[test]
    fn test_admit_open_within_cool_down_rejects() {
        let c = cfg();
        let mut n = BreakerNode::new();
        for _ in 0..3 {
            record(&mut n, &c, Outcome::Failure);
        }
        assert_eq!(admit(&mut n, &c), Admission::RejectOpen);
    }

    /// @covers: admit — Open after cool-down promotes to HalfOpen.
    #[test]
    fn test_admit_open_after_cool_down_promotes_half_open() {
        let c = cfg();
        let mut n = BreakerNode::new();
        n.state = BreakerState::Open {
            since: Instant::now() - Duration::from_secs(2),
        };
        assert_eq!(admit(&mut n, &c), Admission::Proceed);
        assert_eq!(n.state, BreakerState::HalfOpen);
    }

    /// @covers: record — HalfOpen success below probe count stays HalfOpen.
    #[test]
    fn test_record_half_open_partial_success_stays_half_open() {
        let c = cfg(); // half_open_probe_count = 2
        let mut n = BreakerNode::new();
        n.state = BreakerState::HalfOpen;
        record(&mut n, &c, Outcome::Success);
        assert_eq!(n.state, BreakerState::HalfOpen);
    }

    /// @covers: record — HalfOpen probes complete → Closed.
    #[test]
    fn test_record_half_open_full_success_closes() {
        let c = cfg();
        let mut n = BreakerNode::new();
        n.state = BreakerState::HalfOpen;
        record(&mut n, &c, Outcome::Success);
        record(&mut n, &c, Outcome::Success);
        assert_eq!(n.state, BreakerState::Closed);
        assert_eq!(n.consecutive_failures, 0);
        assert_eq!(n.consecutive_successes, 0);
    }

    /// @covers: record — HalfOpen failure → Open.
    #[test]
    fn test_record_half_open_failure_reopens() {
        let c = cfg();
        let mut n = BreakerNode::new();
        n.state = BreakerState::HalfOpen;
        n.consecutive_successes = 1;
        record(&mut n, &c, Outcome::Failure);
        assert!(matches!(n.state, BreakerState::Open { .. }));
        assert_eq!(n.consecutive_successes, 0);
    }

    /// @covers: record — Open is a no-op (defensive).
    #[test]
    fn test_record_in_open_state_is_noop() {
        let c = cfg();
        let mut n = BreakerNode::new();
        let since = Instant::now();
        n.state = BreakerState::Open { since };
        record(&mut n, &c, Outcome::Failure);
        assert!(matches!(n.state, BreakerState::Open { .. }));
        record(&mut n, &c, Outcome::Success);
        assert!(matches!(n.state, BreakerState::Open { .. }));
    }
}
