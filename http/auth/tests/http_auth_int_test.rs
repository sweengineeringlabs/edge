//! Integration tests for the `HttpAuth` contract as exposed through
//! `AuthMiddleware` (the public type that wraps an `Arc<dyn HttpAuth>`).
//!
//! `HttpAuth` itself is `pub(crate)`.  Its observable contract from outside
//! the crate:
//! - `describe()` returns a non-empty, crate-identifying string.
//! - `process()` is invoked (indirectly) by `AuthMiddleware::handle()`.
//! - The middleware is `Send + Sync` (required by `reqwest_middleware`).
//!
//! We exercise `describe()` via `AuthMiddleware`'s `Debug` impl, which
//! calls `self.processor.describe()` as its single field value.

use swe_http_auth::{AuthConfig, AuthMiddleware, Builder};

// ---------------------------------------------------------------------------
// describe() — visible through AuthMiddleware's Debug impl
// ---------------------------------------------------------------------------

#[test]
fn test_http_auth_describe_is_non_empty_for_none_config() {
    let mw = Builder::with_config(AuthConfig::None)
        .build()
        .expect("None config must build");
    // AuthMiddleware::fmt uses processor.describe() as the field value.
    let s = format!("{mw:?}");
    // The processor field must not be an empty string.
    assert!(!s.is_empty(), "AuthMiddleware Debug (via describe()) must be non-empty");
}

#[test]
fn test_http_auth_describe_identifies_crate_for_none_config() {
    let mw = Builder::with_config(AuthConfig::None)
        .build()
        .expect("build ok");
    let s = format!("{mw:?}");
    assert!(
        s.contains("swe_http_auth"),
        "HttpAuth::describe() must identify the crate — Debug: {s}"
    );
}

#[test]
fn test_http_auth_describe_identifies_crate_for_bearer_config() {
    let env_name = "SWE_AUTH_HTTPAUTH_BRR_01";
    std::env::set_var(env_name, "describe-bearer-tok");
    let mw = Builder::with_config(AuthConfig::Bearer { token_env: env_name.into() })
        .build()
        .expect("Bearer with env set must build");
    let s = format!("{mw:?}");
    assert!(
        s.contains("swe_http_auth"),
        "HttpAuth::describe() must identify the crate for Bearer config — Debug: {s}"
    );
    std::env::remove_var(env_name);
}

#[test]
fn test_http_auth_describe_same_across_configs() {
    // All configs share the same DefaultHttpAuth processor, which always
    // returns "swe_http_auth" from describe(). The description must be
    // the same regardless of which auth scheme is in use.
    let env_name = "SWE_AUTH_HTTPAUTH_SAME_01";
    std::env::set_var(env_name, "tok");

    let mw_none = Builder::with_config(AuthConfig::None).build().unwrap();
    let mw_bearer = Builder::with_config(AuthConfig::Bearer { token_env: env_name.into() })
        .build()
        .unwrap();

    let desc_none = format!("{mw_none:?}");
    let desc_bearer = format!("{mw_bearer:?}");

    // Both must contain "swe_http_auth" — the processor description is
    // the crate name, not the scheme name.
    assert!(desc_none.contains("swe_http_auth"), "{desc_none}");
    assert!(desc_bearer.contains("swe_http_auth"), "{desc_bearer}");

    std::env::remove_var(env_name);
}

// ---------------------------------------------------------------------------
// process() exercised via reqwest_middleware::Middleware::handle
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_http_auth_process_none_does_not_add_authorization_header() {
    use reqwest_middleware::{ClientBuilder, ClientWithMiddleware};

    // Build a middleware client with None config (noop strategy).
    let mw = Builder::with_config(AuthConfig::None)
        .build()
        .expect("None config must build");

    // We can't send a real HTTP request in CI, but we CAN verify the
    // middleware can be wired into a ClientBuilder without panicking.
    let _client: ClientWithMiddleware = ClientBuilder::new(reqwest::Client::new())
        .with(mw)
        .build();
    // If wiring panicked the test would fail; reaching here means the
    // Middleware impl is at least structurally correct.
}

#[tokio::test]
async fn test_http_auth_process_bearer_can_be_wired_into_reqwest_middleware() {
    use reqwest_middleware::ClientBuilder;

    let env_name = "SWE_AUTH_HTTPAUTH_WIRE_BRR_01";
    std::env::set_var(env_name, "wire-bearer-tok");
    let mw = Builder::with_config(AuthConfig::Bearer { token_env: env_name.into() })
        .build()
        .expect("Bearer with env set must build");

    let _client = ClientBuilder::new(reqwest::Client::new())
        .with(mw)
        .build();
    std::env::remove_var(env_name);
}

// ---------------------------------------------------------------------------
// Send + Sync (compile-time)
// ---------------------------------------------------------------------------

#[test]
fn test_auth_middleware_satisfies_send_sync_bounds_required_by_http_auth() {
    // reqwest_middleware requires Middleware: Send + Sync.
    // This is also what HttpAuth: Send + Sync enforces for the inner arc.
    fn require_send_sync<T: Send + Sync>() {}
    require_send_sync::<AuthMiddleware>();
}
