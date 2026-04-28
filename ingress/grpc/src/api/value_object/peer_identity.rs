//! Peer identity value object + reserved metadata keys for mTLS
//! peer identity flow-through.
//!
//! When the ingress gRPC server is configured with mTLS, the
//! [`crate::TonicGrpcServer`] parses the peer's leaf certificate
//! and injects the identity into the [`crate::GrpcMetadata`] under
//! these reserved keys before the handler runs.
//!
//! # Reserved keys
//!
//! | Key | Source | Notes |
//! |---|---|---|
//! | `x-edge-peer-identity` | leaf cert subject DN | full Distinguished Name |
//! | `x-edge-peer-cn`       | subject CN attribute | `CN=` value, if present |
//! | `x-edge-peer-san-dns`  | DNS SANs             | comma-separated when multiple |
//! | `x-edge-peer-san-uri`  | URI SANs             | comma-separated when multiple |
//! | `x-edge-peer-cert-fingerprint-sha256` | leaf cert | lower-hex sha256 of DER bytes |
//!
//! Handlers and authz interceptors MUST treat these as **trusted**
//! only when the underlying transport actually performed mTLS — the
//! server only injects them after a successful client-cert handshake.
//! Plaintext / TLS-only connections never carry these keys.
//!
//! # Stripping rule
//!
//! The server strips any incoming header that uses one of these
//! reserved key prefixes (`x-edge-peer-*`) before it injects its
//! own values.  This prevents a client from spoofing identity over
//! a non-mTLS connection.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

/// Cryptographically authenticated identity of a TLS peer.
///
/// Built by [`crate::core::peer_identity::extract_peer_identity`] from the
/// leaf certificate the client presented during the mTLS handshake.
/// Handlers and authz interceptors should treat the values as **trusted**
/// only when the underlying transport actually performed mTLS — the
/// server only synthesises a `PeerIdentity` after a successful client-cert
/// handshake.
///
/// `cn` and `san` come from the standard X.509 fields; `custom_oids`
/// carries any additional subject attributes by their dotted-decimal OID.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PeerIdentity {
    /// Common Name (CN attribute) of the subject.  `None` when absent.
    pub cn: Option<String>,
    /// Subject Alternative Names (DNS + URI), preserving wire order.
    pub san: Vec<String>,
    /// Other subject attributes keyed by their dotted-decimal OID
    /// (e.g. `"1.2.840.113549.1.9.1"` for `emailAddress`).  Unknown
    /// OIDs are surfaced verbatim so authz policies can match on
    /// custom organisational identifiers.
    pub custom_oids: HashMap<String, String>,
}

impl PeerIdentity {
    /// Construct an empty identity (used when only the fingerprint is known).
    pub fn empty() -> Self { Self::default() }

    /// Returns `true` when neither CN nor any SAN is populated.
    pub fn is_empty(&self) -> bool {
        self.cn.is_none() && self.san.is_empty() && self.custom_oids.is_empty()
    }
}

/// Subject DN of the peer certificate.
pub const PEER_IDENTITY: &str = "x-edge-peer-identity";

/// Common name (CN attribute) of the peer certificate's subject.
pub const PEER_CN: &str = "x-edge-peer-cn";

/// DNS Subject Alternative Names, joined by `,` if multiple.
pub const PEER_SAN_DNS: &str = "x-edge-peer-san-dns";

/// URI Subject Alternative Names, joined by `,` if multiple.
pub const PEER_SAN_URI: &str = "x-edge-peer-san-uri";

/// Lower-hex SHA-256 fingerprint of the peer's leaf certificate (DER).
pub const PEER_CERT_FINGERPRINT_SHA256: &str = "x-edge-peer-cert-fingerprint-sha256";

/// All reserved peer-identity key prefixes the server strips from
/// incoming metadata before it injects its own values.
pub const RESERVED_PEER_PREFIXES: &[&str] = &["x-edge-peer-"];

/// Returns `true` when `name` matches a reserved peer-identity key prefix.
pub fn is_reserved_peer_key(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    RESERVED_PEER_PREFIXES.iter().any(|p| lower.starts_with(p))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// @covers: is_reserved_peer_key — matches `x-edge-peer-*`.
    #[test]
    fn test_is_reserved_peer_key_matches_x_edge_peer_prefix() {
        assert!(is_reserved_peer_key("x-edge-peer-cn"));
        assert!(is_reserved_peer_key("x-edge-peer-identity"));
    }

    /// @covers: is_reserved_peer_key — case-insensitive.
    #[test]
    fn test_is_reserved_peer_key_is_case_insensitive() {
        assert!(is_reserved_peer_key("X-Edge-Peer-Cn"));
    }

    /// @covers: is_reserved_peer_key — unrelated keys are not reserved.
    #[test]
    fn test_is_reserved_peer_key_does_not_match_unrelated_keys() {
        assert!(!is_reserved_peer_key("authorization"));
        assert!(!is_reserved_peer_key("x-edge-trace"));
    }

    /// @covers: PeerIdentity::empty — all fields default-initialised.
    #[test]
    fn test_peer_identity_empty_has_no_cn_or_san_or_custom_oids() {
        let id = PeerIdentity::empty();
        assert!(id.cn.is_none());
        assert!(id.san.is_empty());
        assert!(id.custom_oids.is_empty());
        assert!(id.is_empty());
    }

    /// @covers: PeerIdentity::is_empty — `false` once any field is populated.
    #[test]
    fn test_peer_identity_is_not_empty_when_cn_is_populated() {
        let id = PeerIdentity {
            cn: Some("svc".into()),
            san: Vec::new(),
            custom_oids: HashMap::new(),
        };
        assert!(!id.is_empty());
    }
}
