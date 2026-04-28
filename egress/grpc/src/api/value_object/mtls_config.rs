//! Outbound mTLS identity configuration.

use serde::{Deserialize, Serialize};

/// Outbound mTLS configuration: client cert chain + private key + an
/// optional override CA bundle to authenticate the server.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MtlsConfig {
    /// PEM file containing the client certificate chain (leaf first).
    pub cert_pem_path: String,
    /// PEM file containing the client's private key.
    pub key_pem_path: String,
    /// Optional CA bundle the transport pins for server-cert
    /// verification.  When `None`, platform trust roots are used.
    pub ca_pem_path: Option<String>,
}

impl MtlsConfig {
    /// Construct an mTLS config with platform trust roots.
    pub fn new(
        cert_pem_path: impl Into<String>,
        key_pem_path:  impl Into<String>,
    ) -> Self {
        Self { cert_pem_path: cert_pem_path.into(), key_pem_path: key_pem_path.into(), ca_pem_path: None }
    }

    /// Construct an mTLS config that pins a specific CA bundle.
    pub fn with_pinned_ca(
        cert_pem_path: impl Into<String>,
        key_pem_path:  impl Into<String>,
        ca_pem_path:   impl Into<String>,
    ) -> Self {
        Self {
            cert_pem_path: cert_pem_path.into(),
            key_pem_path:  key_pem_path.into(),
            ca_pem_path:   Some(ca_pem_path.into()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// @covers: MtlsConfig::new — pinned CA is None.
    #[test]
    fn test_new_leaves_pinned_ca_unset() {
        let cfg = MtlsConfig::new("c.pem", "k.pem");
        assert!(cfg.ca_pem_path.is_none());
        assert_eq!(cfg.cert_pem_path, "c.pem");
        assert_eq!(cfg.key_pem_path, "k.pem");
    }

    /// @covers: MtlsConfig::with_pinned_ca — stores all three.
    #[test]
    fn test_with_pinned_ca_stores_all_three_paths() {
        let cfg = MtlsConfig::with_pinned_ca("c.pem", "k.pem", "ca.pem");
        assert_eq!(cfg.cert_pem_path, "c.pem");
        assert_eq!(cfg.key_pem_path,  "k.pem");
        assert_eq!(cfg.ca_pem_path.as_deref(), Some("ca.pem"));
    }
}
