//! Integration tests exercising the public gateway surface of the swe-http-main crate.

use swe_http_main::{Builder, builder};

#[test]
fn test_builder_fn_loads_swe_defaults_and_succeeds() {
    builder().expect("builder() must succeed");
}

#[test]
fn test_with_config_stores_config() {
    use swe_http_main::StackConfig;
    use swe_http_auth::AuthConfig;
    let cfg = StackConfig { auth: AuthConfig::None };
    let b = Builder::with_config(cfg);
    assert!(matches!(b.config().auth, AuthConfig::None));
}
