//! Integration tests for `api/breaker_config.rs` — the public `BreakerConfig`
//! struct and its field semantics.

use swe_edge_egress_breaker::{BreakerConfig, Builder};

// ---------------------------------------------------------------------------
// Struct literal construction — all four fields are public
// ---------------------------------------------------------------------------

/// All `BreakerConfig` fields must be publicly constructable via a struct
/// literal.  If a field is renamed, removed, or made `pub(crate)`, this test
/// fails to compile.
#[test]
fn test_breaker_config_all_fields_constructable_and_readable() {
    let cfg = BreakerConfig {
        failure_threshold: 3,
        half_open_after_seconds: 60,
        reset_after_successes: 2,
        failure_statuses: vec![500, 502, 503],
    };
    assert_eq!(cfg.failure_threshold, 3);
    assert_eq!(cfg.half_open_after_seconds, 60);
    assert_eq!(cfg.reset_after_successes, 2);
    assert_eq!(cfg.failure_statuses, vec![500u16, 502, 503]);
}

// ---------------------------------------------------------------------------
// failure_threshold — boundary values
// ---------------------------------------------------------------------------

/// `failure_threshold = 1` means the breaker opens after a single failure.
/// Strict but valid.
#[test]
fn test_breaker_config_threshold_one_builds() {
    let cfg = BreakerConfig {
        failure_threshold: 1,
        half_open_after_seconds: 5,
        reset_after_successes: 1,
        failure_statuses: vec![503],
    };
    Builder::with_config(cfg).build().expect("failure_threshold=1 must build");
}

/// Large failure threshold — tolerant of many failures before tripping.
#[test]
fn test_breaker_config_large_threshold_builds() {
    let cfg = BreakerConfig {
        failure_threshold: u32::MAX,
        half_open_after_seconds: 1,
        reset_after_successes: 1,
        failure_statuses: vec![],
    };
    Builder::with_config(cfg).build().expect("large failure_threshold must build");
}

// ---------------------------------------------------------------------------
// half_open_after_seconds — boundary values
// ---------------------------------------------------------------------------

/// `half_open_after_seconds = 0` means the probe happens immediately after
/// opening.  Must not be rejected.
#[test]
fn test_breaker_config_zero_wait_builds() {
    let cfg = BreakerConfig {
        failure_threshold: 3,
        half_open_after_seconds: 0,
        reset_after_successes: 2,
        failure_statuses: vec![500],
    };
    Builder::with_config(cfg).build().expect("half_open_after_seconds=0 must build");
}

/// Large wait value — slow recovery policy.
#[test]
fn test_breaker_config_large_wait_builds() {
    let cfg = BreakerConfig {
        failure_threshold: 3,
        half_open_after_seconds: 3600,
        reset_after_successes: 2,
        failure_statuses: vec![503],
    };
    Builder::with_config(cfg).build().expect("half_open_after_seconds=3600 must build");
}

// ---------------------------------------------------------------------------
// failure_statuses — boundary values
// ---------------------------------------------------------------------------

/// Empty `failure_statuses` — only network errors trip the breaker.
#[test]
fn test_breaker_config_empty_failure_statuses_builds() {
    let cfg = BreakerConfig {
        failure_threshold: 3,
        half_open_after_seconds: 10,
        reset_after_successes: 2,
        failure_statuses: vec![],
    };
    Builder::with_config(cfg).build().expect("empty failure_statuses must build");
}

/// All 5xx statuses in `failure_statuses` — maximum strictness.
#[test]
fn test_breaker_config_all_5xx_failure_statuses_builds() {
    let all_5xx: Vec<u16> = (500..=599).collect();
    let cfg = BreakerConfig {
        failure_threshold: 5,
        half_open_after_seconds: 30,
        reset_after_successes: 3,
        failure_statuses: all_5xx,
    };
    Builder::with_config(cfg).build().expect("all 5xx failure_statuses must build");
}

// ---------------------------------------------------------------------------
// Config round-trip through Builder
// ---------------------------------------------------------------------------

/// No field must be silently modified between `with_config()` and `config()`.
#[test]
fn test_breaker_config_round_trips_through_builder_unchanged() {
    let cfg = BreakerConfig {
        failure_threshold: 11,
        half_open_after_seconds: 45,
        reset_after_successes: 4,
        failure_statuses: vec![500, 503, 504],
    };
    let b = Builder::with_config(cfg);
    let out = b.config();
    assert_eq!(out.failure_threshold, 11);
    assert_eq!(out.half_open_after_seconds, 45);
    assert_eq!(out.reset_after_successes, 4);
    assert_eq!(out.failure_statuses, vec![500u16, 503, 504]);
}
