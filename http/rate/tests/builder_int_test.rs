//! Integration tests for `api/builder.rs` and `saf/builder.rs`.
//!
//! Covers the full public builder surface: the `builder()` free function and
//! the `Builder` type's `with_config`, `config`, and `build` methods.

use swe_edge_http_rate::{Builder, Error, RateConfig, RateLayer};

// ---------------------------------------------------------------------------
// builder() free function
// ---------------------------------------------------------------------------

/// The `builder()` free function must return `Ok` — the crate-shipped TOML
/// baseline must always be parseable.
#[test]
fn test_builder_fn_succeeds_with_swe_default() {
    swe_edge_http_rate::builder()
        .expect("builder() must succeed with the crate-shipped baseline");
}

/// `tokens_per_second` must be >= 1 in the default config.  A rate of 0
/// would block all requests permanently.
#[test]
fn test_builder_fn_swe_default_tokens_per_second_is_positive() {
    let b = swe_edge_http_rate::builder().expect("baseline parses");
    assert!(
        b.config().tokens_per_second >= 1,
        "swe_default tokens_per_second must be >= 1, got {}",
        b.config().tokens_per_second
    );
}

/// `burst_capacity` must be >= 1 in the default config.  A burst of 0 would
/// deny every request that does not arrive exactly at the refill moment.
#[test]
fn test_builder_fn_swe_default_burst_capacity_is_positive() {
    let b = swe_edge_http_rate::builder().expect("baseline parses");
    assert!(
        b.config().burst_capacity >= 1,
        "swe_default burst_capacity must be >= 1, got {}",
        b.config().burst_capacity
    );
}

// ---------------------------------------------------------------------------
// Builder::with_config — custom policy round-trips correctly
// ---------------------------------------------------------------------------

/// `with_config` must preserve every field without modification.
#[test]
fn test_with_config_preserves_all_fields() {
    let cfg = RateConfig {
        tokens_per_second: 50,
        burst_capacity: 100,
        per_host: true,
    };
    let b = Builder::with_config(cfg);
    let policy = b.config();
    assert_eq!(policy.tokens_per_second, 50);
    assert_eq!(policy.burst_capacity, 100);
    assert!(policy.per_host);
}

/// `with_config` with `per_host = false` must preserve the flag.
#[test]
fn test_with_config_per_host_false_preserved() {
    let cfg = RateConfig {
        tokens_per_second: 10,
        burst_capacity: 20,
        per_host: false,
    };
    let b = Builder::with_config(cfg);
    assert!(!b.config().per_host, "per_host=false must survive with_config");
}

/// `config()` must return a reference to the stored policy.
#[test]
fn test_config_accessor_returns_reference_not_divergent_copy() {
    let cfg = RateConfig {
        tokens_per_second: 7,
        burst_capacity: 14,
        per_host: true,
    };
    let b = Builder::with_config(cfg);
    let policy: &RateConfig = b.config();
    assert_eq!(policy.tokens_per_second, 7);
    assert_eq!(policy.burst_capacity, 14);
}

// ---------------------------------------------------------------------------
// Builder::build — produces a usable RateLayer
// ---------------------------------------------------------------------------

/// The nominal build path must succeed.
#[test]
fn test_build_from_swe_default_returns_rate_layer() {
    let layer: RateLayer = swe_edge_http_rate::builder()
        .expect("baseline parses")
        .build()
        .expect("build() must succeed");
    let dbg = format!("{layer:?}");
    assert!(
        dbg.contains("RateLayer"),
        "Debug must identify the type; got: {dbg}"
    );
}

/// Building from a custom config must succeed.
#[test]
fn test_build_with_custom_config_succeeds() {
    let cfg = RateConfig {
        tokens_per_second: 20,
        burst_capacity: 40,
        per_host: false,
    };
    Builder::with_config(cfg).build().expect("custom config must build");
}

/// `per_host = true` must build successfully.
#[test]
fn test_build_with_per_host_true_succeeds() {
    let cfg = RateConfig {
        tokens_per_second: 10,
        burst_capacity: 10,
        per_host: true,
    };
    Builder::with_config(cfg).build().expect("per_host=true must build");
}

/// `per_host = false` must build successfully.
#[test]
fn test_build_with_per_host_false_succeeds() {
    let cfg = RateConfig {
        tokens_per_second: 10,
        burst_capacity: 10,
        per_host: false,
    };
    Builder::with_config(cfg).build().expect("per_host=false must build");
}

/// High token rate and burst capacity are valid operator choices.
#[test]
fn test_build_with_high_rate_and_burst_succeeds() {
    let cfg = RateConfig {
        tokens_per_second: 10_000,
        burst_capacity: 50_000,
        per_host: false,
    };
    Builder::with_config(cfg).build().expect("high rate + burst must build");
}

// ---------------------------------------------------------------------------
// Error variants — public constructability
// ---------------------------------------------------------------------------

/// `Error::ParseFailed` display must name the crate and echo the reason.
#[test]
fn test_error_parse_failed_display_names_crate_and_echoes_reason() {
    let reason = "missing field `tokens_per_second`";
    let err = Error::ParseFailed(reason.to_string());
    let msg = err.to_string();
    assert!(
        msg.contains("swe_edge_http_rate"),
        "ParseFailed display must name the crate; got: {msg}"
    );
    assert!(
        msg.contains(reason),
        "ParseFailed display must echo the reason; got: {msg}"
    );
}

/// `Error::NotImplemented` display must be non-empty and name the crate.
#[test]
fn test_error_not_implemented_display_is_non_empty_and_names_crate() {
    let err = Error::NotImplemented("token bucket");
    let msg = err.to_string();
    assert!(!msg.is_empty());
    assert!(
        msg.contains("swe_edge_http_rate"),
        "NotImplemented display must name the crate; got: {msg}"
    );
}
