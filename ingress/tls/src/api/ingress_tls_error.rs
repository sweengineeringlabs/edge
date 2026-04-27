//! Error type for TLS configuration loading.

/// Errors that can occur while loading or building a server TLS configuration.
#[derive(Debug, thiserror::Error)]
pub enum IngressTlsError {
    /// Certificate file could not be opened.
    #[error("failed to load certificate from {0}: {1}")]
    CertLoad(String, #[source] std::io::Error),

    /// Certificate PEM data could not be parsed, or the file contained no
    /// certificates.
    #[error("failed to parse certificate: {0}")]
    CertParse(String),

    /// Private key file could not be opened.
    #[error("failed to load private key from {0}: {1}")]
    KeyLoad(String, #[source] std::io::Error),

    /// Private key PEM data could not be parsed, or the file contained no
    /// key.
    #[error("failed to parse private key: {0}")]
    KeyParse(String),

    /// rustls [`ServerConfig`](rustls::ServerConfig) could not be built from
    /// the provided materials (e.g. cert/key type mismatch, unsupported CA
    /// format).
    #[error("TLS configuration error: {0}")]
    Config(String),
}
