//! Integration tests for `swe_http_retry` `Builder` and `builder()` SAF entry point.
//!
//! Covers: `builder()`, `Builder::with_config`, `Builder::config`, `Builder::build`.

use swe_http_retry::{builder, Builder, RetryConfig, RetryLayer};

fn make_cfg(max_retries: u32) -> RetryConfig {
    RetryConfig {
        max_retries,
        initial_interval_ms: 100,
        max_interval_ms: 5000,
        multiplier: 2.0,
        retryable_statuses: vec![429, 500, 502, 503],
        retryable_methods: vec!["GET".to_string(), "HEAD".to_string()],
    }
}

// ---------------------------------------------------------------------------
// builder() — SAF entry point
// ---------------------------------------------------------------------------

/// The crate-shipped `config/application.toml` must always parse cleanly.
/// If that file is corrupted or missing, this is the first test to break.
#[test]
fn test_builder_fn_returns_ok_with_swe_default() {
    builder().expect("builder() must succeed with crate baseline");
}

/// A baseline of zero max_retries would mean the middleware is a no-op
/// by default, making the crate useless without explicit configuration.
#[test]
fn test_builder_fn_swe_default_has_at_least_one_retry() {
    let b = builder().expect("baseline parses");
    assert!(
        b.config().max_retries >= 1,
        "swe_default max_retries must be >= 1; got {}",
        b.config().max_retries
    );
}

/// An empty retryable_statuses list in the default config means no HTTP
/// response will ever trigger a retry — rendering the middleware inert.
#[test]
fn test_builder_fn_swe_default_has_non_empty_retryable_statuses() {
    let b = builder().expect("baseline parses");
    assert!(
        !b.config().retryable_statuses.is_empty(),
        "swe_default retryable_statuses must not be empty"
    );
}

/// The default initial_interval_ms must be positive — a zero delay would
/// cause the middleware to retry without any backoff.
#[test]
fn test_builder_fn_swe_default_initial_interval_is_positive() {
    let b = builder().expect("baseline parses");
    assert!(
        b.config().initial_interval_ms > 0,
        "swe_default initial_interval_ms must be > 0; got {}",
        b.config().initial_interval_ms
    );
}

// ---------------------------------------------------------------------------
// Builder::with_config — custom config flows through unchanged
// ---------------------------------------------------------------------------

/// All fields supplied through `with_config` must be readable via `config()`
/// without modification before `build` is called.
#[test]
fn test_with_config_stores_all_fields_unchanged() {
    let cfg = make_cfg(5);
    let b = Builder::with_config(cfg);
    assert_eq!(b.config().max_retries, 5);
    assert_eq!(b.config().initial_interval_ms, 100);
    assert_eq!(b.config().max_interval_ms, 5000);
    assert_eq!(b.config().multiplier, 2.0);
    assert_eq!(b.config().retryable_statuses, vec![429u16, 500, 502, 503]);
    assert_eq!(b.config().retryable_methods, vec!["GET", "HEAD"]);
}

/// `config()` must return a reference to the same policy that `build` will
/// embed in the layer — not a detached copy.
#[test]
fn test_config_accessor_returns_stored_reference() {
    let cfg = make_cfg(3);
    let b = Builder::with_config(cfg);
    let policy: &RetryConfig = b.config();
    assert_eq!(policy.max_retries, 3);
    assert_eq!(policy.multiplier, 2.0);
}

// ---------------------------------------------------------------------------
// Builder::build — produces a RetryLayer
// ---------------------------------------------------------------------------

/// Happy path: `build` must succeed and return a `RetryLayer` whose Debug
/// output names the type and exposes `max_retries` for operator visibility.
#[test]
fn test_build_with_custom_config_returns_retry_layer() {
    let layer: RetryLayer = Builder::with_config(make_cfg(3))
        .build()
        .expect("build must succeed");
    let dbg = format!("{layer:?}");
    assert!(dbg.contains("RetryLayer"), "Debug must name the type; got: {dbg}");
    assert!(dbg.contains("max_retries"), "Debug must expose max_retries; got: {dbg}");
}

/// Building via the `builder()` entry point then `build()` must produce a
/// layer with the same `max_retries` visible in Debug as the config reports.
#[test]
fn test_build_from_swe_default_debug_contains_max_retries() {
    let layer = builder().expect("baseline parses").build().expect("build ok");
    let dbg = format!("{layer:?}");
    assert!(dbg.contains("max_retries"), "Debug must expose max_retries; got: {dbg}");
}

/// `max_retries=0` is valid — it means "pass-through, never retry". The
/// builder must accept this and produce a layer without error.
#[test]
fn test_build_with_zero_max_retries_succeeds() {
    let cfg = RetryConfig {
        max_retries: 0,
        initial_interval_ms: 100,
        max_interval_ms: 100,
        multiplier: 1.0,
        retryable_statuses: vec![],
        retryable_methods: vec![],
    };
    Builder::with_config(cfg).build().expect("max_retries=0 must build");
}

/// Empty `retryable_statuses` and `retryable_methods` are valid — the
/// middleware simply never triggers a retry. Must build without error.
#[test]
fn test_build_with_empty_retryable_lists_succeeds() {
    let cfg = RetryConfig {
        max_retries: 5,
        initial_interval_ms: 50,
        max_interval_ms: 2000,
        multiplier: 1.5,
        retryable_statuses: vec![],
        retryable_methods: vec![],
    };
    Builder::with_config(cfg).build().expect("empty retryable lists must build");
}

/// A very large multiplier (e.g. 100×) is a valid operator choice for
/// aggressive backoff during incident response. The builder must accept it.
#[test]
fn test_build_with_large_multiplier_succeeds() {
    let cfg = RetryConfig {
        max_retries: 2,
        initial_interval_ms: 10,
        max_interval_ms: 60_000,
        multiplier: 100.0,
        retryable_statuses: vec![503],
        retryable_methods: vec!["POST".to_string()],
    };
    Builder::with_config(cfg).build().expect("multiplier=100.0 must build");
}

// ---------------------------------------------------------------------------
// RetryLayer: Send + Sync — compile-time proof
// ---------------------------------------------------------------------------

/// `RetryLayer` must be `Send + Sync` so it can be shared across async task
/// boundaries inside a `reqwest_middleware` chain.
#[test]
fn test_retry_layer_is_send_and_sync() {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<RetryLayer>();
}
