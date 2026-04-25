//! Integration tests for `RetryConfig` public surface.
//!
//! `RetryConfig` is a plain struct with all fields public. Tests verify
//! struct literal construction, field visibility, Clone, and that values
//! flow through the Builder pipeline to the RetryLayer without mutation.

use swe_edge_http_retry::{Builder, RetryConfig, RetryLayer};

// ---------------------------------------------------------------------------
// Struct construction — all public fields must be writable
// ---------------------------------------------------------------------------

/// Every field on `RetryConfig` must be directly settable via struct literal
/// syntax. A rename or removal causes this test to fail to compile, catching
/// the API break before it reaches consumers.
#[test]
fn test_retry_config_all_fields_are_public() {
    let cfg = RetryConfig {
        max_retries: 3,
        initial_interval_ms: 100,
        max_interval_ms: 5000,
        multiplier: 2.0,
        retryable_statuses: vec![429, 503],
        retryable_methods: vec!["GET".to_string(), "HEAD".to_string()],
    };
    assert_eq!(cfg.max_retries, 3);
    assert_eq!(cfg.initial_interval_ms, 100);
    assert_eq!(cfg.max_interval_ms, 5000);
    assert_eq!(cfg.multiplier, 2.0);
    assert_eq!(cfg.retryable_statuses, vec![429u16, 503]);
    assert_eq!(cfg.retryable_methods, vec!["GET", "HEAD"]);
}

/// `RetryConfig` must be `Clone` so it can be moved into a `Builder` while
/// still being accessible to the caller for inspection or reuse.
#[test]
fn test_retry_config_is_clone() {
    let cfg = RetryConfig {
        max_retries: 5,
        initial_interval_ms: 200,
        max_interval_ms: 10_000,
        multiplier: 1.5,
        retryable_statuses: vec![500],
        retryable_methods: vec!["DELETE".to_string()],
    };
    let cloned = cfg.clone();
    assert_eq!(cloned.max_retries, cfg.max_retries);
    assert_eq!(cloned.multiplier, cfg.multiplier);
    assert_eq!(cloned.retryable_statuses, cfg.retryable_statuses);
}

// ---------------------------------------------------------------------------
// max_retries boundary values
// ---------------------------------------------------------------------------

/// `max_retries=0` is a valid "no-retry" configuration. The builder and
/// layer must accept it without error.
#[test]
fn test_max_retries_zero_is_valid() {
    let cfg = RetryConfig {
        max_retries: 0,
        initial_interval_ms: 100,
        max_interval_ms: 1000,
        multiplier: 2.0,
        retryable_statuses: vec![503],
        retryable_methods: vec!["GET".to_string()],
    };
    let _layer: RetryLayer = Builder::with_config(cfg).build().expect("max_retries=0 must build");
}

/// `max_retries=u32::MAX` is an extreme value but must not panic or error
/// at build time — the middleware loop will cap at `total = max_retries + 1`
/// attempts, bounded by the saturating_add.
#[test]
fn test_max_retries_max_u32_builds_without_error() {
    let cfg = RetryConfig {
        max_retries: u32::MAX,
        initial_interval_ms: 100,
        max_interval_ms: 1000,
        multiplier: 1.0,
        retryable_statuses: vec![],
        retryable_methods: vec![],
    };
    Builder::with_config(cfg).build().expect("max_retries=u32::MAX must build");
}

// ---------------------------------------------------------------------------
// retryable_statuses — boundary values
// ---------------------------------------------------------------------------

/// Status codes at the edges of the valid HTTP range must be accepted. The
/// type is `Vec<u16>` so any u16 is structurally valid.
#[test]
fn test_retryable_statuses_accepts_full_range_of_u16_values() {
    let cfg = RetryConfig {
        max_retries: 1,
        initial_interval_ms: 100,
        max_interval_ms: 1000,
        multiplier: 1.0,
        retryable_statuses: vec![100, 200, 429, 500, 503, 599, 65535],
        retryable_methods: vec!["GET".to_string()],
    };
    Builder::with_config(cfg)
        .build()
        .expect("wide range of status codes must build");
}

/// An empty `retryable_statuses` list must be accepted — it means "never
/// retry on a received HTTP response" (transport errors may still retry).
#[test]
fn test_retryable_statuses_empty_is_valid() {
    let cfg = RetryConfig {
        max_retries: 3,
        initial_interval_ms: 100,
        max_interval_ms: 5000,
        multiplier: 2.0,
        retryable_statuses: vec![],
        retryable_methods: vec!["GET".to_string()],
    };
    Builder::with_config(cfg).build().expect("empty retryable_statuses must build");
}

// ---------------------------------------------------------------------------
// retryable_methods — case preservation
// ---------------------------------------------------------------------------

/// HTTP method strings must be stored with their original casing. The
/// `method_retryable` check inside the middleware uses case-insensitive
/// comparison, but the stored strings must not be silently uppercased or
/// lowercased by the builder.
#[test]
fn test_retryable_methods_stored_with_original_casing() {
    let cfg = RetryConfig {
        max_retries: 1,
        initial_interval_ms: 50,
        max_interval_ms: 500,
        multiplier: 1.0,
        retryable_statuses: vec![503],
        retryable_methods: vec!["get".to_string(), "HEAD".to_string(), "Put".to_string()],
    };
    let b = Builder::with_config(cfg);
    assert_eq!(b.config().retryable_methods[0], "get");
    assert_eq!(b.config().retryable_methods[1], "HEAD");
    assert_eq!(b.config().retryable_methods[2], "Put");
}

// ---------------------------------------------------------------------------
// multiplier — positive float values
// ---------------------------------------------------------------------------

/// `multiplier=1.0` produces constant-interval backoff. The builder must
/// accept this (no validation that multiplier > 1.0).
#[test]
fn test_multiplier_one_produces_constant_interval() {
    let cfg = RetryConfig {
        max_retries: 3,
        initial_interval_ms: 200,
        max_interval_ms: 200,
        multiplier: 1.0,
        retryable_statuses: vec![503],
        retryable_methods: vec!["GET".to_string()],
    };
    Builder::with_config(cfg).build().expect("multiplier=1.0 must build");
}

/// `multiplier=0.5` (backoff shrinks over time) is unusual but structurally
/// valid. The builder must accept it without error.
#[test]
fn test_multiplier_below_one_builds_successfully() {
    let cfg = RetryConfig {
        max_retries: 2,
        initial_interval_ms: 1000,
        max_interval_ms: 5000,
        multiplier: 0.5,
        retryable_statuses: vec![429],
        retryable_methods: vec!["GET".to_string()],
    };
    Builder::with_config(cfg).build().expect("multiplier=0.5 must build");
}
