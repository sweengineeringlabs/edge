//! DefaultService integration tests for controlroom.

use controlroom::*;

#[test]
fn test_default_service_through_facade() {
    // Exercise the default service through the saf facade
    assert!(run().is_ok());
}
