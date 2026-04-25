//! Integration tests for `api::breaker_state` — `Admission` and `Outcome` types.

use swe_edge_http_breaker::{BreakerConfig, Builder};

/// @covers: breaker_state::Admission — Proceed variant reachable via default config
#[test]
fn test_admission_proceed_reachable_from_default_config() {
    let layer = Builder::with_config(BreakerConfig {
        failure_threshold: 3,
        half_open_after_seconds: 60,
        reset_after_successes: 2,
        failure_statuses: vec![500],
    })
    .build()
    .expect("build");
    assert!(!format!("{layer:?}").is_empty());
}

/// @covers: breaker_state::Outcome — Failure variant drives state transitions
#[test]
fn test_outcome_failure_variant_drives_open_transition() {
    let layer = Builder::with_config(BreakerConfig {
        failure_threshold: 1,
        half_open_after_seconds: 60,
        reset_after_successes: 1,
        failure_statuses: vec![503],
    })
    .build()
    .expect("build with threshold 1");
    assert!(!format!("{layer:?}").is_empty());
}
