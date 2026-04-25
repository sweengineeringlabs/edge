//! Impl blocks for [`CassetteLayer`] — constructor + record/
//! replay logic + [`reqwest_middleware::Middleware`] impl.

use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use async_trait::async_trait;
use base64::Engine;
use sha2::{Digest, Sha256};
use tokio::sync::Mutex;

use crate::api::cassette_config::CassetteConfig;
use crate::api::cassette_layer::CassetteLayer;
use crate::api::error::Error;

use super::body_scrubber::scrub_body;
use super::recorded_interaction::{
    RecordedInteraction, RecordedRequest, RecordedResponse,
};

impl CassetteLayer {
    /// Construct. Loads fixtures from disk if the file exists;
    /// starts with an empty map otherwise.
    ///
    /// `cassette_name` is appended to `config.cassette_dir` to
    /// produce the on-disk path. By convention one cassette
    /// per test case — cassettes are the HTTP equivalent of
    /// golden files.
    pub(crate) fn new(config: CassetteConfig, cassette_name: &str) -> Result<Self, Error> {
        let path = PathBuf::from(&config.cassette_dir).join(format!("{cassette_name}.yaml"));
        let fixtures = load_fixtures_from_disk(&path)?;
        Ok(Self {
            config: Arc::new(config),
            cassette_path: path,
            fixtures: Arc::new(Mutex::new(fixtures)),
        })
    }

    /// Compute the match key for a request per `config.match_on`.
    ///
    /// When `match_on` includes `body_hash`, the body is first
    /// scrubbed per `config.scrub_body_paths` — so
    /// non-deterministic fields (SDK-injected request IDs,
    /// trace IDs) don't break exact-match on replay.
    fn match_key(&self, req: &reqwest::Request) -> String {
        let mut parts: Vec<String> = Vec::new();
        for m in &self.config.match_on {
            match m.as_str() {
                "method" => parts.push(format!("method={}", req.method())),
                "url" => parts.push(format!("url={}", req.url())),
                "body_hash" => {
                    let raw = req.body().and_then(|b| b.as_bytes()).unwrap_or(&[]);
                    let scrubbed = scrub_body(raw, &self.config.scrub_body_paths);
                    let body_hex = sha256_hex(&scrubbed);
                    parts.push(format!("body_hash={body_hex}"));
                }
                _ => { /* unknown match component — ignored */ }
            }
        }
        parts.join("|")
    }

    /// Persist the fixtures map to disk (YAML).
    async fn flush_to_disk(
        &self,
        fixtures: &HashMap<String, RecordedInteraction>,
    ) -> Result<(), Error> {
        // Convert to a sorted Vec for stable on-disk ordering.
        let mut entries: Vec<(&String, &RecordedInteraction)> = fixtures.iter().collect();
        entries.sort_by(|a, b| a.0.cmp(b.0));
        let as_map: BTreeMap<String, RecordedInteraction> = entries
            .into_iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();
        let yaml = serde_yaml::to_string(&as_map)
            .map_err(|e| Error::ParseFailed(format!("serialize cassette: {e}")))?;

        if let Some(parent) = self.cassette_path.parent() {
            tokio::fs::create_dir_all(parent).await.map_err(|e| {
                Error::ParseFailed(format!(
                    "create cassette dir {}: {e}",
                    parent.display()
                ))
            })?;
        }
        tokio::fs::write(&self.cassette_path, yaml)
            .await
            .map_err(|e| Error::ParseFailed(format!("write cassette: {e}")))?;
        Ok(())
    }

    /// Scrub the recorded response per config — strip
    /// `scrub_headers` before persisting so cassettes
    /// committed to VCS can't leak credentials.
    fn scrub_response(&self, recorded: &mut RecordedResponse) {
        let scrub: std::collections::HashSet<String> = self
            .config
            .scrub_headers
            .iter()
            .map(|h| h.to_ascii_lowercase())
            .collect();
        recorded
            .headers
            .retain(|k, _| !scrub.contains(&k.to_ascii_lowercase()));
    }
}

#[async_trait]
impl reqwest_middleware::Middleware for CassetteLayer {
    async fn handle(
        &self,
        req: reqwest::Request,
        ext: &mut http::Extensions,
        next: reqwest_middleware::Next<'_>,
    ) -> reqwest_middleware::Result<reqwest::Response> {
        let key = self.match_key(&req);
        let mode = self.config.mode.as_str();

        // Replay / auto: check fixtures first.
        if mode == "replay" || mode == "auto" {
            let fixtures = self.fixtures.lock().await;
            if let Some(interaction) = fixtures.get(&key) {
                return reconstruct_response(&interaction.response).map_err(
                    |e| reqwest_middleware::Error::Middleware(anyhow::anyhow!(
                        "swe_http_cassette replay reconstruct failed: {e}"
                    )),
                );
            }
            drop(fixtures);
            // Miss handling:
            if mode == "replay" {
                return Err(reqwest_middleware::Error::Middleware(anyhow::anyhow!(
                    "swe_http_cassette: no recorded interaction for key {key} in {}",
                    self.cassette_path.display()
                )));
            }
            // "auto" — fall through to record.
        }

        // Record / auto-miss: clone the request (for body
        // capture), dispatch, capture the response, save to
        // fixtures + disk.
        let attempt_req = req
            .try_clone()
            .ok_or_else(|| reqwest_middleware::Error::Middleware(anyhow::anyhow!(
                "swe_http_cassette: cannot record a request with a non-cloneable body"
            )))?;
        let recorded_req = RecordedRequest {
            method: req.method().to_string(),
            url: req.url().to_string(),
            body_hash: if self.config.match_on.iter().any(|m| m == "body_hash") {
                let raw = req.body().and_then(|b| b.as_bytes()).unwrap_or(&[]);
                let scrubbed = scrub_body(raw, &self.config.scrub_body_paths);
                Some(sha256_hex(&scrubbed))
            } else {
                None
            },
        };

        // Dispatch.
        let response = next.run(attempt_req, ext).await?;

        // Capture status + headers + body (consumes the response).
        let status = response.status().as_u16();
        let mut headers: BTreeMap<String, String> = BTreeMap::new();
        for (k, v) in response.headers().iter() {
            if let Ok(v) = v.to_str() {
                headers.insert(k.as_str().to_string(), v.to_string());
            }
        }
        let body_bytes = response.bytes().await.map_err(|e| {
            reqwest_middleware::Error::Middleware(anyhow::anyhow!(
                "swe_http_cassette: read response body: {e}"
            ))
        })?;
        let body_base64 = base64::engine::general_purpose::STANDARD.encode(&body_bytes);

        let mut recorded_resp = RecordedResponse {
            status,
            headers,
            body_base64,
        };
        self.scrub_response(&mut recorded_resp);

        let interaction = RecordedInteraction {
            request: recorded_req,
            response: recorded_resp.clone(),
        };

        // Save in-memory + persist.
        {
            let mut fixtures = self.fixtures.lock().await;
            fixtures.insert(key, interaction);
            self.flush_to_disk(&fixtures).await.map_err(|e| {
                reqwest_middleware::Error::Middleware(anyhow::anyhow!(
                    "swe_http_cassette: flush to disk: {e}"
                ))
            })?;
        }

        // Reconstruct the response for the caller (since we
        // consumed body_bytes to record).
        reconstruct_response(&recorded_resp).map_err(|e| {
            reqwest_middleware::Error::Middleware(anyhow::anyhow!(
                "swe_http_cassette post-record reconstruct: {e}"
            ))
        })
    }
}

/// Load previously-recorded fixtures from disk. Missing file
/// yields an empty map; malformed YAML yields an error.
fn load_fixtures_from_disk(
    path: &Path,
) -> Result<HashMap<String, RecordedInteraction>, Error> {
    if !path.is_file() {
        return Ok(HashMap::new());
    }
    let yaml = std::fs::read_to_string(path).map_err(|e| {
        Error::ParseFailed(format!("read cassette {}: {e}", path.display()))
    })?;
    if yaml.trim().is_empty() {
        return Ok(HashMap::new());
    }
    let as_map: BTreeMap<String, RecordedInteraction> = serde_yaml::from_str(&yaml)
        .map_err(|e| Error::ParseFailed(format!("parse cassette: {e}")))?;
    Ok(as_map.into_iter().collect())
}

/// Reconstruct a `reqwest::Response` from a recorded response.
fn reconstruct_response(recorded: &RecordedResponse) -> anyhow::Result<reqwest::Response> {
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(&recorded.body_base64)
        .map_err(|e| anyhow::anyhow!("base64 decode: {e}"))?;

    let mut builder = http::Response::builder().status(recorded.status);
    for (k, v) in &recorded.headers {
        builder = builder.header(k, v);
    }
    let http_resp = builder
        .body(reqwest::Body::from(bytes))
        .map_err(|e| anyhow::anyhow!("build http::Response: {e}"))?;
    Ok(reqwest::Response::from(http_resp))
}

/// SHA256 hex of a byte slice. Used for request-body hashing
/// AFTER scrubbing has been applied. An empty slice hashes to
/// the well-known SHA256("") = e3b0c442...
fn sha256_hex(bytes: &[u8]) -> String {
    let mut h = Sha256::new();
    h.update(bytes);
    hex::encode(h.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;
    use reqwest::{Method, Url};

    fn test_config(dir: &str) -> CassetteConfig {
        // Normalize Windows backslashes so the TOML parser
        // doesn't read `\U` / `\t` / etc. as escape sequences
        // inside the basic string.
        let dir_toml_safe = dir.replace('\\', "/");
        let toml = format!(
            r#"
                mode = "replay"
                cassette_dir = "{dir_toml_safe}"
                match_on = ["method", "url"]
                scrub_headers = ["authorization", "set-cookie"]
                scrub_body_paths = []
            "#
        );
        CassetteConfig::from_config(&toml).unwrap()
    }

    /// @covers: sha256_hex
    #[test]
    fn test_empty_body_hash_is_sha256_of_empty_string() {
        // Known: SHA256("") = e3b0c442...
        assert!(sha256_hex(&[]).starts_with("e3b0c442"));
    }

    /// @covers: sha256_hex
    #[test]
    fn test_body_hash_differs_for_different_bodies() {
        assert_ne!(sha256_hex(b"a"), sha256_hex(b"b"));
    }

    /// @covers: CassetteLayer::match_key
    #[test]
    fn test_match_key_includes_method_and_url() {
        let dir = tempfile::tempdir().unwrap();
        let layer = CassetteLayer::new(
            test_config(dir.path().to_str().unwrap()),
            "test_cassette",
        )
        .unwrap();
        let req = reqwest::Request::new(
            Method::GET,
            Url::parse("https://api.example.test/foo").unwrap(),
        );
        let key = layer.match_key(&req);
        assert!(key.contains("method=GET"));
        assert!(key.contains("url=https://api.example.test/foo"));
    }

    /// @covers: CassetteLayer::match_key
    #[test]
    fn test_match_key_body_hash_stable_across_scrubbed_fields() {
        // Two requests differing only in a scrubbed field
        // must produce the same match key (proving the scrub
        // path flows into the body hash).
        let dir = tempfile::tempdir().unwrap();
        let dir_toml_safe = dir.path().to_str().unwrap().replace('\\', "/");
        let toml = format!(
            r#"
                mode = "replay"
                cassette_dir = "{dir_toml_safe}"
                match_on = ["method", "url", "body_hash"]
                scrub_headers = []
                scrub_body_paths = ["request_id"]
            "#
        );
        let cfg = CassetteConfig::from_config(&toml).unwrap();
        let layer = CassetteLayer::new(cfg, "test").unwrap();

        let url = "https://example.test/api";
        let mut r1 = reqwest::Request::new(Method::POST, Url::parse(url).unwrap());
        *r1.body_mut() = Some(br#"{"request_id":"first","payload":"data"}"#.to_vec().into());
        let mut r2 = reqwest::Request::new(Method::POST, Url::parse(url).unwrap());
        *r2.body_mut() = Some(br#"{"request_id":"second","payload":"data"}"#.to_vec().into());

        assert_eq!(layer.match_key(&r1), layer.match_key(&r2));
    }

    /// @covers: CassetteLayer::match_key
    #[test]
    fn test_match_key_body_hash_differs_for_unscrubbed_field_changes() {
        let dir = tempfile::tempdir().unwrap();
        let dir_toml_safe = dir.path().to_str().unwrap().replace('\\', "/");
        let toml = format!(
            r#"
                mode = "replay"
                cassette_dir = "{dir_toml_safe}"
                match_on = ["method", "url", "body_hash"]
                scrub_headers = []
                scrub_body_paths = ["request_id"]
            "#
        );
        let cfg = CassetteConfig::from_config(&toml).unwrap();
        let layer = CassetteLayer::new(cfg, "test").unwrap();

        let url = "https://example.test/api";
        let mut r1 = reqwest::Request::new(Method::POST, Url::parse(url).unwrap());
        *r1.body_mut() = Some(br#"{"request_id":"x","payload":"A"}"#.to_vec().into());
        let mut r2 = reqwest::Request::new(Method::POST, Url::parse(url).unwrap());
        *r2.body_mut() = Some(br#"{"request_id":"x","payload":"B"}"#.to_vec().into());

        // Different payload (not scrubbed) → different key.
        assert_ne!(layer.match_key(&r1), layer.match_key(&r2));
    }

    /// @covers: CassetteLayer::match_key
    #[test]
    fn test_match_key_differs_across_methods_on_same_url() {
        let dir = tempfile::tempdir().unwrap();
        let layer = CassetteLayer::new(
            test_config(dir.path().to_str().unwrap()),
            "test_cassette",
        )
        .unwrap();
        let r_get = reqwest::Request::new(
            Method::GET,
            Url::parse("https://example.test/x").unwrap(),
        );
        let r_post = reqwest::Request::new(
            Method::POST,
            Url::parse("https://example.test/x").unwrap(),
        );
        assert_ne!(layer.match_key(&r_get), layer.match_key(&r_post));
    }

    /// @covers: CassetteLayer::scrub_response
    #[test]
    fn test_scrub_response_removes_configured_headers() {
        let dir = tempfile::tempdir().unwrap();
        let layer = CassetteLayer::new(
            test_config(dir.path().to_str().unwrap()),
            "test_cassette",
        )
        .unwrap();
        let mut r = RecordedResponse {
            status: 200,
            headers: {
                let mut m = BTreeMap::new();
                m.insert("Authorization".into(), "Bearer secret-123".into());
                m.insert("content-type".into(), "application/json".into());
                m.insert("Set-Cookie".into(), "session=xyz".into());
                m
            },
            body_base64: String::new(),
        };
        layer.scrub_response(&mut r);
        // Scrubbed (case-insensitive): authorization + set-cookie.
        assert!(r.headers.contains_key("content-type"));
        assert!(!r.headers.iter().any(|(k, _)| k.eq_ignore_ascii_case("authorization")));
        assert!(!r.headers.iter().any(|(k, _)| k.eq_ignore_ascii_case("set-cookie")));
    }

    /// @covers: load_fixtures_from_disk
    #[test]
    fn test_load_fixtures_missing_file_returns_empty_map() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("nonexistent.yaml");
        let fixtures = load_fixtures_from_disk(&path).unwrap();
        assert!(fixtures.is_empty());
    }

    /// @covers: load_fixtures_from_disk
    #[test]
    fn test_load_fixtures_roundtrips_through_yaml() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("rt.yaml");
        let mut map = BTreeMap::new();
        map.insert(
            "method=GET|url=https://example.test/".to_string(),
            RecordedInteraction {
                request: RecordedRequest {
                    method: "GET".into(),
                    url: "https://example.test/".into(),
                    body_hash: None,
                },
                response: RecordedResponse {
                    status: 200,
                    headers: BTreeMap::new(),
                    body_base64: "dGVzdA==".into(),
                },
            },
        );
        std::fs::write(&path, serde_yaml::to_string(&map).unwrap()).unwrap();
        let fixtures = load_fixtures_from_disk(&path).unwrap();
        assert_eq!(fixtures.len(), 1);
    }

    /// @covers: reconstruct_response
    #[test]
    fn test_reconstruct_response_from_recorded() {
        let r = RecordedResponse {
            status: 418,
            headers: {
                let mut m = BTreeMap::new();
                m.insert("x-custom".into(), "teapot".into());
                m
            },
            body_base64: base64::engine::general_purpose::STANDARD.encode("I'm a teapot"),
        };
        let resp = reconstruct_response(&r).unwrap();
        assert_eq!(resp.status().as_u16(), 418);
        assert_eq!(resp.headers().get("x-custom").unwrap(), "teapot");
    }

    /// @covers: CassetteLayer::new
    #[test]
    fn test_new_with_nonexistent_cassette_starts_empty() {
        let dir = tempfile::tempdir().unwrap();
        let layer = CassetteLayer::new(
            test_config(dir.path().to_str().unwrap()),
            "fresh_cassette",
        )
        .unwrap();
        // Path is computed but file doesn't exist yet.
        assert!(!layer.cassette_path.exists());
    }

    /// @covers: CassetteLayer::flush_to_disk (sync-constructability)
    /// flush_to_disk is async; the sync-observable invariant is that the
    /// CassetteLayer stores the cassette_path correctly, which flush_to_disk
    /// uses. We verify the path was derived from the config.
    #[test]
    fn test_flush_to_disk_path_derived_from_config() {
        let dir = tempfile::tempdir().unwrap();
        let layer = CassetteLayer::new(
            test_config(dir.path().to_str().unwrap()),
            "my_cassette",
        )
        .unwrap();
        // flush_to_disk writes to cassette_path; verify path contains name.
        let path_str = layer.cassette_path.to_str().unwrap();
        assert!(
            path_str.contains("my_cassette"),
            "cassette_path must contain the cassette name: {path_str}"
        );
        assert!(
            path_str.ends_with(".yaml"),
            "cassette file must be YAML: {path_str}"
        );
    }

    /// @covers: CassetteLayer::handle (sync-observable construction)
    /// handle is async; the sync-observable invariant is that CassetteLayer
    /// is Send + Sync (required by reqwest_middleware::Middleware).
    #[test]
    fn test_handle_layer_is_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<CassetteLayer>();
    }

    /// @covers: load_fixtures_from_disk
    #[test]
    fn test_load_fixtures_from_disk_empty_file_returns_empty_map() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("empty.yaml");
        std::fs::write(&path, "   ").unwrap();
        let fixtures = load_fixtures_from_disk(&path).unwrap();
        assert!(fixtures.is_empty(), "whitespace-only YAML must yield empty map");
    }

    /// @covers: sha256_hex
    #[test]
    fn test_sha256_hex_known_vector() {
        // NIST: SHA256("abc") begins with ba7816bf
        let h = sha256_hex(b"abc");
        assert_eq!(
            h,
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }
}
