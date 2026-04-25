//! Integration tests for `swe_edge_http_retry` trait re-exports (`api/traits.rs`).
//!
//! `api/traits.rs` defines the `pub(crate)` type alias `HttpRetryTrait`
//! for `dyn HttpRetry`. The relevant integration-level contract is that the
//! SAF re-export surface is complete and `RetryLayer` satisfies all trait
//! bounds required for use inside `reqwest_middleware::ClientBuilder`.

use swe_edge_http_retry::{Builder, RetryConfig, RetryLayer};

fn make_cfg() -> RetryConfig {
    RetryConfig {
        max_retries: 2,
        initial_interval_ms: 50,
        max_interval_ms: 1000,
        multiplier: 2.0,
        retryable_statuses: vec![503],
        retryable_methods: vec!["GET".to_string()],
    }
}

// ---------------------------------------------------------------------------
// reqwest_middleware::Middleware — RetryLayer must implement it
// ---------------------------------------------------------------------------

/// `RetryLayer` must implement `reqwest_middleware::Middleware`. This is the
/// primary consumer contract — without this impl, the layer cannot be
/// attached to a `reqwest_middleware::ClientBuilder`.
#[test]
fn test_retry_layer_implements_reqwest_middleware_trait() {
    fn assert_middleware<T: reqwest_middleware::Middleware>() {}
    assert_middleware::<RetryLayer>();
}

// ---------------------------------------------------------------------------
// Send + Sync — required for Arc<T> + async boundary safety
// ---------------------------------------------------------------------------

/// `RetryLayer` must be `Send + Sync` to be wrapped in `Arc<T>` and shared
/// across async executor tasks, as `reqwest_middleware` requires.
#[test]
fn test_retry_layer_is_arc_send_sync() {
    use std::sync::Arc;
    fn assert_arc_safe<T: Send + Sync + 'static>() {
        let _ = std::mem::size_of::<Arc<T>>();
    }
    assert_arc_safe::<RetryLayer>();
}

// ---------------------------------------------------------------------------
// Full pipeline: layer attaches to reqwest_middleware::ClientBuilder
// ---------------------------------------------------------------------------

/// The full type chain: `RetryConfig` → `Builder` → `RetryLayer` →
/// `reqwest_middleware::ClientBuilder::with` must compile and run without
/// panic. No real HTTP call is made.
#[test]
fn test_retry_layer_attaches_to_reqwest_middleware_client_builder_via_trait_chain() {
    let layer = Builder::with_config(make_cfg()).build().expect("build must succeed");
    let _client = reqwest_middleware::ClientBuilder::new(reqwest::Client::new())
        .with(layer)
        .build();
}

// ---------------------------------------------------------------------------
// RetryLayer as Arc<dyn Middleware>
// ---------------------------------------------------------------------------

/// `RetryLayer` must be usable as `Arc<dyn reqwest_middleware::Middleware>`,
/// the primary polymorphic usage in production middleware chains.
#[test]
fn test_retry_layer_usable_as_arc_dyn_middleware() {
    use std::sync::Arc;
    let layer = Builder::with_config(make_cfg()).build().expect("build");
    let _arc: Arc<dyn reqwest_middleware::Middleware> = Arc::new(layer);
}
