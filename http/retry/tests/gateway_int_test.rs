//! Integration tests exercising the public gateway surface of the swe_http_retry crate.

use swe_http_retry::{builder, Builder, Error, RetryConfig, RetryLayer};

// ---------------------------------------------------------------------------
// Helper
// ---------------------------------------------------------------------------

fn make_cfg() -> RetryConfig {
    RetryConfig {
        max_retries: 3,
        initial_interval_ms: 100,
        max_interval_ms: 5000,
        multiplier: 2.0,
        retryable_statuses: vec![500, 502, 503],
        retryable_methods: vec!["GET".to_string()],
    }
}

// ---------------------------------------------------------------------------
// builder() — SAF entry point
// ---------------------------------------------------------------------------

#[test]
fn test_builder_fn_loads_swe_default_returns_ok() {
    // builder() must succeed: the crate-shipped baseline TOML must always parse.
    builder().expect("swe_default baseline must parse without error");
}

#[test]
fn test_builder_fn_swe_default_has_at_least_one_retry() {
    // A baseline of zero retries would mean the middleware is a no-op by
    // default — the SWE default must allow at least one retry attempt.
    let b = builder().expect("baseline parses");
    assert!(
        b.config().max_retries >= 1,
        "swe_default max_retries must be >= 1, got {}",
        b.config().max_retries
    );
}

#[test]
fn test_builder_fn_swe_default_has_non_empty_retryable_statuses() {
    // If the default status list is empty the middleware will never trigger,
    // making the baseline configuration useless in practice.
    let b = builder().expect("baseline parses");
    assert!(
        !b.config().retryable_statuses.is_empty(),
        "swe_default retryable_statuses must not be empty"
    );
}

// ---------------------------------------------------------------------------
// builder().build() — finalisation
// ---------------------------------------------------------------------------

#[test]
fn test_build_from_swe_default_returns_retry_layer_with_correct_debug() {
    // The full happy path: default config → Builder → RetryLayer.
    // Debug output must name the type and expose max_retries for
    // operator visibility (log lines, test output).
    let layer = builder()
        .expect("baseline parses")
        .build()
        .expect("build must succeed");
    let dbg = format!("{layer:?}");
    assert!(
        dbg.contains("RetryLayer"),
        "Debug output must name the type; got: {dbg}"
    );
    assert!(
        dbg.contains("max_retries"),
        "Debug output must expose max_retries; got: {dbg}"
    );
}

// ---------------------------------------------------------------------------
// RetryLayer: Send + Sync — compile-time proof
// ---------------------------------------------------------------------------

#[test]
fn test_retry_layer_satisfies_send_and_sync_bounds() {
    // This test fails to compile if RetryLayer stops being Send + Sync.
    // No runtime assertion needed — the compile itself is the assertion.
    fn require_send_sync<T: Send + Sync>() {}
    require_send_sync::<RetryLayer>();
}

// ---------------------------------------------------------------------------
// Builder::with_config — custom RetryConfig flows through correctly
// ---------------------------------------------------------------------------

#[test]
fn test_builder_with_config_stores_all_custom_policy_fields() {
    let cfg = make_cfg();
    let b = Builder::with_config(cfg);
    assert_eq!(b.config().max_retries, 3, "max_retries must be stored unmodified");
    assert_eq!(
        b.config().initial_interval_ms,
        100,
        "initial_interval_ms must be stored unmodified"
    );
    assert_eq!(
        b.config().max_interval_ms,
        5000,
        "max_interval_ms must be stored unmodified"
    );
    assert_eq!(b.config().multiplier, 2.0, "multiplier must be stored unmodified");
    assert_eq!(
        b.config().retryable_statuses,
        vec![500u16, 502, 503],
        "retryable_statuses must be stored unmodified"
    );
    assert_eq!(
        b.config().retryable_methods,
        vec!["GET".to_string()],
        "retryable_methods must be stored unmodified"
    );
}

#[test]
fn test_builder_with_zero_max_retries_builds_successfully() {
    // max_retries=0 is "pass-through with no retry" — a valid choice when
    // the caller wants the middleware wired but inactive.
    let cfg = RetryConfig {
        max_retries: 0,
        initial_interval_ms: 100,
        max_interval_ms: 100,
        multiplier: 1.0,
        retryable_statuses: vec![],
        retryable_methods: vec![],
    };
    Builder::with_config(cfg)
        .build()
        .expect("max_retries=0 must produce a valid RetryLayer");
}

#[test]
fn test_builder_with_empty_retryable_lists_builds_successfully() {
    // Empty status and method lists are valid — the middleware simply never
    // triggers a retry, which matches a "disabled" deployment configuration.
    let cfg = RetryConfig {
        max_retries: 5,
        initial_interval_ms: 50,
        max_interval_ms: 2000,
        multiplier: 1.5,
        retryable_statuses: vec![],
        retryable_methods: vec![],
    };
    Builder::with_config(cfg)
        .build()
        .expect("empty retryable lists must produce a valid RetryLayer");
}

#[test]
fn test_builder_with_large_multiplier_builds_successfully() {
    // An operator might configure aggressive backoff (e.g. 100×) during
    // incident response; the builder must accept any positive f64.
    let cfg = RetryConfig {
        max_retries: 2,
        initial_interval_ms: 10,
        max_interval_ms: 60_000,
        multiplier: 100.0,
        retryable_statuses: vec![503],
        retryable_methods: vec!["POST".to_string()],
    };
    Builder::with_config(cfg)
        .build()
        .expect("multiplier=100.0 must produce a valid RetryLayer");
}

#[test]
fn test_builder_config_accessor_returns_reference_to_stored_policy() {
    // config() must hand back the same policy that will be used at runtime,
    // not a detached copy.
    let cfg = make_cfg();
    let b = Builder::with_config(cfg);
    let policy: &RetryConfig = b.config();
    assert_eq!(policy.max_retries, 3);
    assert_eq!(policy.multiplier, 2.0);
}

// ---------------------------------------------------------------------------
// Error variants — Display must be actionable
// ---------------------------------------------------------------------------

#[test]
fn test_error_parse_failed_display_contains_crate_name() {
    // Consumers catching Error::ParseFailed must be able to identify which
    // middleware produced the error without reading source code.
    let err = Error::ParseFailed("x".to_string());
    let msg = err.to_string();
    assert!(
        msg.contains("swe_http_retry"),
        "ParseFailed display must name the crate; got: {msg}"
    );
}

#[test]
fn test_error_parse_failed_display_contains_supplied_reason() {
    // The wrapped reason must appear verbatim so the operator knows which
    // field or value triggered the parse failure.
    let err = Error::ParseFailed("missing field `max_retries`".to_string());
    let msg = err.to_string();
    assert!(
        msg.contains("max_retries"),
        "ParseFailed display must echo the reason; got: {msg}"
    );
}

#[test]
fn test_error_not_implemented_display_is_non_empty_and_names_crate() {
    // A blank or opaque error message leaves operators with no actionable
    // information when the scaffold-phase feature is reached at runtime.
    let err = Error::NotImplemented("retry strategy");
    let msg = err.to_string();
    assert!(!msg.is_empty(), "NotImplemented display must not be empty");
    assert!(
        msg.contains("swe_http_retry"),
        "NotImplemented display must name the crate; got: {msg}"
    );
}
