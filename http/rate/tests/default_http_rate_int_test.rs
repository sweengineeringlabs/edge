//! Integration tests for `core/default_http_rate.rs`.
//!
//! `DefaultHttpRate` is `pub(crate)`.  Its observable effect is through the
//! SAF `builder()` function, which loads the crate-shipped SWE baseline.

use swe_edge_http_rate::{Builder, RateConfig};

// ---------------------------------------------------------------------------
// SWE baseline — verify default config has production-safe values
// ---------------------------------------------------------------------------

/// The `builder()` function must load the baseline without error.
#[test]
fn test_default_http_rate_swe_default_builder_succeeds() {
    swe_edge_http_rate::builder().expect("swe_default baseline must parse without error");
}

/// Default `tokens_per_second` must be >= 1.
#[test]
fn test_default_http_rate_swe_default_tokens_per_second_is_positive() {
    let b = swe_edge_http_rate::builder().expect("baseline parses");
    assert!(
        b.config().tokens_per_second >= 1,
        "swe_default tokens_per_second must be >= 1, got {}",
        b.config().tokens_per_second
    );
}

/// Default `burst_capacity` must be >= 1.
#[test]
fn test_default_http_rate_swe_default_burst_capacity_is_positive() {
    let b = swe_edge_http_rate::builder().expect("baseline parses");
    assert!(
        b.config().burst_capacity >= 1,
        "swe_default burst_capacity must be >= 1, got {}",
        b.config().burst_capacity
    );
}

/// Building from the SWE default must produce a valid `RateLayer`.
#[test]
fn test_default_http_rate_swe_default_builds_rate_layer() {
    swe_edge_http_rate::builder()
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
fn test_default_http_rate_custom_config_is_not_overridden_by_swe_default() {
    let custom = RateConfig {
        tokens_per_second: 3,
        burst_capacity: 7,
        per_host: true,
    };
    let b = Builder::with_config(custom);
    assert_eq!(b.config().tokens_per_second, 3);
    assert_eq!(b.config().burst_capacity, 7);
    assert!(b.config().per_host);
}
