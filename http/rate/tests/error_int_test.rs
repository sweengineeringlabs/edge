//! Integration tests for `api/error.rs` — the public `Error` enum.

use swe_edge_http_rate::Error;

// ---------------------------------------------------------------------------
// Error::ParseFailed
// ---------------------------------------------------------------------------

/// `ParseFailed` must be publicly constructable.
#[test]
fn test_error_parse_failed_is_publicly_constructable() {
    let _err = Error::ParseFailed("any reason".to_string());
}

/// `Display` must name the crate.
#[test]
fn test_error_parse_failed_display_names_crate() {
    let err = Error::ParseFailed("bad field".to_string());
    let msg = err.to_string();
    assert!(
        msg.contains("swe_edge_http_rate"),
        "ParseFailed display must name the crate; got: {msg}"
    );
}

/// `Display` must echo the supplied reason.
#[test]
fn test_error_parse_failed_display_echoes_reason() {
    let reason = "missing field `tokens_per_second`";
    let err = Error::ParseFailed(reason.to_string());
    assert!(
        err.to_string().contains(reason),
        "ParseFailed display must contain the reason; got: {}",
        err
    );
}

/// Empty reason still produces a non-empty display.
#[test]
fn test_error_parse_failed_with_empty_reason_display_is_non_empty() {
    let err = Error::ParseFailed(String::new());
    assert!(
        !err.to_string().is_empty(),
        "ParseFailed display must not be empty even with empty reason"
    );
}

// ---------------------------------------------------------------------------
// Error::NotImplemented
// ---------------------------------------------------------------------------

/// `NotImplemented` must be publicly constructable.
#[test]
fn test_error_not_implemented_is_publicly_constructable() {
    let _err = Error::NotImplemented("some feature");
}

/// `Display` must name the crate.
#[test]
fn test_error_not_implemented_display_names_crate() {
    let err = Error::NotImplemented("token bucket");
    let msg = err.to_string();
    assert!(
        msg.contains("swe_edge_http_rate"),
        "NotImplemented display must name the crate; got: {msg}"
    );
}

/// `Display` must not be empty.
#[test]
fn test_error_not_implemented_display_is_non_empty() {
    assert!(
        !Error::NotImplemented("x").to_string().is_empty(),
        "NotImplemented display must not be empty"
    );
}

// ---------------------------------------------------------------------------
// Debug — both variants
// ---------------------------------------------------------------------------

/// Both variants must implement `Debug`.
#[test]
fn test_error_variants_implement_debug() {
    let _ = format!("{:?}", Error::ParseFailed("p".to_string()));
    let _ = format!("{:?}", Error::NotImplemented("n"));
}

// ---------------------------------------------------------------------------
// Distinctness
// ---------------------------------------------------------------------------

/// The two variants must produce distinguishable `Display` output.
#[test]
fn test_error_parse_failed_and_not_implemented_display_are_distinct() {
    let a = Error::ParseFailed("foo".to_string()).to_string();
    let b = Error::NotImplemented("foo").to_string();
    assert_ne!(a, b, "the two Error variants must format differently");
}
