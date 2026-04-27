//! Server-side TLS/mTLS configuration.

use serde::{Deserialize, Serialize};

/// Server-side TLS (and optionally mTLS) configuration for inbound servers.
///
/// Paths point to PEM-encoded files that are read eagerly when
/// [`build_tls_acceptor`](crate::build_tls_acceptor) is called — failures
/// surface at startup, not on the first connection.
///
/// # TLS
///
/// ```rust,ignore
/// let cfg = IngressTlsConfig::tls("certs/server.crt", "certs/server.key");
/// let server = AxumHttpServer::new("0.0.0.0:443", handler).with_tls(cfg);
/// ```
///
/// # mTLS
///
/// ```rust,ignore
/// let cfg = IngressTlsConfig::mtls("certs/server.crt", "certs/server.key", "certs/client-ca.crt");
/// let server = TonicGrpcServer::new("0.0.0.0:50443", handler).with_tls(cfg);
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IngressTlsConfig {
    /// Path to a PEM file containing the server certificate chain (leaf first).
    pub cert_pem_path: String,

    /// Path to a PEM file containing the server's private key (RSA, EC, or
    /// Ed25519).
    pub key_pem_path: String,

    /// If set, enables mTLS: clients must present a certificate signed by the
    /// CA in this PEM file. When absent, client certificates are not required.
    pub client_ca_pem_path: Option<String>,
}

impl IngressTlsConfig {
    /// TLS-only: server authenticates with `cert`/`key`; client certificates
    /// are not required.
    pub fn tls(cert_pem_path: impl Into<String>, key_pem_path: impl Into<String>) -> Self {
        Self {
            cert_pem_path:     cert_pem_path.into(),
            key_pem_path:      key_pem_path.into(),
            client_ca_pem_path: None,
        }
    }

    /// mTLS: server authenticates with `cert`/`key`; clients must present a
    /// certificate signed by `client_ca`.
    pub fn mtls(
        cert_pem_path:     impl Into<String>,
        key_pem_path:      impl Into<String>,
        client_ca_pem_path: impl Into<String>,
    ) -> Self {
        Self {
            cert_pem_path:     cert_pem_path.into(),
            key_pem_path:      key_pem_path.into(),
            client_ca_pem_path: Some(client_ca_pem_path.into()),
        }
    }

    /// Whether client certificate verification is required.
    pub fn is_mtls(&self) -> bool {
        self.client_ca_pem_path.is_some()
    }
}
