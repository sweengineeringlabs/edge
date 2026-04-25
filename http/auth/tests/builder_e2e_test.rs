//! End-to-end tests for builder/SAF facade.

use swe_http_auth::{AuthConfig, Builder};

#[test]
fn e2e_builder() {
    let _b = swe_http_auth::builder().unwrap();
}

#[test]
fn e2e_with_config_parses_custom_toml_and_builds_layer() {
    let toml = r#"kind = "none""#;
    let cfg = AuthConfig::from_config(toml).expect("from_config parses");
    assert!(matches!(cfg, AuthConfig::None));
    let b = Builder::with_config(cfg);
    let _layer = b.build().expect("build ok");
}

#[test]
fn e2e_config() {
    let b = swe_http_auth::builder().unwrap();
    let _cfg = b.config();
}

#[test]
fn e2e_build() {
    let b = swe_http_auth::builder().unwrap();
    let _layer = b.build().unwrap();
}

