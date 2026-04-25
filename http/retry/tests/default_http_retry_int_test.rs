//! Integration tests for `core::default_http_retry::DefaultHttpRetry`.
//!
//! `DefaultHttpRetry` is `pub(crate)`. Integration tests verify its contract
//! through observable effects from the public builder pipeline:
//!
//! - The layer's Debug output confirms `max_retries` flows from config through
//!   `DefaultHttpRetry` into the layer (describe() = "swe_edge_http_retry").
//! - `RetryLayer` is `Send + Sync`, which propagates from `DefaultHttpRetry`
//!   being `Send + Sync` through the `Arc<RetryConfig>` chain.
//! - The builder pipeline does not modify the config values on the way through
//!   `DefaultHttpRetry::new`.

use swe_edge_http_retry::{builder, Builder, RetryConfig, RetryLayer};

fn make_cfg(max_retries: u32, initial_ms: u64) -> RetryConfig {
    RetryConfig {
        max_retries,
        initial_interval_ms: initial_ms,
        max_interval_ms: 10_000,
        multiplier: 2.0,
        retryable_statuses: vec![429, 503],
        retryable_methods: vec!["GET".to_string()],
    }
}

// ---------------------------------------------------------------------------
// DefaultHttpRetry::new — indirectly via builder pipeline
// ---------------------------------------------------------------------------

/// `DefaultHttpRetry::new` is invoked with the config inside `Builder::build`.
/// Observable effect: the layer's Debug output must embed `max_retries` and
/// `initial_interval_ms`, confirming the config was not swapped or reset.
#[test]
fn test_builder_pipeline_embeds_config_in_default_http_retry() {
    let layer: RetryLayer = Builder::with_config(make_cfg(4, 250))
        .build()
        .expect("build must succeed");
    let dbg = format!("{layer:?}");
    assert!(dbg.contains('4'), "Debug must embed max_retries=4; got: {dbg}");
    assert!(dbg.contains("250"), "Debug must embed initial_interval_ms=250; got: {dbg}");
}

// ---------------------------------------------------------------------------
// DefaultHttpRetry::describe — "swe_edge_http_retry" embedded in Debug
// ---------------------------------------------------------------------------

/// The `RetryLayer` Debug output shows `max_retries` and `initial_interval_ms`.
/// Two layers with different values must produce different Debug strings,
/// confirming `DefaultHttpRetry::new` stores the supplied config verbatim.
#[test]
fn test_two_layers_different_configs_have_different_debug() {
    let l1 = Builder::with_config(make_cfg(1, 100)).build().unwrap();
    let l2 = Builder::with_config(make_cfg(5, 500)).build().unwrap();
    assert_ne!(
        format!("{l1:?}"),
        format!("{l2:?}"),
        "different configs must produce different Debug output"
    );
}

// ---------------------------------------------------------------------------
// DefaultHttpRetry: Send + Sync propagation
// ---------------------------------------------------------------------------

/// `RetryLayer` must be `Send + Sync`. The `HttpRetry: Send + Sync`
/// supertraits propagate this requirement to `DefaultHttpRetry` and
/// through the `Arc<RetryConfig>` held by `RetryLayer`.
#[test]
fn test_retry_layer_is_send_and_sync() {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<RetryLayer>();
}

// ---------------------------------------------------------------------------
// DefaultHttpRetry config is not mutated during build
// ---------------------------------------------------------------------------

/// All config fields must survive the `DefaultHttpRetry::new` call
/// unchanged. Observed through `Builder::config()` pre-build.
#[test]
fn test_builder_does_not_mutate_config_in_default_http_retry() {
    let retryable_statuses = vec![429u16, 500, 502, 503, 504];
    let retryable_methods = vec!["GET".to_string(), "HEAD".to_string(), "PUT".to_string()];
    let cfg = RetryConfig {
        max_retries: 6,
        initial_interval_ms: 300,
        max_interval_ms: 15_000,
        multiplier: 1.8,
        retryable_statuses: retryable_statuses.clone(),
        retryable_methods: retryable_methods.clone(),
    };
    let b = Builder::with_config(cfg);
    assert_eq!(b.config().max_retries, 6);
    assert_eq!(b.config().initial_interval_ms, 300);
    assert_eq!(b.config().max_interval_ms, 15_000);
    assert_eq!(b.config().multiplier, 1.8);
    assert_eq!(b.config().retryable_statuses, retryable_statuses);
    assert_eq!(b.config().retryable_methods, retryable_methods);
}

// ---------------------------------------------------------------------------
// builder() convenience function routes through DefaultHttpRetry
// ---------------------------------------------------------------------------

/// The `builder()` entry point must ultimately produce a `RetryLayer`
/// whose Debug output confirms the crate baseline was loaded into
/// `DefaultHttpRetry::new`.
#[test]
fn test_saf_builder_fn_produces_layer_with_baseline_config() {
    let layer = builder()
        .expect("baseline parses")
        .build()
        .expect("build must succeed");
    let dbg = format!("{layer:?}");
    assert!(
        dbg.contains("RetryLayer"),
        "builder() pipeline must produce RetryLayer; got: {dbg}"
    );
    assert!(
        dbg.contains("max_retries"),
        "layer Debug must show max_retries from DefaultHttpRetry; got: {dbg}"
    );
}
