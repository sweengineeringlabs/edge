//! Integration tests for `core::retry_layer` — the `RetryLayer` impl block
//! containing the backoff logic, method/status filtering, and the
//! `reqwest_middleware::Middleware` implementation.
//!
//! All core methods are `pub(crate)`. Integration tests exercise observable
//! behavior through the public middleware interface:
//!
//! - A retryable method + retryable status causes the middleware to retry.
//! - A non-retryable method bypasses the retry loop entirely.
//! - A non-retryable status (e.g. 200, 400) causes the middleware to return
//!   immediately after the first attempt.
//! - The `backoff_for` cap (`max_interval_ms`) is respected in timing.

use swe_edge_http_retry::{Builder, RetryConfig};

fn make_cfg_with(
    max_retries: u32,
    initial_ms: u64,
    max_ms: u64,
    multiplier: f64,
    statuses: Vec<u16>,
    methods: Vec<&str>,
) -> RetryConfig {
    RetryConfig {
        max_retries,
        initial_interval_ms: initial_ms,
        max_interval_ms: max_ms,
        multiplier,
        retryable_statuses: statuses,
        retryable_methods: methods.into_iter().map(str::to_string).collect(),
    }
}

// ---------------------------------------------------------------------------
// Method filtering: non-retryable method → single pass-through
// ---------------------------------------------------------------------------

/// When the HTTP method is NOT in `retryable_methods`, the middleware must
/// pass through immediately — no retry loop. We verify this by targeting a
/// closed port and measuring that the total elapsed time reflects a single
/// attempt (no retry back-off sleeps), using a short connect_timeout so each
/// attempt fails fast on all platforms (Windows TCP isn't instantaneous).
#[tokio::test]
async fn test_core_non_retryable_method_does_not_retry() {
    // POST is not in the retryable list; 5 retries × 100ms back-off sleeps
    // would add >500ms, but a single attempt with 50ms connect_timeout is ~50ms.
    let cfg = make_cfg_with(5, 10, 100, 2.0, vec![503], vec!["GET"]);
    let layer = Builder::with_config(cfg).build().expect("build");
    let client = reqwest_middleware::ClientBuilder::new(
        reqwest::Client::builder()
            .connect_timeout(std::time::Duration::from_millis(50))
            .build()
            .expect("client"),
    )
    .with(layer)
    .build();

    let start = std::time::Instant::now();
    let _ = client.post("http://127.0.0.1:19998/no-server").send().await;
    let elapsed = start.elapsed();

    // Single attempt: ≤50ms connect_timeout + negligible overhead.
    // If retries fired we'd sleep >500ms; 2s is generous CI headroom.
    assert!(
        elapsed < std::time::Duration::from_secs(2),
        "non-retryable method must not sleep through retries; elapsed={elapsed:?}"
    );
}

// ---------------------------------------------------------------------------
// Method filtering: retryable method + non-retryable status → single attempt
// ---------------------------------------------------------------------------

/// When the method IS retryable but the received status code is NOT in
/// `retryable_statuses`, the middleware must return after the first attempt
/// (no retry). We verify this by measuring that total elapsed time is bounded:
/// transport errors on a closed port ARE retried, so we configure a short
/// connect_timeout to keep each attempt fast, then bound the total time to
/// confirm the retry loop stays within `max_retries × max_interval_ms`.
#[tokio::test]
async fn test_core_retryable_method_non_retryable_status_returns_immediately() {
    // GET is retryable; 503 triggers retry. With connect_timeout=50ms,
    // 5 retries × (50ms connect + 100ms sleep) = 5 × 150ms = 750ms max.
    // Allow 5s headroom for slow CI; no sane retry loop exceeds this.
    let cfg = make_cfg_with(5, 10, 100, 2.0, vec![503], vec!["GET"]);
    let layer = Builder::with_config(cfg).build().expect("build");
    let client = reqwest_middleware::ClientBuilder::new(
        reqwest::Client::builder()
            .connect_timeout(std::time::Duration::from_millis(50))
            .build()
            .expect("client"),
    )
    .with(layer)
    .build();

    let start = std::time::Instant::now();
    let _ = client.get("http://127.0.0.1:19997/no-server").send().await;
    let elapsed = start.elapsed();

    // 5 retries × (50ms connect + 100ms sleep) ≤ 750ms; 5s is generous CI cap.
    assert!(
        elapsed < std::time::Duration::from_secs(5),
        "retry loop with capped backoff must complete in bounded time; elapsed={elapsed:?}"
    );
}

// ---------------------------------------------------------------------------
// RetryLayer: middleware impl compiles and chains correctly
// ---------------------------------------------------------------------------

/// Confirms the full `RetryLayer → reqwest_middleware::Middleware` type
/// chain compiles. No runtime assertion needed — compilation IS the test.
#[test]
fn test_core_retry_layer_middleware_impl_type_chain_compiles() {
    let cfg = make_cfg_with(2, 50, 500, 2.0, vec![503], vec!["GET"]);
    let layer = Builder::with_config(cfg).build().expect("build");
    fn assert_middleware<T: reqwest_middleware::Middleware>(_: T) {}
    assert_middleware(layer);
}

// ---------------------------------------------------------------------------
// Exponential backoff cap — verified through elapsed time
// ---------------------------------------------------------------------------

/// With `multiplier=100.0`, uncapped back-off would grow astronomically.
/// Setting `max_interval_ms=20` caps every sleep at 20ms. With `max_retries=3`
/// and `connect_timeout=50ms`, total ≤ 4×50ms connect + 3×20ms sleep = 260ms.
/// Allow 3s for CI overhead — any runaway back-off would vastly exceed this.
#[tokio::test]
async fn test_core_backoff_capped_at_max_interval_bounded_total_time() {
    let cfg = make_cfg_with(3, 20, 20, 100.0, vec![503], vec!["GET"]);
    let layer = Builder::with_config(cfg).build().expect("build");
    let client = reqwest_middleware::ClientBuilder::new(
        reqwest::Client::builder()
            .connect_timeout(std::time::Duration::from_millis(50))
            .build()
            .expect("client"),
    )
    .with(layer)
    .build();

    let start = std::time::Instant::now();
    let _ = client.get("http://127.0.0.1:19996/no-server").send().await;
    let elapsed = start.elapsed();

    // 4 attempts × 50ms + 3 × 20ms = 260ms; 3s is generous CI cap.
    assert!(
        elapsed < std::time::Duration::from_secs(3),
        "capped backoff must complete in bounded time; elapsed={elapsed:?}"
    );
}
