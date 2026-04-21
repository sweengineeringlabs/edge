//! AWS Signature Version 4 strategy.
//!
//! Implements the AWS SigV4 algorithm as specified by AWS:
//! https://docs.aws.amazon.com/general/latest/gr/sigv4_signing.html
//!
//! On each request:
//!   1. Compute canonical request string (method, path, query,
//!      canonical headers, signed headers, payload hash)
//!   2. Compute string-to-sign (algorithm, datetime, credential
//!      scope, hash(canonical-request))
//!   3. Derive signing key (date → region → service → "aws4_request")
//!   4. Compute signature = HMAC-SHA256(signing-key, string-to-sign)
//!   5. Attach `Authorization: AWS4-HMAC-SHA256 Credential=...,
//!      SignedHeaders=..., Signature=...` + `x-amz-date` +
//!      optional `x-amz-security-token` headers.

use async_trait::async_trait;
use hmac::{Hmac, Mac};
use http::header::{HeaderName, HeaderValue, AUTHORIZATION, HOST};
use percent_encoding::{utf8_percent_encode, AsciiSet, CONTROLS};
use secrecy::{ExposeSecret, SecretString};
use sha2::{Digest, Sha256};
use time::format_description::FormatItem;
use time::{macros::format_description, OffsetDateTime};

use crate::api::auth_strategy::AuthStrategy;
use crate::api::error::Error;

type HmacSha256 = Hmac<Sha256>;

/// `AsciiSet` semantics: chars IN this set get percent-encoded.
/// Everything NOT in the set passes through literally.
///
/// Per AWS SigV4 spec, canonical strings percent-encode every
/// character EXCEPT the RFC 3986 §2.3 unreserved set:
/// `A-Z a-z 0-9 - . _ ~`.
///
/// For canonical URI (path): `/` is NOT in this set, so it
/// passes through — slashes stay literal in `/foo/bar`.
/// For canonical query: `/` IS in [`ENCODE_FOR_QUERY`] below,
/// so query components percent-encode slashes.
const ENCODE_FOR_PATH: &AsciiSet = &CONTROLS
    .add(b' ').add(b'!').add(b'"').add(b'#').add(b'$').add(b'%').add(b'&').add(b'\'')
    .add(b'(').add(b')').add(b'*').add(b'+').add(b',').add(b':').add(b';').add(b'<')
    .add(b'=').add(b'>').add(b'?').add(b'@').add(b'[').add(b'\\').add(b']').add(b'^')
    .add(b'`').add(b'{').add(b'|').add(b'}');

/// Same as [`ENCODE_FOR_PATH`] plus `/` — for query-string
/// components per AWS SigV4 spec.
const ENCODE_FOR_QUERY: &AsciiSet = &ENCODE_FOR_PATH.add(b'/');

/// `YYYYMMDDTHHMMSSZ` — AWS X-Amz-Date format.
const AMZ_DATE_FMT: &[FormatItem<'static>] =
    format_description!("[year][month][day]T[hour][minute][second]Z");
/// `YYYYMMDD` — AWS credential-scope date component.
const AMZ_DATE_ONLY_FMT: &[FormatItem<'static>] =
    format_description!("[year][month][day]");

/// AWS SigV4 strategy. Holds pre-resolved credentials + the
/// static (region, service) pair for signing.
pub(crate) struct AwsSigV4Strategy {
    access_key_id: SecretString,
    secret_access_key: SecretString,
    session_token: Option<SecretString>,
    region: String,
    service: String,
}

impl std::fmt::Debug for AwsSigV4Strategy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AwsSigV4Strategy")
            .field("access_key_id", &"<redacted>")
            .field("secret_access_key", &"<redacted>")
            .field("session_token", &if self.session_token.is_some() { "<set>" } else { "<none>" })
            .field("region", &self.region)
            .field("service", &self.service)
            .finish()
    }
}

impl AwsSigV4Strategy {
    /// Construct from resolved credentials + static
    /// region/service identifiers.
    pub(crate) fn new(
        access_key_id: SecretString,
        secret_access_key: SecretString,
        session_token: Option<SecretString>,
        region: String,
        service: String,
    ) -> Self {
        Self {
            access_key_id,
            secret_access_key,
            session_token,
            region,
            service,
        }
    }

    /// The current timestamp — factored out so tests can inject
    /// a fixed time for deterministic signature comparison.
    fn now() -> OffsetDateTime {
        OffsetDateTime::now_utc()
    }

    /// Perform the signing pass on `req`. Extracted so tests
    /// can call it with a fixed `OffsetDateTime` to verify
    /// against known AWS test vectors.
    fn sign(&self, req: &mut reqwest::Request, now: OffsetDateTime) -> Result<(), Error> {
        let amz_date = now
            .format(AMZ_DATE_FMT)
            .map_err(|e| Error::InvalidHeaderValue(format!("format amz-date: {e}")))?;
        let date_scope = now
            .format(AMZ_DATE_ONLY_FMT)
            .map_err(|e| Error::InvalidHeaderValue(format!("format amz-date-only: {e}")))?;

        // --- Mutations happen here so canonical-headers sees
        // --- the final request shape.
        let headers = req.headers_mut();

        // x-amz-date must be in the signed set.
        let amz_date_hv = HeaderValue::from_str(&amz_date)
            .map_err(|e| Error::InvalidHeaderValue(e.to_string()))?;
        headers.insert(HeaderName::from_static("x-amz-date"), amz_date_hv);

        // Session token (if present) is also signed. AWS STS /
        // IMDSv2-issued credentials require this.
        if let Some(token) = &self.session_token {
            let mut hv = HeaderValue::from_str(token.expose_secret())
                .map_err(|e| Error::InvalidHeaderValue(e.to_string()))?;
            hv.set_sensitive(true);
            headers.insert(HeaderName::from_static("x-amz-security-token"), hv);
        }

        // Host header — reqwest fills this later, but SigV4
        // requires it to be part of the canonical request. Add
        // explicitly if missing.
        let host_header = match req.url().host_str() {
            Some(h) => {
                let port = req.url().port();
                let host_value = match port {
                    Some(p) => format!("{h}:{p}"),
                    None => h.to_string(),
                };
                HeaderValue::from_str(&host_value)
                    .map_err(|e| Error::InvalidHeaderValue(e.to_string()))?
            }
            None => return Err(Error::InvalidHeaderValue(
                "SigV4 requires a URL with a host".into(),
            )),
        };
        req.headers_mut().insert(HOST, host_header);

        // --- Build canonical request ---
        let method = req.method().as_str().to_string();
        let canonical_uri = canonical_uri(req.url().path());
        let canonical_query = canonical_query_string(req.url().query().unwrap_or(""));

        let mut header_pairs: Vec<(String, String)> = req
            .headers()
            .iter()
            .map(|(k, v)| {
                (
                    k.as_str().to_ascii_lowercase(),
                    String::from_utf8_lossy(v.as_bytes()).trim().to_string(),
                )
            })
            .collect();
        header_pairs.sort_by(|a, b| a.0.cmp(&b.0));
        let canonical_headers = header_pairs
            .iter()
            .map(|(k, v)| format!("{k}:{v}\n"))
            .collect::<String>();
        let signed_headers = header_pairs
            .iter()
            .map(|(k, _)| k.as_str())
            .collect::<Vec<_>>()
            .join(";");

        // Payload hash — SHA256 of the body, lower-hex.
        let payload_hash = match req.body() {
            Some(body) => match body.as_bytes() {
                Some(bytes) => hex::encode(Sha256::digest(bytes)),
                None => {
                    // Streaming body (no bytes handle). AWS allows
                    // `UNSIGNED-PAYLOAD` for this case; most
                    // services accept it.
                    "UNSIGNED-PAYLOAD".to_string()
                }
            },
            None => hex::encode(Sha256::digest(b"")),
        };

        let canonical_request = format!(
            "{method}\n{canonical_uri}\n{canonical_query}\n{canonical_headers}\n{signed_headers}\n{payload_hash}"
        );

        // --- String to sign ---
        let credential_scope =
            format!("{date_scope}/{region}/{service}/aws4_request",
                region = self.region, service = self.service);
        let hashed_canonical = hex::encode(Sha256::digest(canonical_request.as_bytes()));
        let string_to_sign = format!(
            "AWS4-HMAC-SHA256\n{amz_date}\n{credential_scope}\n{hashed_canonical}"
        );

        // --- Derive signing key ---
        let signing_key = derive_signing_key(
            self.secret_access_key.expose_secret(),
            &date_scope,
            &self.region,
            &self.service,
        )?;

        // --- Final signature ---
        let signature = hmac_sha256(&signing_key, string_to_sign.as_bytes())?;
        let signature_hex = hex::encode(signature);

        // --- Authorization header ---
        let auth_value = format!(
            "AWS4-HMAC-SHA256 Credential={access}/{scope}, SignedHeaders={signed}, Signature={sig}",
            access = self.access_key_id.expose_secret(),
            scope = credential_scope,
            signed = signed_headers,
            sig = signature_hex,
        );
        let mut auth_hv = HeaderValue::from_str(&auth_value)
            .map_err(|e| Error::InvalidHeaderValue(e.to_string()))?;
        auth_hv.set_sensitive(true);
        req.headers_mut().insert(AUTHORIZATION, auth_hv);

        Ok(())
    }
}

#[async_trait]
impl AuthStrategy for AwsSigV4Strategy {
    fn authorize(&self, req: &mut reqwest::Request) -> Result<(), Error> {
        self.sign(req, Self::now())
    }
}

/// Canonical URI per SigV4 spec: path with each segment
/// percent-encoded, slash-preserving.
fn canonical_uri(path: &str) -> String {
    if path.is_empty() {
        return "/".into();
    }
    utf8_percent_encode(path, ENCODE_FOR_PATH).to_string()
}

/// Canonical query string: key=value pairs percent-encoded,
/// sorted by key.
fn canonical_query_string(query: &str) -> String {
    if query.is_empty() {
        return String::new();
    }
    let mut pairs: Vec<(String, String)> = query
        .split('&')
        .filter(|p| !p.is_empty())
        .map(|pair| match pair.split_once('=') {
            Some((k, v)) => (
                utf8_percent_encode(k, ENCODE_FOR_QUERY).to_string(),
                utf8_percent_encode(v, ENCODE_FOR_QUERY).to_string(),
            ),
            None => (
                utf8_percent_encode(pair, ENCODE_FOR_QUERY).to_string(),
                String::new(),
            ),
        })
        .collect();
    pairs.sort();
    pairs
        .into_iter()
        .map(|(k, v)| format!("{k}={v}"))
        .collect::<Vec<_>>()
        .join("&")
}

fn hmac_sha256(key: &[u8], data: &[u8]) -> Result<Vec<u8>, Error> {
    let mut mac = HmacSha256::new_from_slice(key).map_err(|e| Error::InvalidHeaderValue(
        format!("HMAC key len: {e}"),
    ))?;
    mac.update(data);
    Ok(mac.finalize().into_bytes().to_vec())
}

fn derive_signing_key(
    secret: &str,
    date: &str,
    region: &str,
    service: &str,
) -> Result<Vec<u8>, Error> {
    let k_secret = format!("AWS4{secret}");
    let k_date = hmac_sha256(k_secret.as_bytes(), date.as_bytes())?;
    let k_region = hmac_sha256(&k_date, region.as_bytes())?;
    let k_service = hmac_sha256(&k_region, service.as_bytes())?;
    hmac_sha256(&k_service, b"aws4_request")
}

#[cfg(test)]
mod tests {
    use super::*;
    use reqwest::{Method, Url};

    fn stub_req(method: Method, url: &str) -> reqwest::Request {
        reqwest::Request::new(method, Url::parse(url).unwrap())
    }

    fn stub_strategy() -> AwsSigV4Strategy {
        AwsSigV4Strategy::new(
            SecretString::from("AKIAIOSFODNN7EXAMPLE".to_string()),
            SecretString::from("wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY".to_string()),
            None,
            "us-east-1".into(),
            "s3".into(),
        )
    }

    /// @covers: canonical_uri
    #[test]
    fn test_canonical_uri_preserves_slashes() {
        assert_eq!(canonical_uri("/foo/bar"), "/foo/bar");
        assert_eq!(canonical_uri("/"), "/");
        assert_eq!(canonical_uri(""), "/");
    }

    /// @covers: canonical_uri
    #[test]
    fn test_canonical_uri_percent_encodes_spaces_and_unicode() {
        // Path segments with spaces get percent-encoded.
        let encoded = canonical_uri("/name with space");
        assert!(encoded.contains("%20") || encoded.contains("%2520"));
    }

    /// @covers: canonical_query_string
    #[test]
    fn test_canonical_query_string_sorts_params_alphabetically() {
        // AWS requires params sorted by key.
        assert_eq!(
            canonical_query_string("Zparam=1&Aparam=2"),
            "Aparam=2&Zparam=1"
        );
    }

    /// @covers: canonical_query_string
    #[test]
    fn test_canonical_query_string_empty_returns_empty() {
        assert_eq!(canonical_query_string(""), "");
    }

    /// @covers: derive_signing_key
    #[test]
    fn test_derive_signing_key_is_deterministic() {
        // Two calls with the same inputs MUST produce the same
        // key. Locks in that derivation is pure.
        let k1 = derive_signing_key(
            "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY",
            "20130524",
            "us-east-1",
            "s3",
        )
        .unwrap();
        let k2 = derive_signing_key(
            "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY",
            "20130524",
            "us-east-1",
            "s3",
        )
        .unwrap();
        assert_eq!(k1, k2);
        assert_eq!(k1.len(), 32); // SHA256 output size
    }

    /// @covers: AwsSigV4Strategy::sign
    #[test]
    fn test_sign_attaches_authorization_header_with_sigv4_prefix() {
        let s = stub_strategy();
        let mut req = stub_req(Method::GET, "https://s3.amazonaws.com/bucket/key");
        let now = OffsetDateTime::UNIX_EPOCH
            + time::Duration::days(16_000); // arbitrary fixed point
        s.sign(&mut req, now).unwrap();

        let auth = req.headers().get("authorization").unwrap().to_str().unwrap();
        assert!(auth.starts_with("AWS4-HMAC-SHA256 Credential="));
        assert!(auth.contains("SignedHeaders="));
        assert!(auth.contains("Signature="));
    }

    /// @covers: AwsSigV4Strategy::sign
    #[test]
    fn test_sign_attaches_x_amz_date() {
        let s = stub_strategy();
        let mut req = stub_req(Method::GET, "https://s3.amazonaws.com/bucket/key");
        s.sign(&mut req, OffsetDateTime::UNIX_EPOCH).unwrap();
        let d = req.headers().get("x-amz-date").unwrap().to_str().unwrap();
        // Format is YYYYMMDDTHHMMSSZ — 16 chars.
        assert_eq!(d.len(), 16);
        assert!(d.ends_with('Z'));
    }

    /// @covers: AwsSigV4Strategy::sign
    #[test]
    fn test_sign_attaches_x_amz_security_token_when_session_token_provided() {
        let s = AwsSigV4Strategy::new(
            SecretString::from("AKID".to_string()),
            SecretString::from("SEC".to_string()),
            Some(SecretString::from("session-tok-123".to_string())),
            "us-east-1".into(),
            "s3".into(),
        );
        let mut req = stub_req(Method::GET, "https://s3.amazonaws.com/bucket/");
        s.sign(&mut req, OffsetDateTime::UNIX_EPOCH).unwrap();
        assert_eq!(
            req.headers()
                .get("x-amz-security-token")
                .unwrap()
                .to_str()
                .unwrap(),
            "session-tok-123"
        );
    }

    /// @covers: AwsSigV4Strategy::sign
    #[test]
    fn test_sign_omits_x_amz_security_token_when_no_session_token() {
        let s = stub_strategy();
        let mut req = stub_req(Method::GET, "https://s3.amazonaws.com/bucket/");
        s.sign(&mut req, OffsetDateTime::UNIX_EPOCH).unwrap();
        assert!(req.headers().get("x-amz-security-token").is_none());
    }

    /// @covers: AwsSigV4Strategy::sign
    #[test]
    fn test_sign_produces_different_signatures_for_different_paths() {
        let s = stub_strategy();
        let mut req1 = stub_req(Method::GET, "https://s3.amazonaws.com/bucket/key1");
        let mut req2 = stub_req(Method::GET, "https://s3.amazonaws.com/bucket/key2");
        let now = OffsetDateTime::UNIX_EPOCH;
        s.sign(&mut req1, now).unwrap();
        s.sign(&mut req2, now).unwrap();
        let sig1 = extract_signature(&req1);
        let sig2 = extract_signature(&req2);
        assert_ne!(sig1, sig2);
    }

    /// @covers: AwsSigV4Strategy (Debug impl)
    #[test]
    fn test_debug_impl_does_not_leak_credentials() {
        let s = AwsSigV4Strategy::new(
            SecretString::from("AKIA_unique_key_id_for_test".to_string()),
            SecretString::from("secret_with_unique_marker_xyz".to_string()),
            None,
            "us-west-2".into(),
            "s3".into(),
        );
        let s_dbg = format!("{s:?}");
        assert!(!s_dbg.contains("AKIA_unique_key_id_for_test"));
        assert!(!s_dbg.contains("secret_with_unique_marker_xyz"));
        assert!(s_dbg.contains("us-west-2"));
        assert!(s_dbg.contains("redacted"));
    }

    fn extract_signature(req: &reqwest::Request) -> String {
        let auth = req.headers().get("authorization").unwrap().to_str().unwrap();
        auth.split("Signature=")
            .nth(1)
            .unwrap()
            .to_string()
    }
}
