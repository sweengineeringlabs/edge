//! Integration tests for `core::cassette_layer` — the `CassetteLayer` impl
//! block containing `new`, `match_key`, `flush_to_disk`, and the
//! `reqwest_middleware::Middleware` implementation.
//!
//! All core functions are `pub(crate)`; integration tests verify observable
//! effects:
//!
//! - A layer built with `mode="replay"` on a missing fixture file starts
//!   with an empty fixture map (no panic, no I/O error).
//! - A pre-written fixture YAML file is loaded at build time, so a
//!   subsequent middleware invocation replays from it without hitting the
//!   network.
//! - The middleware returns an error when `mode="replay"` and no fixture
//!   matches the incoming request.

use std::collections::BTreeMap;

use swe_http_cassette::{Builder, CassetteConfig};

fn make_cfg(dir: &str, mode: &str, match_on: Vec<String>) -> CassetteConfig {
    CassetteConfig {
        mode: mode.to_string(),
        cassette_dir: dir.replace('\\', "/"),
        match_on,
        scrub_headers: vec!["authorization".to_string()],
        scrub_body_paths: vec![],
    }
}

// ---------------------------------------------------------------------------
// new() — no fixture file → empty layer
// ---------------------------------------------------------------------------

/// Building with `mode="replay"` on a directory that has no fixture file
/// must succeed and produce a layer whose cassette path does not yet exist
/// on disk (the file is created on first flush in record / auto mode).
#[test]
fn test_new_missing_fixture_file_starts_empty_layer() {
    let tmpdir = tempfile::tempdir().unwrap();
    let dir = tmpdir.path().to_str().unwrap();
    let layer = Builder::with_config(make_cfg(
        dir,
        "replay",
        vec!["method".to_string(), "url".to_string()],
    ))
    .build("missing_fixture")
    .expect("build must succeed");

    // Cassette file must NOT be created at build time — only on record.
    let expected_path = tmpdir.path().join("missing_fixture.yaml");
    assert!(
        !expected_path.exists(),
        "cassette file must not exist before any request is handled"
    );
    drop(layer);
}

// ---------------------------------------------------------------------------
// new() — pre-written fixture file → loaded at build time
// ---------------------------------------------------------------------------

/// If the cassette file already exists on disk, `CassetteLayer::new` must
/// load it into the in-memory fixture map without error.  We write a minimal
/// valid YAML cassette manually and confirm the layer builds successfully
/// (the replay path in the middleware depends on this).
#[test]
fn test_new_with_existing_fixture_file_succeeds() {
    let tmpdir = tempfile::tempdir().unwrap();
    let dir_path = tmpdir.path();

    // Minimal valid cassette YAML produced by the same serde_yaml path
    // the CassetteLayer itself uses for `flush_to_disk`.
    let yaml = r#"
method=GET|url=https://example.test/:
  request:
    method: GET
    url: "https://example.test/"
  response:
    status: 200
    headers:
      content-type: application/json
    body_base64: e30=
"#;
    std::fs::write(dir_path.join("pre_written.yaml"), yaml).unwrap();

    let dir = dir_path.to_str().unwrap();
    let layer = Builder::with_config(make_cfg(
        dir,
        "replay",
        vec!["method".to_string(), "url".to_string()],
    ))
    .build("pre_written")
    .expect("layer must load pre-written fixture file without error");
    drop(layer);
}

// ---------------------------------------------------------------------------
// Middleware: replay mode returns error on cache miss
// ---------------------------------------------------------------------------

/// When `mode="replay"` and no fixture matches the request, the middleware
/// must return an error with a message that names the cassette path and the
/// unmatched key, giving the operator enough context to diagnose the miss.
#[tokio::test]
async fn test_middleware_replay_mode_returns_error_on_cache_miss() {
    use reqwest_middleware::ClientBuilder;

    let tmpdir = tempfile::tempdir().unwrap();
    let dir = tmpdir.path().to_str().unwrap();
    let layer = Builder::with_config(make_cfg(
        dir,
        "replay",
        vec!["method".to_string(), "url".to_string()],
    ))
    .build("replay_miss")
    .expect("build must succeed");

    let client = ClientBuilder::new(reqwest::Client::new())
        .with(layer)
        .build();

    // This URL has no fixture — replay mode must return an error, not panic,
    // not make a real network call.
    let result = client.get("https://no-such-fixture.test/api").send().await;
    assert!(
        result.is_err(),
        "replay mode with no matching fixture must return Err; got Ok"
    );
    let err_msg = result.unwrap_err().to_string();
    assert!(
        err_msg.contains("swe_http_cassette") || err_msg.contains("no recorded interaction"),
        "error must identify the cassette miss; got: {err_msg}"
    );
}

// ---------------------------------------------------------------------------
// Middleware: replay mode returns recorded response on cache hit
// ---------------------------------------------------------------------------

/// When the cassette file contains a matching fixture, the middleware must
/// return the recorded response without making a network request. This is
/// the core value proposition of the cassette middleware.
#[tokio::test]
async fn test_middleware_replay_mode_returns_recorded_response_on_hit() {
    use base64::Engine;
    use reqwest_middleware::ClientBuilder;

    let tmpdir = tempfile::tempdir().unwrap();
    let dir_path = tmpdir.path();

    // Encode `{"ok":true}` → base64.
    let body_b64 = base64::engine::general_purpose::STANDARD.encode(r#"{"ok":true}"#);

    // Write a fixture that matches GET https://example.test/status.
    let yaml = format!(
        r#"
method=GET|url=https://example.test/status:
  request:
    method: GET
    url: "https://example.test/status"
  response:
    status: 200
    headers:
      content-type: application/json
    body_base64: {body_b64}
"#
    );
    std::fs::write(dir_path.join("replay_hit.yaml"), yaml).unwrap();

    let dir = dir_path.to_str().unwrap();
    let layer = Builder::with_config(make_cfg(
        dir,
        "replay",
        vec!["method".to_string(), "url".to_string()],
    ))
    .build("replay_hit")
    .expect("build must succeed");

    let client = ClientBuilder::new(reqwest::Client::new())
        .with(layer)
        .build();

    let resp = client
        .get("https://example.test/status")
        .send()
        .await
        .expect("replay must succeed on a fixture hit");

    assert_eq!(resp.status().as_u16(), 200, "replayed status must match fixture");
    let body = resp.text().await.expect("must read body");
    assert_eq!(body, r#"{"ok":true}"#, "replayed body must match fixture");
}

// ---------------------------------------------------------------------------
// match_key: different methods produce different keys
// ---------------------------------------------------------------------------

/// Two requests to the same URL but with different HTTP methods must miss
/// each other's fixtures when `method` is included in `match_on`.
#[tokio::test]
async fn test_middleware_replay_different_methods_do_not_share_fixtures() {
    use base64::Engine;
    use reqwest_middleware::ClientBuilder;

    let tmpdir = tempfile::tempdir().unwrap();
    let dir_path = tmpdir.path();

    let body_b64 = base64::engine::general_purpose::STANDARD.encode("GET response");
    let yaml = format!(
        r#"
method=GET|url=https://example.test/res:
  request:
    method: GET
    url: "https://example.test/res"
  response:
    status: 200
    headers: {{}}
    body_base64: {body_b64}
"#
    );
    std::fs::write(dir_path.join("method_isolation.yaml"), yaml).unwrap();

    let dir = dir_path.to_str().unwrap();
    let layer = Builder::with_config(make_cfg(
        dir,
        "replay",
        vec!["method".to_string(), "url".to_string()],
    ))
    .build("method_isolation")
    .expect("build must succeed");

    let client = ClientBuilder::new(reqwest::Client::new())
        .with(layer)
        .build();

    // POST to the same URL must miss (GET fixture doesn't match POST).
    let result = client.post("https://example.test/res").send().await;
    assert!(
        result.is_err(),
        "POST to a URL only recorded for GET must result in replay miss"
    );
}
