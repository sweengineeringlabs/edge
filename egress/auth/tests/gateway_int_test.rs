//! Integration tests exercising the public gateway surface of the swe_edge_egress_auth crate.

use swe_edge_egress_auth::{AuthConfig, AuthMiddleware, Builder, Error};

#[test]
fn test_builder_fn_loads_swe_default_and_succeeds() {
    swe_edge_egress_auth::builder().expect("builder() must succeed");
}

#[test]
fn test_builder_fn_default_config_is_none_pass_through() {
    let b = swe_edge_egress_auth::builder().expect("builder() must succeed");
    assert!(matches!(b.config(), AuthConfig::None), "swe_default must be AuthConfig::None");
}

#[test]
fn test_with_config_none_stores_none_variant() {
    let b = Builder::with_config(AuthConfig::None);
    assert!(matches!(b.config(), AuthConfig::None));
}

#[test]
fn test_build_none_config_produces_auth_middleware() {
    let mw: AuthMiddleware = Builder::with_config(AuthConfig::None)
        .build()
        .expect("None config must build");
    let s = format!("{mw:?}");
    assert!(!s.is_empty(), "AuthMiddleware Debug must be non-empty: {s}");
}

#[test]
fn test_auth_middleware_is_send_and_sync() {
    fn require_send_sync<T: Send + Sync>() {}
    require_send_sync::<AuthMiddleware>();
}

#[test]
fn test_build_bearer_missing_env_returns_missing_env_var() {
    let env_name = "SWE_GW_IT_AUTH_BEARER_01";
    std::env::remove_var(env_name);
    let cfg = AuthConfig::Bearer { token_env: env_name.into() };
    let err = Builder::with_config(cfg).build().unwrap_err();
    match err {
        Error::MissingEnvVar { name } => assert_eq!(name, env_name),
        other => panic!("expected MissingEnvVar, got {other:?}"),
    }
}

#[test]
fn test_build_bearer_env_set_produces_middleware() {
    let env_name = "SWE_GW_IT_AUTH_BEARER_02";
    std::env::set_var(env_name, "tok-test-value");
    let cfg = AuthConfig::Bearer { token_env: env_name.into() };
    Builder::with_config(cfg).build().expect("bearer with env set must build");
    std::env::remove_var(env_name);
}

#[test]
fn test_build_basic_missing_user_env_returns_missing_env_var() {
    let user_env = "SWE_GW_IT_AUTH_BASIC_U_01";
    let pass_env = "SWE_GW_IT_AUTH_BASIC_P_01";
    std::env::remove_var(user_env);
    std::env::remove_var(pass_env);
    let cfg = AuthConfig::Basic { user_env: user_env.into(), pass_env: pass_env.into() };
    let err = Builder::with_config(cfg).build().unwrap_err();
    assert!(matches!(err, Error::MissingEnvVar { .. }), "missing basic env must fail: {err:?}");
}

#[test]
fn test_build_header_missing_value_env_returns_missing_env_var() {
    let env_name = "SWE_GW_IT_AUTH_HEADER_01";
    std::env::remove_var(env_name);
    let cfg = AuthConfig::Header { name: "x-api-key".into(), value_env: env_name.into() };
    let err = Builder::with_config(cfg).build().unwrap_err();
    assert!(matches!(err, Error::MissingEnvVar { .. }), "missing header env must fail: {err:?}");
}

#[test]
fn test_with_config_bearer_stores_bearer_variant() {
    let cfg = AuthConfig::Bearer { token_env: "IRRELEVANT".into() };
    let b = Builder::with_config(cfg);
    assert!(matches!(b.config(), AuthConfig::Bearer { .. }));
}

#[test]
fn test_error_parse_failed_display_contains_crate_name() {
    let err = Error::ParseFailed("oops".to_string());
    let s = err.to_string();
    assert!(s.contains("swe_edge_egress_auth"), "ParseFailed Display must name the crate: {s}");
}

#[test]
fn test_error_missing_env_var_display_contains_var_name() {
    let err = Error::MissingEnvVar { name: "MY_SECRET_VAR".to_string() };
    let s = err.to_string();
    assert!(s.contains("MY_SECRET_VAR"), "MissingEnvVar Display must contain var name: {s}");
}

#[test]
fn test_error_not_implemented_display_is_non_empty() {
    let s = Error::NotImplemented("test").to_string();
    assert!(!s.is_empty(), "NotImplemented Display must not be empty");
}

#[test]
fn test_build_none_config_always_succeeds_regardless_of_env() {
    Builder::with_config(AuthConfig::None)
        .build()
        .expect("None config must always build regardless of env state");
}
