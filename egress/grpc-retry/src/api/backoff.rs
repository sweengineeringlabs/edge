//! Interface contract for the backoff scheduler.
//!
//! The implementation lives in `core::backoff`.  This file
//! holds the public type that `core::backoff` produces — a
//! [`BackoffSchedule`] descriptor — so the layer-boundary check
//! can find an api/ counterpart for every core/ submodule.

use std::time::Duration;

/// Outcome of one call to the backoff scheduler.
///
/// Construct via the helpers in `core::backoff` — consumers
/// outside the crate don't need to build these directly; the
/// retry loop drives the schedule internally.  The type is
/// public for tests and observability tools that want to
/// inspect the computed delay.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BackoffSchedule {
    /// How long to sleep before the next retry.
    pub sleep: Duration,
}

impl BackoffSchedule {
    /// Wrap a [`Duration`] as a [`BackoffSchedule`].
    pub fn from_duration(sleep: Duration) -> Self {
        Self { sleep }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// @covers: from_duration
    #[test]
    fn test_from_duration_wraps_sleep() {
        let s = BackoffSchedule::from_duration(Duration::from_millis(150));
        assert_eq!(s.sleep, Duration::from_millis(150));
    }
}
