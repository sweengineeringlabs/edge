//! Serializable shape of one recorded request/response pair.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// A single recorded HTTP exchange. Bodies are Base64 so binary
/// content round-trips safely through YAML.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct RecordedInteraction {
    pub(crate) request: RecordedRequest,
    pub(crate) response: RecordedResponse,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct RecordedRequest {
    pub(crate) method: String,
    pub(crate) url: String,
    /// Hex-encoded SHA256 of the request body (when
    /// `match_on` includes "body_hash"). `None` when the
    /// request had no body or body_hash isn't in match_on.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) body_hash: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct RecordedResponse {
    pub(crate) status: u16,
    /// Response headers. BTreeMap so the on-disk serialization
    /// is stable across runs (reproducible diffs).
    pub(crate) headers: BTreeMap<String, String>,
    /// Base64-encoded body bytes — survives binary content.
    pub(crate) body_base64: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    /// @covers: RecordedInteraction
    #[test]
    fn test_serde_roundtrip_through_yaml() {
        let ri = RecordedInteraction {
            request: RecordedRequest {
                method: "GET".into(),
                url: "https://example.test/foo".into(),
                body_hash: None,
            },
            response: RecordedResponse {
                status: 200,
                headers: {
                    let mut m = BTreeMap::new();
                    m.insert("content-type".into(), "application/json".into());
                    m
                },
                body_base64: "aGVsbG8=".into(), // "hello"
            },
        };
        let yaml = serde_yaml::to_string(&ri).unwrap();
        let decoded: RecordedInteraction = serde_yaml::from_str(&yaml).unwrap();
        assert_eq!(decoded.request.method, "GET");
        assert_eq!(decoded.response.status, 200);
    }
}
