//! Integration tests for `swe_edge_http_retry::Error`.
//!
//! Covers: `Error::ParseFailed`, `Error::NotImplemented` — Display messages
//! must be actionable: name the crate, embed the payload, be non-empty.

use swe_edge_http_retry::Error;

// ---------------------------------------------------------------------------
// Error::ParseFailed
// ---------------------------------------------------------------------------

/// `ParseFailed` display must name the crate so the error is traceable.
#[test]
fn test_parse_failed_display_names_the_crate() {
    let err = Error::ParseFailed("unexpected field `max_retry`".to_string());
    let msg = err.to_string();
    assert!(
        msg.contains("swe_edge_http_retry"),
        "ParseFailed Display must name the crate; got: {msg}"
    );
}

/// `ParseFailed` display must echo the supplied reason so the operator
/// knows which field or value triggered the parse failure.
#[test]
fn test_parse_failed_display_contains_supplied_reason() {
    let err = Error::ParseFailed("missing field `max_retries`".to_string());
    let msg = err.to_string();
    assert!(
        msg.contains("max_retries"),
        "ParseFailed Display must embed the reason; got: {msg}"
    );
}

/// `ParseFailed` must not produce an empty display string.
#[test]
fn test_parse_failed_display_is_non_empty() {
    let err = Error::ParseFailed("x".to_string());
    assert!(!err.to_string().is_empty());
}

/// `ParseFailed` must be `Debug`-printable without panicking.
#[test]
fn test_parse_failed_is_debug_printable() {
    let _ = format!("{:?}", Error::ParseFailed("bad config".to_string()));
}

/// `ParseFailed` must be usable as a `std::error::Error` trait object.
#[test]
fn test_parse_failed_is_std_error() {
    let err: Box<dyn std::error::Error> = Box::new(Error::ParseFailed("oops".to_string()));
    assert!(!err.to_string().is_empty());
}

// ---------------------------------------------------------------------------
// Error::NotImplemented
// ---------------------------------------------------------------------------

/// `NotImplemented` display must not be empty — a blank error gives
/// operators nothing to act on.
#[test]
fn test_not_implemented_display_is_non_empty() {
    assert!(!Error::NotImplemented("retry strategy").to_string().is_empty());
}

/// `NotImplemented` display must name the crate.
#[test]
fn test_not_implemented_display_names_the_crate() {
    let msg = Error::NotImplemented("some feature").to_string();
    assert!(
        msg.contains("swe_edge_http_retry"),
        "NotImplemented Display must name the crate; got: {msg}"
    );
}

/// `NotImplemented` display must embed the label passed in so operators
/// know which scaffold stub was reached.
#[test]
fn test_not_implemented_display_contains_label() {
    let msg = Error::NotImplemented("backoff_strategy").to_string();
    assert!(
        msg.contains("backoff_strategy"),
        "NotImplemented Display must contain the label; got: {msg}"
    );
}

/// `NotImplemented` must be `Debug`-printable without panicking.
#[test]
fn test_not_implemented_is_debug_printable() {
    let _ = format!("{:?}", Error::NotImplemented("builder"));
}

/// `NotImplemented` must be usable as a `std::error::Error` trait object.
#[test]
fn test_not_implemented_is_std_error() {
    let err: Box<dyn std::error::Error> = Box::new(Error::NotImplemented("builder"));
    assert!(!err.to_string().is_empty());
}

// ---------------------------------------------------------------------------
// Distinct variants — must not produce identical messages
// ---------------------------------------------------------------------------

/// `ParseFailed` and `NotImplemented` for the same payload must produce
/// distinct messages — callers must be able to distinguish them.
#[test]
fn test_error_variants_have_distinct_display_messages() {
    let pf = Error::ParseFailed("foo".to_string()).to_string();
    let ni = Error::NotImplemented("foo").to_string();
    assert_ne!(pf, ni, "ParseFailed and NotImplemented must produce distinct display messages");
}
