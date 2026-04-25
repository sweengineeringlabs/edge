//! Integration tests for `api::error::Error`.

use swe_edge_http_main::Error;

/// @covers: Error::BuildFailed — Display format is non-empty and contains cause
#[test]
fn test_build_failed_display_contains_cause() {
    let e = Error::BuildFailed("auth middleware failed".to_string());
    let msg = e.to_string();
    assert!(msg.contains("middleware build failed"), "display: {msg}");
    assert!(msg.contains("auth middleware failed"), "display: {msg}");
}

/// @covers: Error::NotImplemented — Display format is non-empty
#[test]
fn test_not_implemented_display_non_empty() {
    let e = Error::NotImplemented("feature X");
    let msg = e.to_string();
    assert!(msg.contains("not implemented"), "display: {msg}");
    assert!(msg.contains("feature X"), "display: {msg}");
}

/// @covers: Error — std::error::Error is implemented
#[test]
fn test_error_implements_std_error() {
    let e: Box<dyn std::error::Error> = Box::new(Error::BuildFailed("x".into()));
    assert!(!e.to_string().is_empty());
}
