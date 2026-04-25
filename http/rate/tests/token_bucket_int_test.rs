//! Integration tests for `core/token_bucket.rs`.
//!
//! `TokenBucket` is `pub(crate)`.  From an integration test we cannot
//! instantiate it directly.  Instead, we verify the behaviours it enables
//! through the public layer surface:
//! - Layers configured with specific burst/rate combinations must build.
//! - The `Debug` output of the `RateLayer` must reflect the policy that the
//!   token bucket will be initialised from.
//! - Edge-case capacity values (minimum, large, burst == rate) must all
//!   be accepted by the builder.

use swe_http_rate::{Builder, RateConfig, RateLayer};

// ---------------------------------------------------------------------------
// burst_capacity == tokens_per_second — no burst beyond steady rate
// ---------------------------------------------------------------------------

/// When `burst_capacity == tokens_per_second`, the bucket starts full at
/// exactly one second's worth of tokens.  This is a valid (tight) policy.
#[test]
fn test_token_bucket_burst_equals_rate_layer_builds() {
    let cfg = RateConfig {
        tokens_per_second: 10,
        burst_capacity: 10, // same as rate — no burst allowance
        per_host: false,
    };
    Builder::with_config(cfg)
        .build()
        .expect("burst_capacity == tokens_per_second must build");
}

// ---------------------------------------------------------------------------
// burst_capacity > tokens_per_second — standard burst policy
// ---------------------------------------------------------------------------

/// A burst capacity that is a multiple of the rate is the most common
/// production pattern (e.g. 10 RPS with a 5-second burst allowance = 50).
#[test]
fn test_token_bucket_burst_multiple_of_rate_builds() {
    let cfg = RateConfig {
        tokens_per_second: 10,
        burst_capacity: 50,
        per_host: true,
    };
    Builder::with_config(cfg)
        .build()
        .expect("burst_capacity=5x rate must build");
}

// ---------------------------------------------------------------------------
// Minimum viable config — rate=1, burst=1
// ---------------------------------------------------------------------------

/// The smallest possible config (1 token per second, burst of 1) must build
/// and produce a layer whose Debug output confirms the values.
#[test]
fn test_token_bucket_minimum_config_builds_and_debug_correct() {
    let cfg = RateConfig {
        tokens_per_second: 1,
        burst_capacity: 1,
        per_host: false,
    };
    let layer: RateLayer = Builder::with_config(cfg).build().expect("build");
    let dbg = format!("{layer:?}");
    assert!(
        dbg.contains("1"),
        "Debug must include the token rate value 1; got: {dbg}"
    );
}

// ---------------------------------------------------------------------------
// Large burst capacity — high-throughput services
// ---------------------------------------------------------------------------

/// Large burst capacity values must not cause overflow or rejection in the
/// token bucket initialisation path (`TokenBucket::full` sets
/// `tokens = burst_capacity as f64`).
#[test]
fn test_token_bucket_large_burst_capacity_builds() {
    let cfg = RateConfig {
        tokens_per_second: 1_000,
        burst_capacity: 100_000,
        per_host: false,
    };
    Builder::with_config(cfg)
        .build()
        .expect("large burst_capacity must not be rejected by token bucket init");
}

// ---------------------------------------------------------------------------
// per_host combinations — bucket keying logic
// ---------------------------------------------------------------------------

/// `per_host = true` means each host gets its own bucket.  Build must succeed.
#[test]
fn test_token_bucket_per_host_keying_builds() {
    let cfg = RateConfig {
        tokens_per_second: 20,
        burst_capacity: 40,
        per_host: true,
    };
    Builder::with_config(cfg).build().expect("per_host=true (per-host buckets) must build");
}

/// `per_host = false` means all hosts share one global bucket.  Build must
/// succeed.
#[test]
fn test_token_bucket_global_keying_builds() {
    let cfg = RateConfig {
        tokens_per_second: 20,
        burst_capacity: 40,
        per_host: false,
    };
    Builder::with_config(cfg).build().expect("per_host=false (global bucket) must build");
}
