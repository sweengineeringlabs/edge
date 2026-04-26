//! Integration tests for `api/traits.rs`.
//!
//! The traits module (`api/traits.rs`) exposes only `pub(crate)` type
//! aliases (`AuthStrategyTrait`, `HttpAuthTrait`) — nothing is visible
//! to external callers. The *effect* of those trait aliases is that
//! `AuthMiddleware` satisfies the `Send + Sync` bound (needed by
//! `reqwest_middleware`) and that the erased-trait dispatch compiles.
//!
//! These tests verify the structural guarantees that the trait aliases
//! underpin:
//! - `AuthMiddleware: Send + Sync`
//! - `AuthMiddleware: std::fmt::Debug`
//! - `AuthMiddleware` implements `reqwest_middleware::Middleware`

use swe_edge_egress_auth::{AuthConfig, AuthMiddleware, Builder};

// ---------------------------------------------------------------------------
// Send + Sync — compile-time checks
// ---------------------------------------------------------------------------

/// If `AuthMiddleware` is not `Send`, this function fails to compile.
fn _require_send<T: Send>() {}
/// If `AuthMiddleware` is not `Sync`, this function fails to compile.
fn _require_sync<T: Sync>() {}

#[test]
fn test_auth_middleware_is_send() {
    _require_send::<AuthMiddleware>();
}

#[test]
fn test_auth_middleware_is_sync() {
    _require_sync::<AuthMiddleware>();
}

// ---------------------------------------------------------------------------
// Debug — compile-time + runtime
// ---------------------------------------------------------------------------

#[test]
fn test_auth_middleware_implements_debug() {
    fn require_debug<T: std::fmt::Debug>() {}
    require_debug::<AuthMiddleware>();
    // Also exercise the impl at runtime.
    let mw = Builder::with_config(AuthConfig::None).build().unwrap();
    let _ = format!("{mw:?}");
}

// ---------------------------------------------------------------------------
// reqwest_middleware::Middleware bound — compile-time
// ---------------------------------------------------------------------------

/// Fails to compile if `AuthMiddleware` does not implement
/// `reqwest_middleware::Middleware`.
fn _require_middleware<T: reqwest_middleware::Middleware>() {}

#[test]
fn test_auth_middleware_implements_reqwest_middleware_middleware() {
    _require_middleware::<AuthMiddleware>();
}

// ---------------------------------------------------------------------------
// Thread safety — move into another thread (verifies Send)
// ---------------------------------------------------------------------------

#[test]
fn test_auth_middleware_can_be_moved_across_thread_boundary() {
    let mw = Builder::with_config(AuthConfig::None)
        .build()
        .expect("None config builds");
    let handle = std::thread::spawn(move || {
        // Use the middleware inside the spawned thread to confirm it
        // actually moved (and didn't just satisfy the bound vacuously).
        let s = format!("{mw:?}");
        assert!(!s.is_empty());
    });
    handle.join().expect("thread must not panic");
}

// ---------------------------------------------------------------------------
// Arc-shared middleware — verifies Sync (shared across threads)
// ---------------------------------------------------------------------------

#[test]
fn test_auth_middleware_can_be_shared_across_threads_via_arc() {
    use std::sync::Arc;
    let mw = Arc::new(
        Builder::with_config(AuthConfig::None)
            .build()
            .expect("None config builds"),
    );
    let mw2 = Arc::clone(&mw);
    let handle = std::thread::spawn(move || {
        let s = format!("{mw2:?}");
        assert!(!s.is_empty());
    });
    let _ = format!("{mw:?}");
    handle.join().expect("thread must not panic");
}
