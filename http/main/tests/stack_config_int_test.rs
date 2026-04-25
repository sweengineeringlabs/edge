//! Integration tests for `api::stack_config::StackConfig`.

use swe_edge_http_auth::AuthConfig;
use swe_edge_http_main::StackConfig;

/// @covers: StackConfig — struct can be constructed with AuthConfig::None
#[test]
fn test_stack_config_construction_with_none_auth() {
    let cfg = StackConfig { auth: AuthConfig::None };
    assert!(matches!(cfg.auth, AuthConfig::None));
}

/// @covers: StackConfig — Clone produces independent copy
#[test]
fn test_stack_config_clone_is_independent() {
    let cfg = StackConfig { auth: AuthConfig::None };
    let cloned = cfg.clone();
    assert!(matches!(cloned.auth, AuthConfig::None));
}

/// @covers: StackConfig — Debug format is non-empty
#[test]
fn test_stack_config_debug_non_empty() {
    let cfg = StackConfig { auth: AuthConfig::None };
    assert!(!format!("{cfg:?}").is_empty());
}
