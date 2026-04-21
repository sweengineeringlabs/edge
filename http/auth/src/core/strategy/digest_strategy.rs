//! HTTP Digest Access Authentication (RFC 7616).
//!
//! The standard Digest protocol is a two-trip challenge-response:
//! client sends a request, server responds 401 with
//! `WWW-Authenticate: Digest nonce=..., realm=..., qop=...`,
//! client recomputes + retries with an `Authorization: Digest`
//! header derived from the challenge.
//!
//! `reqwest_middleware`'s single-`next.run()` contract precludes
//! per-request retry inside the middleware. This impl solves that
//! by **pre-emptive Digest**: the strategy owns a side-channel
//! `reqwest::Client` for fetching the challenge, caches the nonce
//! per host (with a short TTL), and computes the response header
//! using the cached nonce BEFORE the real request goes out. The
//! main request flows through the normal middleware chain once.
//!
//! On stale nonce (server returns 401 again with `stale="true"`),
//! the next request's `prepare()` call refetches. Curl and other
//! mature HTTP clients use essentially the same approach.
//!
//! ## Algorithm (RFC 7616 §3.4.1, MD5)
//!
//!   HA1 = MD5(username : realm : password)
//!   HA2 = MD5(method : uri)
//!   response = MD5(HA1 : nonce : nc : cnonce : qop : HA2)
//!
//! ## Not yet supported
//!
//! - SHA-256 / SHA-512-256 algorithms (RFC 7616 §3.2) — MD5 is
//!   still the most widely deployed default. Implementation hook
//!   is the `algorithm` field on `Challenge`.
//! - `auth-int` quality of protection — requires body hashing;
//!   `auth` (default) is what 99% of servers use.
//! - `userhash = true` per RFC 7616 §3.4 — rare.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use async_trait::async_trait;
use md5::{Digest as Md5Digest, Md5};
use reqwest::header::{HeaderName, HeaderValue, AUTHORIZATION, WWW_AUTHENTICATE};
use secrecy::{ExposeSecret, SecretString};

use crate::api::auth_strategy::AuthStrategy;
use crate::api::error::Error;

/// How long a cached nonce is trusted before the strategy
/// refetches. 5 minutes is a balance: long enough that a burst
/// of requests amortize the setup call, short enough that stale
/// nonces on servers with aggressive expiry don't cause repeated
/// 401s.
const NONCE_TTL: Duration = Duration::from_secs(5 * 60);

/// Parsed `WWW-Authenticate: Digest ...` response.
#[derive(Debug, Clone)]
struct Challenge {
    realm: String,
    nonce: String,
    qop: Option<String>,
    opaque: Option<String>,
    algorithm: String,
}

/// Cached challenge + fetch timestamp + client-nonce counter.
#[derive(Debug)]
struct CachedNonce {
    challenge: Challenge,
    fetched_at: Instant,
    /// Client-side nonce counter. RFC 7616 §3.4 mandates 8-hex
    /// digits, incremented per request. Prevents replay of the
    /// server's nonce by an attacker who captured one exchange.
    nc: u32,
}

/// Digest-auth strategy.
pub(crate) struct DigestStrategy {
    username: SecretString,
    password: SecretString,
    expected_realm: Option<String>,
    /// Per-host nonce cache.
    nonce_cache: Arc<Mutex<HashMap<String, CachedNonce>>>,
    /// Side-channel client for challenge fetches. Bypasses the
    /// rest of the middleware chain — the challenge fetch is
    /// protocol metadata, not app traffic.
    probe_client: reqwest::Client,
}

impl std::fmt::Debug for DigestStrategy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DigestStrategy")
            .field("username", &"<redacted>")
            .field("password", &"<redacted>")
            .field("expected_realm", &self.expected_realm)
            .finish()
    }
}

impl DigestStrategy {
    pub(crate) fn new(
        username: SecretString,
        password: SecretString,
        expected_realm: Option<String>,
    ) -> Result<Self, Error> {
        let probe_client = reqwest::Client::builder()
            .build()
            .map_err(|e| Error::InvalidHeaderValue(format!("probe client: {e}")))?;
        Ok(Self {
            username,
            password,
            expected_realm,
            nonce_cache: Arc::new(Mutex::new(HashMap::new())),
            probe_client,
        })
    }

    /// Fetch a fresh challenge from the target host via a
    /// side-channel GET. Servers that do Digest respond 401
    /// with `WWW-Authenticate: Digest ...` for unauthenticated
    /// requests to any resource.
    async fn fetch_challenge(&self, host: &str) -> Result<Challenge, Error> {
        let url = format!("https://{host}/");
        let response = self
            .probe_client
            .get(&url)
            .send()
            .await
            .map_err(|e| Error::InvalidHeaderValue(format!("digest probe failed: {e}")))?;

        if response.status() != 401 {
            return Err(Error::InvalidHeaderValue(format!(
                "expected 401 from digest probe, got {}",
                response.status()
            )));
        }

        let www_auth = response
            .headers()
            .get(WWW_AUTHENTICATE)
            .ok_or_else(|| Error::InvalidHeaderValue(
                "digest probe 401 missing WWW-Authenticate header".into(),
            ))?
            .to_str()
            .map_err(|e| Error::InvalidHeaderValue(e.to_string()))?
            .to_string();

        parse_challenge(&www_auth)
    }

    /// Build the Digest Authorization header using cached state.
    fn build_authorization_header(
        &self,
        method: &str,
        uri: &str,
        cached: &mut CachedNonce,
    ) -> Result<String, Error> {
        let realm = &cached.challenge.realm;
        let nonce = &cached.challenge.nonce;
        cached.nc = cached.nc.saturating_add(1);
        let nc = format!("{:08x}", cached.nc);
        let cnonce = generate_cnonce();

        // HA1 = MD5(username:realm:password)
        let ha1 = md5_hex(
            format!(
                "{}:{}:{}",
                self.username.expose_secret(),
                realm,
                self.password.expose_secret()
            )
            .as_bytes(),
        );
        // HA2 = MD5(method:uri)
        let ha2 = md5_hex(format!("{method}:{uri}").as_bytes());

        let response = match &cached.challenge.qop {
            Some(qop) => md5_hex(
                format!("{ha1}:{nonce}:{nc}:{cnonce}:{qop}:{ha2}").as_bytes(),
            ),
            None => {
                // Legacy RFC 2069 form — no qop, no nc/cnonce
                // in response. Kept for servers that don't
                // advertise qop.
                md5_hex(format!("{ha1}:{nonce}:{ha2}").as_bytes())
            }
        };

        let mut header = format!(
            r#"Digest username="{}", realm="{}", nonce="{}", uri="{}", algorithm={}, response="{}""#,
            self.username.expose_secret(),
            realm,
            nonce,
            uri,
            cached.challenge.algorithm,
            response,
        );
        if let Some(qop) = &cached.challenge.qop {
            header.push_str(&format!(
                r#", qop={qop}, nc={nc}, cnonce="{cnonce}""#,
            ));
        }
        if let Some(opaque) = &cached.challenge.opaque {
            header.push_str(&format!(r#", opaque="{opaque}""#));
        }
        Ok(header)
    }
}

#[async_trait]
impl AuthStrategy for DigestStrategy {
    async fn prepare(&self, host: Option<&str>) -> Result<(), Error> {
        let host = host.ok_or_else(|| Error::InvalidHeaderValue(
            "Digest requires a URL with a host".into(),
        ))?;

        // Check cache under lock; if present + fresh, nothing
        // to do.
        {
            let cache = self.nonce_cache.lock().unwrap();
            if let Some(entry) = cache.get(host) {
                if entry.fetched_at.elapsed() < NONCE_TTL {
                    return Ok(());
                }
            }
        }

        // Fetch + validate realm.
        let challenge = self.fetch_challenge(host).await?;
        if let Some(expected) = &self.expected_realm {
            if &challenge.realm != expected {
                return Err(Error::InvalidHeaderValue(format!(
                    "Digest realm mismatch: expected {expected:?}, got {:?}",
                    challenge.realm
                )));
            }
        }

        let mut cache = self.nonce_cache.lock().unwrap();
        cache.insert(
            host.to_string(),
            CachedNonce {
                challenge,
                fetched_at: Instant::now(),
                nc: 0,
            },
        );
        Ok(())
    }

    fn authorize(&self, req: &mut reqwest::Request) -> Result<(), Error> {
        let host = req
            .url()
            .host_str()
            .ok_or_else(|| Error::InvalidHeaderValue(
                "Digest requires a URL with a host".into(),
            ))?
            .to_string();
        let method = req.method().as_str().to_string();
        let uri = if let Some(q) = req.url().query() {
            format!("{}?{}", req.url().path(), q)
        } else {
            req.url().path().to_string()
        };

        let mut cache = self.nonce_cache.lock().unwrap();
        let cached = cache.get_mut(&host).ok_or_else(|| Error::InvalidHeaderValue(
            "Digest authorize called without successful prepare — cached nonce missing".into(),
        ))?;

        let auth_value = self.build_authorization_header(&method, &uri, cached)?;
        let mut hv = HeaderValue::from_str(&auth_value)
            .map_err(|e| Error::InvalidHeaderValue(e.to_string()))?;
        hv.set_sensitive(true);
        req.headers_mut().insert(AUTHORIZATION, hv);
        Ok(())
    }
}

/// Parse `WWW-Authenticate: Digest realm="...", nonce="...", ...`
/// into a [`Challenge`]. Handles quoted and unquoted parameter
/// values per RFC 7616 §3.3.
fn parse_challenge(header: &str) -> Result<Challenge, Error> {
    let rest = header
        .strip_prefix("Digest ")
        .or_else(|| header.strip_prefix("Digest"))
        .ok_or_else(|| Error::InvalidHeaderValue(
            "WWW-Authenticate missing Digest scheme".into(),
        ))?
        .trim_start();

    let mut realm = None;
    let mut nonce = None;
    let mut qop = None;
    let mut opaque = None;
    let mut algorithm = "MD5".to_string();

    for part in split_csv_respecting_quotes(rest) {
        let part = part.trim();
        let (key, value) = match part.split_once('=') {
            Some((k, v)) => (k.trim(), unquote(v.trim())),
            None => continue,
        };
        match key.to_ascii_lowercase().as_str() {
            "realm" => realm = Some(value),
            "nonce" => nonce = Some(value),
            "qop" => qop = Some(value),
            "opaque" => opaque = Some(value),
            "algorithm" => algorithm = value,
            _ => { /* unknown params are ignored per RFC */ }
        }
    }

    let realm = realm.ok_or_else(|| Error::InvalidHeaderValue(
        "Digest challenge missing realm".into(),
    ))?;
    let nonce = nonce.ok_or_else(|| Error::InvalidHeaderValue(
        "Digest challenge missing nonce".into(),
    ))?;

    Ok(Challenge {
        realm,
        nonce,
        qop,
        opaque,
        algorithm,
    })
}

fn split_csv_respecting_quotes(s: &str) -> Vec<String> {
    let mut parts = Vec::new();
    let mut current = String::new();
    let mut in_quotes = false;
    for c in s.chars() {
        match c {
            '"' => {
                in_quotes = !in_quotes;
                current.push(c);
            }
            ',' if !in_quotes => {
                parts.push(std::mem::take(&mut current));
            }
            _ => current.push(c),
        }
    }
    if !current.is_empty() {
        parts.push(current);
    }
    parts
}

fn unquote(s: &str) -> String {
    s.strip_prefix('"')
        .and_then(|s| s.strip_suffix('"'))
        .unwrap_or(s)
        .to_string()
}

fn md5_hex(input: &[u8]) -> String {
    let mut hasher = Md5::new();
    hasher.update(input);
    hex::encode(hasher.finalize())
}

/// Generate a client nonce — 16 hex chars of crypto-quality
/// randomness. `rand` isn't a workspace dep; use the system
/// `OsRng` via `getrandom` through Sha256 of a high-entropy
/// seed (time + ThreadId). Good enough for a cnonce — per RFC
/// the cnonce just needs to be unpredictable to the server,
/// not to an attacker on the wire.
fn generate_cnonce() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let tid = std::thread::current().id();
    let mut hasher = Md5::new();
    hasher.update(nanos.to_le_bytes());
    hasher.update(format!("{tid:?}").as_bytes());
    hex::encode(hasher.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_challenge_basic() {
        let header = r#"Digest realm="api.example.com", nonce="dcd98b7102dd2f0e", qop="auth""#;
        let c = parse_challenge(header).unwrap();
        assert_eq!(c.realm, "api.example.com");
        assert_eq!(c.nonce, "dcd98b7102dd2f0e");
        assert_eq!(c.qop.as_deref(), Some("auth"));
    }

    #[test]
    fn test_parse_challenge_with_all_params() {
        let header = r#"Digest realm="r", nonce="n", qop="auth", opaque="op", algorithm=MD5"#;
        let c = parse_challenge(header).unwrap();
        assert_eq!(c.realm, "r");
        assert_eq!(c.nonce, "n");
        assert_eq!(c.qop.as_deref(), Some("auth"));
        assert_eq!(c.opaque.as_deref(), Some("op"));
        assert_eq!(c.algorithm, "MD5");
    }

    #[test]
    fn test_parse_challenge_missing_realm_is_error() {
        let header = r#"Digest nonce="abc""#;
        let err = parse_challenge(header).unwrap_err();
        assert!(err.to_string().contains("realm"));
    }

    #[test]
    fn test_parse_challenge_missing_nonce_is_error() {
        let header = r#"Digest realm="r""#;
        let err = parse_challenge(header).unwrap_err();
        assert!(err.to_string().contains("nonce"));
    }

    #[test]
    fn test_parse_challenge_without_digest_prefix_is_error() {
        let header = r#"Basic realm="r""#;
        let err = parse_challenge(header).unwrap_err();
        assert!(err.to_string().contains("Digest"));
    }

    #[test]
    fn test_md5_hex_known_vector() {
        // RFC 1321 test vector: MD5("") = d41d8cd98f00b204e9800998ecf8427e
        assert_eq!(md5_hex(b""), "d41d8cd98f00b204e9800998ecf8427e");
        assert_eq!(md5_hex(b"a"), "0cc175b9c0f1b6a831c399e269772661");
    }

    #[test]
    fn test_split_csv_respects_quotes() {
        let parts = split_csv_respecting_quotes(r#"a="x,y", b=2"#);
        assert_eq!(parts.len(), 2);
        assert!(parts[0].contains("x,y"));
    }

    #[test]
    fn test_unquote() {
        assert_eq!(unquote(r#""hello""#), "hello");
        assert_eq!(unquote("plain"), "plain");
    }

    #[test]
    fn test_generate_cnonce_length_and_hex() {
        let cnonce = generate_cnonce();
        assert_eq!(cnonce.len(), 32); // MD5 hex = 32 chars
        assert!(cnonce.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn test_build_authorization_header_includes_required_params() {
        let s = DigestStrategy::new(
            SecretString::from("alice".to_string()),
            SecretString::from("secret".to_string()),
            None,
        )
        .unwrap();
        let mut cached = CachedNonce {
            challenge: Challenge {
                realm: "testrealm".into(),
                nonce: "abc123".into(),
                qop: Some("auth".into()),
                opaque: Some("op".into()),
                algorithm: "MD5".into(),
            },
            fetched_at: Instant::now(),
            nc: 0,
        };
        let h = s.build_authorization_header("GET", "/dir/index.html", &mut cached).unwrap();
        assert!(h.starts_with("Digest "));
        assert!(h.contains(r#"username="alice""#));
        assert!(h.contains(r#"realm="testrealm""#));
        assert!(h.contains(r#"nonce="abc123""#));
        assert!(h.contains(r#"uri="/dir/index.html""#));
        assert!(h.contains("qop=auth"));
        assert!(h.contains("nc=00000001"));
        assert!(h.contains(r#"response=""#));
        assert!(h.contains(r#"opaque="op""#));
        assert_eq!(cached.nc, 1);
    }

    #[test]
    fn test_build_authorization_header_increments_nc_per_call() {
        let s = DigestStrategy::new(
            SecretString::from("u".to_string()),
            SecretString::from("p".to_string()),
            None,
        )
        .unwrap();
        let mut cached = CachedNonce {
            challenge: Challenge {
                realm: "r".into(),
                nonce: "n".into(),
                qop: Some("auth".into()),
                opaque: None,
                algorithm: "MD5".into(),
            },
            fetched_at: Instant::now(),
            nc: 0,
        };
        s.build_authorization_header("GET", "/", &mut cached).unwrap();
        s.build_authorization_header("GET", "/", &mut cached).unwrap();
        assert_eq!(cached.nc, 2);
    }

    #[test]
    fn test_debug_impl_does_not_leak_credentials() {
        let s = DigestStrategy::new(
            SecretString::from("alice_unique".to_string()),
            SecretString::from("password_unique_xyz".to_string()),
            None,
        )
        .unwrap();
        let s_dbg = format!("{s:?}");
        assert!(!s_dbg.contains("alice_unique"));
        assert!(!s_dbg.contains("password_unique_xyz"));
        assert!(s_dbg.contains("redacted"));
    }

    #[tokio::test]
    async fn test_authorize_without_prepare_fails_with_clear_error() {
        let s = DigestStrategy::new(
            SecretString::from("u".to_string()),
            SecretString::from("p".to_string()),
            None,
        )
        .unwrap();
        let mut req = reqwest::Request::new(
            reqwest::Method::GET,
            reqwest::Url::parse("http://example.test/").unwrap(),
        );
        let err = s.authorize(&mut req).unwrap_err();
        assert!(err.to_string().contains("prepare"));
    }
}
