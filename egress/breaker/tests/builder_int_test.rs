//! Integration tests for `api/builder.rs` and `saf/builder.rs`.
//!
//! Covers the full public builder surface: the `builder()` free function and
//! the `Builder` type's `with_config`, `config`, and `build` methods.

use swe_edge_egress_breaker::{BreakerConfig, BreakerLayer, Builder, Error};

// ---------------------------------------------------------------------------
// builder() free function
// ---------------------------------------------------------------------------

/// The `builder()` free function must return `Ok` — the crate-shipped TOML
/// baseline must always parse.  A failure here means the embedded default
/// config is broken.
#[test]
fn test_builder_fn_succeeds_with_swe_default() {
    swe_edge_egress_breaker::builder()
        .expect("builder() must succeed with the crate-shipped baseline");
}

/// The default `failure_threshold` must be >= 1.  A threshold of 0 would
/// open the breaker on every single request, making it permanently open.
#[test]
fn test_builder_fn_swe_default_failure_threshold_is_positive() {
    let b = swe_edge_egress_breaker::builder().expect("baseline parses");
    assert!(
        b.config().failure_threshold >= 1,
        "swe_default failure_threshold must be >= 1, got {}",
        b.config().failure_threshold
    );
}

/// The default `reset_after_successes` must be >= 1.  Zero successes to close
/// would mean the breaker immediately closes on a half-open probe attempt,
/// defeating its purpose.
#[test]
fn test_builder_fn_swe_default_reset_after_successes_is_positive() {
    let b = swe_edge_egress_breaker::builder().expect("baseline parses");
    assert!(
        b.config().reset_after_successes >= 1,
        "swe_default reset_after_successes must be >= 1, got {}",
        b.config().reset_after_successes
    );
}

// ---------------------------------------------------------------------------
// Builder::with_config — custom policy round-trips correctly
// ---------------------------------------------------------------------------

/// `with_config` must preserve every field of the supplied `BreakerConfig`
/// without silent modification.
#[test]
fn test_with_config_preserves_all_fields() {
    let cfg = BreakerConfig {
        failure_threshold: 5,
        half_open_after_seconds: 30,
        reset_after_successes: 3,
        failure_statuses: vec![500, 502, 503],
    };
    let b = Builder::with_config(cfg);
    let policy = b.config();
    assert_eq!(policy.failure_threshold, 5);
    assert_eq!(policy.half_open_after_seconds, 30);
    assert_eq!(policy.reset_after_successes, 3);
    assert_eq!(policy.failure_statuses, vec![500u16, 502, 503]);
}

/// `config()` must return a borrow of the stored policy, not an owned copy
/// that could diverge from what `build()` consumes.
#[test]
fn test_config_accessor_returns_reference_not_divergent_copy() {
    let cfg = BreakerConfig {
        failure_threshold: 7,
        half_open_after_seconds: 10,
        reset_after_successes: 2,
        failure_statuses: vec![503],
    };
    let b = Builder::with_config(cfg);
    let policy: &BreakerConfig = b.config();
    assert_eq!(policy.failure_threshold, 7);
    assert_eq!(policy.reset_after_successes, 2);
}

// ---------------------------------------------------------------------------
// Builder::build — produces a usable BreakerLayer
// ---------------------------------------------------------------------------

/// The nominal build path must succeed and return a `BreakerLayer`.
#[test]
fn test_build_from_swe_default_returns_breaker_layer() {
    let layer: BreakerLayer = swe_edge_egress_breaker::builder()
        .expect("baseline parses")
        .build()
        .expect("build() must succeed");
    let dbg = format!("{layer:?}");
    assert!(
        dbg.contains("BreakerLayer"),
        "Debug output must identify the type; got: {dbg}"
    );
}

/// Building from a custom config must succeed.
#[test]
fn test_build_with_custom_config_succeeds() {
    let cfg = BreakerConfig {
        failure_threshold: 3,
        half_open_after_seconds: 60,
        reset_after_successes: 2,
        failure_statuses: vec![500, 503],
    };
    Builder::with_config(cfg).build().expect("custom config must build");
}

/// An empty `failure_statuses` list is a valid policy (no HTTP status triggers
/// a failure — only network errors do).
#[test]
fn test_build_with_empty_failure_statuses_succeeds() {
    let cfg = BreakerConfig {
        failure_threshold: 3,
        half_open_after_seconds: 60,
        reset_after_successes: 2,
        failure_statuses: vec![],
    };
    Builder::with_config(cfg)
        .build()
        .expect("empty failure_statuses must not be rejected");
}

/// A high `failure_threshold` (aggressive tolerance) is a legitimate
/// configuration and must not be rejected at build time.
#[test]
fn test_build_with_high_failure_threshold_succeeds() {
    let cfg = BreakerConfig {
        failure_threshold: 1000,
        half_open_after_seconds: 5,
        reset_after_successes: 1,
        failure_statuses: vec![500],
    };
    Builder::with_config(cfg)
        .build()
        .expect("failure_threshold=1000 must not be rejected");
}

/// A single-success reset policy is legitimate — probe once and close.
#[test]
fn test_build_with_single_success_reset_policy_succeeds() {
    let cfg = BreakerConfig {
        failure_threshold: 5,
        half_open_after_seconds: 30,
        reset_after_successes: 1,
        failure_statuses: vec![503],
    };
    Builder::with_config(cfg)
        .build()
        .expect("reset_after_successes=1 must not be rejected");
}

// ---------------------------------------------------------------------------
// Error variants — public constructability
// ---------------------------------------------------------------------------

/// `Error::ParseFailed` must display the crate name.
#[test]
fn test_error_parse_failed_display_names_crate() {
    let err = Error::ParseFailed("bad toml".to_string());
    let msg = err.to_string();
    assert!(
        msg.contains("swe_edge_egress_breaker"),
        "ParseFailed display must name the crate; got: {msg}"
    );
}

/// `Error::ParseFailed` display must echo the supplied reason.
#[test]
fn test_error_parse_failed_display_echoes_reason() {
    let reason = "missing field `failure_threshold`";
    let err = Error::ParseFailed(reason.to_string());
    assert!(err.to_string().contains(reason));
}

/// `Error::NotImplemented` must produce a non-empty display.
#[test]
fn test_error_not_implemented_display_is_non_empty_and_names_crate() {
    let err = Error::NotImplemented("some feature");
    let msg = err.to_string();
    assert!(!msg.is_empty());
    assert!(
        msg.contains("swe_edge_egress_breaker"),
        "NotImplemented display must name the crate; got: {msg}"
    );
}
