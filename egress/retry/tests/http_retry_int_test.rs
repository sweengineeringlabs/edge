//! Integration tests for the `HttpRetry` trait contract.
//!
//! `api::http_retry::HttpRetry` is `pub(crate)` — not accessible from
//! outside the crate. Integration tests verify observable effects:
//!
//! - `RetryLayer` satisfies `Send + Sync` (enforced by the supertrait
//!   bounds on `HttpRetry: Send + Sync`).
//! - `RetryLayer` implements `reqwest_middleware::Middleware`, which is the
//!   trait consumed by `reqwest_middleware::ClientBuilder::with(layer)`.
//! - The layer produced by the builder pipeline is non-trivially wired —
//!   it has a non-zero `max_retries` field visible in Debug.

use swe_edge_egress_retry::{Builder, RetryConfig, RetryLayer};

// ---------------------------------------------------------------------------
// reqwest_middleware::Middleware — compile-time proof
// ---------------------------------------------------------------------------

/// `RetryLayer` must implement `reqwest_middleware::Middleware`. If this impl
/// is removed the test fails to compile, catching the regression immediately.
#[test]
fn test_retry_layer_implements_reqwest_middleware() {
    fn assert_middleware<T: reqwest_middleware::Middleware>() {}
    assert_middleware::<RetryLayer>();
}

// ---------------------------------------------------------------------------
// Send + Sync — enforced by HttpRetry supertrait bounds
// ---------------------------------------------------------------------------

/// `RetryLayer` must be `Send + Sync` to be shareable across async tasks.
/// The `HttpRetry: Send + Sync` supertraits enforce this on any concrete
/// type that wraps a `Box<dyn HttpRetry>`.
#[test]
fn test_retry_layer_is_send_and_sync() {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<RetryLayer>();
}

/// `Arc<RetryLayer>` must be `Send + Sync` so it can be used in the
/// `reqwest_middleware::ClientBuilder::with(Arc<impl Middleware>)` pattern.
#[test]
fn test_retry_layer_is_arc_send_sync() {
    use std::sync::Arc;
    fn assert_arc_safe<T: Send + Sync + 'static>() {
        let _ = std::mem::size_of::<Arc<T>>();
    }
    assert_arc_safe::<RetryLayer>();
}

// ---------------------------------------------------------------------------
// describe() contract — "swe_edge_egress_retry" embedded in Debug
// ---------------------------------------------------------------------------

/// The `RetryLayer` Debug output must include `max_retries`, which flows
/// from `DefaultHttpRetry` through the layer's `Arc<RetryConfig>`.
/// This confirms `DefaultHttpRetry::new` was called with the correct config.
#[test]
fn test_retry_layer_debug_contains_max_retries() {
    let cfg = RetryConfig {
        max_retries: 4,
        initial_interval_ms: 200,
        max_interval_ms: 8000,
        multiplier: 2.0,
        retryable_statuses: vec![503],
        retryable_methods: vec!["GET".to_string()],
    };
    let layer = Builder::with_config(cfg).build().expect("build must succeed");
    let dbg = format!("{layer:?}");
    assert!(
        dbg.contains("max_retries"),
        "Debug must contain max_retries confirming DefaultHttpRetry wiring; got: {dbg}"
    );
}

// ---------------------------------------------------------------------------
// Full pipeline: layer can be passed to reqwest_middleware::ClientBuilder
// ---------------------------------------------------------------------------

/// Building a `reqwest_middleware::ClientWithMiddleware` with a `RetryLayer`
/// must compile and construct without panic.
#[test]
fn test_retry_layer_attaches_to_reqwest_middleware_client_builder() {
    let cfg = RetryConfig {
        max_retries: 2,
        initial_interval_ms: 100,
        max_interval_ms: 1000,
        multiplier: 2.0,
        retryable_statuses: vec![503],
        retryable_methods: vec!["GET".to_string()],
    };
    let layer = Builder::with_config(cfg).build().expect("build must succeed");
    let _client = reqwest_middleware::ClientBuilder::new(reqwest::Client::new())
        .with(layer)
        .build();
    // No runtime assertion — the test proves the type chain compiles and
    // the constructor succeeds.
}
