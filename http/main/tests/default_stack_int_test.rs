//! Integration tests for `core::stack::default_stack::DefaultStack`.

use swe_edge_http_auth::AuthConfig;
use swe_edge_http_main::{Builder, StackConfig};

/// @covers: DefaultStack::new — builder() entry point exercises the constructor path
#[test]
fn test_builder_exercises_default_stack_construction() {
    let b = Builder::with_config(StackConfig { auth: AuthConfig::None });
    assert!(matches!(b.config().auth, AuthConfig::None));
}

/// @covers: DefaultStack + StackAssembler — the assembled config is accessible
#[test]
fn test_default_stack_config_accessible_through_builder() {
    let cfg = StackConfig { auth: AuthConfig::Bearer { token_env: "TOKEN".into() } };
    let b = Builder::with_config(cfg);
    assert!(matches!(b.config().auth, AuthConfig::Bearer { .. }));
}
