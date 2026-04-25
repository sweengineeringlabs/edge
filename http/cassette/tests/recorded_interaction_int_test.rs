//! Integration tests for `core::recorded_interaction` — the
//! `RecordedInteraction`, `RecordedRequest`, and `RecordedResponse` types.
//!
//! All three types are `pub(crate)`. Integration tests verify their
//! contract through the public cassette pipeline: a cassette YAML file
//! written in the format these types serialize to must be loadable by
//! `CassetteLayer::new` and replayable by the middleware.
//!
//! Tests confirm:
//! - The YAML schema expected by `RecordedInteraction` serde is stable.
//! - Optional fields (`body_hash`) are handled correctly when absent.
//! - The middleware correctly reconstructs a `reqwest::Response` from the
//!   recorded bytes stored in `body_base64`.

use swe_edge_http_cassette::{Builder, CassetteConfig};

fn replay_cfg(dir: &str, cassette_name: &str) -> (CassetteConfig, String) {
    let cfg = CassetteConfig {
        mode: "replay".to_string(),
        cassette_dir: dir.replace('\\', "/"),
        match_on: vec!["method".to_string(), "url".to_string()],
        scrub_headers: vec![],
        scrub_body_paths: vec![],
    };
    (cfg, cassette_name.to_string())
}

// ---------------------------------------------------------------------------
// YAML schema stability — load and replay a hand-written cassette
// ---------------------------------------------------------------------------

/// A cassette YAML written in the serialized format of `RecordedInteraction`
/// must load without error and replay the correct status + body. This test
/// would fail if the serde field names change (e.g. `body_base64` renamed
/// to `body_bytes`).
#[tokio::test]
async fn test_recorded_interaction_yaml_schema_is_stable_for_get_200() {
    use base64::Engine;
    use reqwest_middleware::ClientBuilder;

    let tmpdir = tempfile::tempdir().unwrap();
    let dir = tmpdir.path();

    let body_b64 = base64::engine::general_purpose::STANDARD.encode(r#"{"status":"ok"}"#);
    let yaml = format!(
        r#"
method=GET|url=https://api.example.test/health:
  request:
    method: GET
    url: "https://api.example.test/health"
  response:
    status: 200
    headers:
      content-type: application/json
    body_base64: {body_b64}
"#
    );
    std::fs::write(dir.join("schema_stable.yaml"), yaml).unwrap();

    let (cfg, name) = replay_cfg(dir.to_str().unwrap(), "schema_stable");
    let layer = Builder::with_config(cfg).build(&name).expect("build must succeed");
    let client = ClientBuilder::new(reqwest::Client::new()).with(layer).build();

    let resp = client
        .get("https://api.example.test/health")
        .send()
        .await
        .expect("replay must succeed");
    assert_eq!(resp.status().as_u16(), 200);
    let body = resp.text().await.expect("read body");
    assert_eq!(body, r#"{"status":"ok"}"#);
}

/// A cassette with `body_hash` as an optional field (present in `request`)
/// must still parse and replay correctly. This verifies the
/// `#[serde(default, skip_serializing_if = "Option::is_none")]` on
/// `body_hash` works as expected.
#[tokio::test]
async fn test_recorded_interaction_with_body_hash_field_replays_correctly() {
    use base64::Engine;
    use reqwest_middleware::ClientBuilder;

    let tmpdir = tempfile::tempdir().unwrap();
    let dir = tmpdir.path();

    let body_b64 = base64::engine::general_purpose::STANDARD.encode("hello");
    let yaml = format!(
        r#"
method=GET|url=https://api.example.test/greet:
  request:
    method: GET
    url: "https://api.example.test/greet"
    body_hash: "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
  response:
    status: 200
    headers: {{}}
    body_base64: {body_b64}
"#
    );
    std::fs::write(dir.join("with_body_hash.yaml"), yaml).unwrap();

    let (cfg, name) = replay_cfg(dir.to_str().unwrap(), "with_body_hash");
    let layer = Builder::with_config(cfg).build(&name).expect("build must succeed");
    let client = ClientBuilder::new(reqwest::Client::new()).with(layer).build();

    let resp = client
        .get("https://api.example.test/greet")
        .send()
        .await
        .expect("replay must succeed");
    assert_eq!(resp.status().as_u16(), 200);
    assert_eq!(resp.text().await.expect("body"), "hello");
}

/// A cassette with a non-200 status code must replay that exact status,
/// confirming the `status: u16` field round-trips through YAML correctly.
#[tokio::test]
async fn test_recorded_interaction_non_200_status_replays_correctly() {
    use base64::Engine;
    use reqwest_middleware::ClientBuilder;

    let tmpdir = tempfile::tempdir().unwrap();
    let dir = tmpdir.path();

    let body_b64 = base64::engine::general_purpose::STANDARD.encode("Not Found");
    let yaml = format!(
        r#"
method=GET|url=https://api.example.test/missing:
  request:
    method: GET
    url: "https://api.example.test/missing"
  response:
    status: 404
    headers:
      content-type: text/plain
    body_base64: {body_b64}
"#
    );
    std::fs::write(dir.join("status_404.yaml"), yaml).unwrap();

    let (cfg, name) = replay_cfg(dir.to_str().unwrap(), "status_404");
    let layer = Builder::with_config(cfg).build(&name).expect("build must succeed");
    let client = ClientBuilder::new(reqwest::Client::new()).with(layer).build();

    let resp = client
        .get("https://api.example.test/missing")
        .send()
        .await
        .expect("replay must succeed");
    assert_eq!(resp.status().as_u16(), 404, "404 must replay as 404");
}

/// An empty cassette file (no interactions) must load as an empty fixture
/// map, not as an error. The layer must build successfully.
#[test]
fn test_empty_cassette_file_loads_without_error() {
    let tmpdir = tempfile::tempdir().unwrap();
    let dir = tmpdir.path();
    // Write an empty (whitespace-only) YAML file.
    std::fs::write(dir.join("empty_cassette.yaml"), "   \n").unwrap();

    let (cfg, name) = replay_cfg(dir.to_str().unwrap(), "empty_cassette");
    Builder::with_config(cfg)
        .build(&name)
        .expect("empty cassette file must load without error");
}

/// A malformed YAML cassette file must cause `build` to return an error
/// (not panic), so test infrastructure failures are diagnosed clearly.
#[test]
fn test_malformed_cassette_yaml_returns_parse_error() {
    let tmpdir = tempfile::tempdir().unwrap();
    let dir = tmpdir.path();
    // Write YAML that doesn't conform to the RecordedInteraction schema.
    std::fs::write(dir.join("malformed.yaml"), ": invalid: yaml: [[\n").unwrap();

    let (cfg, name) = replay_cfg(dir.to_str().unwrap(), "malformed");
    let err = Builder::with_config(cfg).build(&name).unwrap_err();
    let msg = err.to_string();
    assert!(
        msg.contains("swe_edge_http_cassette") || msg.contains("parse"),
        "malformed YAML must return a ParseFailed error; got: {msg}"
    );
}
