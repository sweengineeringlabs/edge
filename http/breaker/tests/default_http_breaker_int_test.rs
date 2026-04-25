//! Integration tests for `core/default_http_breaker.rs`.
//!
//! `DefaultHttpBreaker` is `pub(crate)`.  Its observable effect is through the
//! SAF `builder()` function, which loads the crate-shipped SWE baseline.

use swe_edge_http_breaker::{BreakerConfig, Builder};

// ---------------------------------------------------------------------------
// SWE baseline — verify default config has production-safe values
// ---------------------------------------------------------------------------

/// The `builder()` function must load the baseline without error.
#[test]
fn test_default_http_breaker_swe_default_builder_succeeds() {
    swe_edge_http_breaker::builder().expect("swe_default baseline must parse without error");
}

/// Default `failure_threshold` must be >= 1.  Zero would trip on any request.
#[test]
fn test_default_http_breaker_swe_default_failure_threshold_is_positive() {
    let b = swe_edge_http_breaker::builder().expect("baseline parses");
    assert!(
        b.config().failure_threshold >= 1,
        "swe_default failure_threshold must be >= 1, got {}",
        b.config().failure_threshold
    );
}

/// Default `reset_after_successes` must be >= 1.
#[test]
fn test_default_http_breaker_swe_default_reset_after_successes_is_positive() {
    let b = swe_edge_http_breaker::builder().expect("baseline parses");
    assert!(
        b.config().reset_after_successes >= 1,
        "swe_default reset_after_successes must be >= 1, got {}",
        b.config().reset_after_successes
    );
}

/// Building from the SWE default must produce a valid `BreakerLayer`.
#[test]
fn test_default_http_breaker_swe_default_builds_layer() {
    swe_edge_http_breaker::builder()
        .expect("baseline parses")
        .build()
        .expect("build from swe_default must succeed");
}

// ---------------------------------------------------------------------------
// Custom config is not overridden by the default-loading path
// ---------------------------------------------------------------------------

/// A consumer-supplied config must survive `Builder::with_config` without any
/// field being silently replaced by the SWE default.
#[test]
fn test_default_http_breaker_custom_config_is_not_overridden_by_swe_default() {
    let custom = BreakerConfig {
        failure_threshold: 99,
        half_open_after_seconds: 7,
        reset_after_successes: 11,
        failure_statuses: vec![418],
    };
    let b = Builder::with_config(custom);
    assert_eq!(b.config().failure_threshold, 99);
    assert_eq!(b.config().half_open_after_seconds, 7);
    assert_eq!(b.config().reset_after_successes, 11);
    assert_eq!(b.config().failure_statuses, vec![418u16]);
}
