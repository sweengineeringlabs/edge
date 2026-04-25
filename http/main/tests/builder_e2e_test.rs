//! End-to-end tests for the swe-http-main SAF builder surface.

use swe_edge_http_main::{Builder, StackConfig};
use swe_edge_http_auth::AuthConfig;

/// @covers: builder
#[test]
fn test_e2e_builder() {
    let b = swe_edge_http_main::builder().expect("builder() must succeed");
    assert!(matches!(b.config().auth, AuthConfig::None));
}

/// @covers: Builder::with_config
#[test]
fn test_e2e_with_config() {
    let cfg = StackConfig { auth: AuthConfig::None };
    let b = Builder::with_config(cfg);
    assert!(matches!(b.config().auth, AuthConfig::None));
}

/// @covers: Builder::config
#[test]
fn test_e2e_config() {
    let cfg = StackConfig { auth: AuthConfig::None };
    let b = Builder::with_config(cfg);
    let c = b.config();
    assert!(matches!(c.auth, AuthConfig::None), "config() must return stored aggregate config");
}
