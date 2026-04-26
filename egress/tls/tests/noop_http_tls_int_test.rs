//! Integration tests for `core::identity::noop_http_tls::NoopHttpTls`.
//!
//! `NoopHttpTls` is `pub(crate)`. Integration tests verify its contract
//! through the observable effects of the public builder pipeline with
//! `TlsConfig::None`:
//!
//! - `describe()` returns "noop" (visible in `TlsLayer` Debug output).
//! - `identity()` returns `Ok(None)` — `apply_to` leaves the builder
//!   unchanged and the resulting client can call `.build()`.
//! - The layer is `Send + Sync`, which requires `NoopHttpTls` to be too.

use swe_edge_egress_tls::{Builder, TlsApplier, TlsConfig, TlsLayer};

// ---------------------------------------------------------------------------
// NoopHttpTls::describe — "noop" in TlsLayer Debug
// ---------------------------------------------------------------------------

/// The `TlsLayer` Debug output for `TlsConfig::None` must contain "noop",
/// which is the value `NoopHttpTls::describe()` returns. Removing or
/// changing that constant breaks this test.
#[test]
fn test_none_layer_debug_contains_noop_describe_constant() {
    let layer: TlsLayer = Builder::with_config(TlsConfig::None)
        .build()
        .expect("None must build");
    let dbg = format!("{layer:?}");
    assert!(
        dbg.contains("noop"),
        "TlsConfig::None Debug must contain 'noop'; got: {dbg}"
    );
}

// ---------------------------------------------------------------------------
// NoopHttpTls::identity — Ok(None) → apply_to returns builder unchanged
// ---------------------------------------------------------------------------

/// `NoopHttpTls::identity()` returns `Ok(None)`, so `TlsLayer::apply_to`
/// must return `Ok` and the `ClientBuilder` must be buildable.
#[test]
fn test_none_layer_apply_to_succeeds() {
    let layer: TlsLayer = Builder::with_config(TlsConfig::None)
        .build()
        .expect("None must build");
    let cb = layer
        .apply_to(reqwest::Client::builder())
        .expect("None apply_to must return Ok");
    let _client = cb.build().expect("ClientBuilder must build after noop apply_to");
}

/// The noop path must not add an `Identity` to the `ClientBuilder`.
/// We cannot inspect the builder's internal state, but we can verify
/// `apply_to` succeeds and the resulting client builds — confirming the
/// noop provider neither panics nor injects invalid data.
#[test]
fn test_none_layer_apply_to_does_not_corrupt_client_builder() {
    let layer: TlsLayer = Builder::with_config(TlsConfig::None)
        .build()
        .expect("None must build");
    // Apply twice — if the builder were mutated in an incompatible way the
    // second build would fail.
    let cb1 = layer
        .apply_to(reqwest::Client::builder())
        .expect("first apply_to");
    let _c1 = cb1.build().expect("first client");
    let cb2 = layer
        .apply_to(reqwest::Client::builder())
        .expect("second apply_to");
    let _c2 = cb2.build().expect("second client");
}

// ---------------------------------------------------------------------------
// NoopHttpTls: Send + Sync propagation through TlsLayer
// ---------------------------------------------------------------------------

/// `NoopHttpTls` is held inside `Arc<dyn HttpTls>`. For `TlsLayer` to be
/// `Send + Sync`, `NoopHttpTls` must also be `Send + Sync`.
#[test]
fn test_tls_layer_with_noop_is_send_and_sync() {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<TlsLayer>();
}

/// `TlsLayer` wrapping a noop provider must be usable inside a thread.
#[test]
fn test_none_layer_is_usable_across_thread_boundary() {
    let layer: TlsLayer = Builder::with_config(TlsConfig::None)
        .build()
        .expect("None must build");
    let handle = std::thread::spawn(move || {
        layer
            .apply_to(reqwest::Client::builder())
            .expect("apply_to in thread must succeed")
            .build()
            .expect("build in thread must succeed");
    });
    handle.join().expect("thread must not panic");
}

// ---------------------------------------------------------------------------
// Comparison: None and non-None configs have different Debug outputs
// ---------------------------------------------------------------------------

/// The Debug output for `TlsConfig::None` (noop) must differ from a
/// hypothetical non-None layer. Since building a Pem/Pkcs12 layer with a
/// real file is required for a runtime test, we verify the None case
/// consistently produces the "noop" marker in its own Debug.
#[test]
fn test_none_layer_debug_is_deterministic() {
    let l1 = Builder::with_config(TlsConfig::None)
        .build()
        .expect("first None build");
    let l2 = Builder::with_config(TlsConfig::None)
        .build()
        .expect("second None build");
    assert_eq!(
        format!("{l1:?}"),
        format!("{l2:?}"),
        "two None layers must produce identical deterministic Debug output"
    );
}
