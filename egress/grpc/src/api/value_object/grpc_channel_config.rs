//! Outbound channel configuration — TLS-by-default, fail-closed.

use serde::{Deserialize, Serialize};

use super::compression_mode::CompressionMode;
use super::keep_alive_config::KeepAliveConfig;
use super::mtls_config::MtlsConfig;

/// Default ceiling for inbound message bytes (4 MiB).
pub const DEFAULT_MAX_MESSAGE_BYTES: usize = 4 * 1024 * 1024;

/// Configuration for a single outbound gRPC channel.
///
/// **TLS-by-default**.  `tls_required` is `true` in
/// `Default::default()`.  Plaintext requires explicit
/// [`GrpcChannelConfig::allow_plaintext`] — fail-closed by design.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GrpcChannelConfig {
    /// Channel endpoint.
    pub endpoint: String,
    /// Require TLS on the wire.  When `true` (default) and the
    /// endpoint is plaintext, the transport refuses to dial.
    pub tls_required: bool,
    /// Optional mTLS client identity.
    pub mtls: Option<MtlsConfig>,
    /// Optional HTTP/2 keep-alive policy.
    pub keep_alive: Option<KeepAliveConfig>,
    /// Hard cap on a single response message in bytes.
    pub max_message_bytes: usize,
    /// Compression mode for outbound payloads.
    pub compression: CompressionMode,
}

impl GrpcChannelConfig {
    /// Construct a TLS-required channel for `endpoint`.
    pub fn new(endpoint: impl Into<String>) -> Self {
        Self {
            endpoint:          endpoint.into(),
            tls_required:      true,
            mtls:              None,
            keep_alive:        None,
            max_message_bytes: DEFAULT_MAX_MESSAGE_BYTES,
            compression:       CompressionMode::None,
        }
    }

    /// Explicitly relax the TLS requirement.
    pub fn allow_plaintext(mut self) -> Self {
        self.tls_required = false;
        self
    }

    /// Attach an mTLS client identity.
    pub fn with_mtls(mut self, mtls: MtlsConfig) -> Self {
        self.mtls = Some(mtls);
        self
    }

    /// Attach an HTTP/2 keep-alive policy.
    pub fn with_keep_alive(mut self, keep_alive: KeepAliveConfig) -> Self {
        self.keep_alive = Some(keep_alive);
        self
    }

    /// Set the max-message-bytes ceiling.
    pub fn with_max_message_bytes(mut self, bytes: usize) -> Self {
        self.max_message_bytes = bytes;
        self
    }

    /// Set the compression mode.
    pub fn with_compression(mut self, mode: CompressionMode) -> Self {
        self.compression = mode;
        self
    }
}

impl Default for GrpcChannelConfig {
    /// Defaults: empty endpoint, TLS required, no mTLS, no keep-alive,
    /// 4 MiB message cap, no compression.
    fn default() -> Self {
        Self {
            endpoint:          String::new(),
            tls_required:      true,
            mtls:              None,
            keep_alive:        None,
            max_message_bytes: DEFAULT_MAX_MESSAGE_BYTES,
            compression:       CompressionMode::None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// @covers: GrpcChannelConfig::default — `tls_required` is true.
    /// Issue #5 acceptance gate.
    #[test]
    fn test_default_sets_tls_required_to_true() {
        let cfg = GrpcChannelConfig::default();
        assert!(cfg.tls_required, "TLS-by-default invariant broken");
    }

    /// @covers: GrpcChannelConfig::default — message cap defaults to 4 MiB.
    #[test]
    fn test_default_max_message_bytes_is_four_mib() {
        let cfg = GrpcChannelConfig::default();
        assert_eq!(cfg.max_message_bytes, 4 * 1024 * 1024);
    }

    /// @covers: GrpcChannelConfig::default — compression defaults to None.
    #[test]
    fn test_default_compression_is_none() {
        let cfg = GrpcChannelConfig::default();
        assert_eq!(cfg.compression, CompressionMode::None);
    }

    /// @covers: GrpcChannelConfig::new — sets endpoint and keeps TLS.
    #[test]
    fn test_new_sets_endpoint_and_keeps_tls_required() {
        let cfg = GrpcChannelConfig::new("https://x.example.com:443");
        assert_eq!(cfg.endpoint, "https://x.example.com:443");
        assert!(cfg.tls_required);
    }

    /// @covers: GrpcChannelConfig::allow_plaintext — only way to relax TLS.
    #[test]
    fn test_allow_plaintext_relaxes_tls_requirement() {
        let cfg = GrpcChannelConfig::new("http://localhost:50051").allow_plaintext();
        assert!(!cfg.tls_required);
    }

    /// @covers: GrpcChannelConfig::with_mtls — stores mTLS identity.
    #[test]
    fn test_with_mtls_stores_identity() {
        let cfg = GrpcChannelConfig::new("https://x")
            .with_mtls(MtlsConfig::new("c.pem", "k.pem"));
        let mtls = cfg.mtls.expect("mtls must be Some");
        assert_eq!(mtls.cert_pem_path, "c.pem");
    }

    /// @covers: GrpcChannelConfig::with_compression — switches mode.
    #[test]
    fn test_with_compression_switches_mode() {
        let cfg = GrpcChannelConfig::new("https://x").with_compression(CompressionMode::Gzip);
        assert_eq!(cfg.compression, CompressionMode::Gzip);
    }
}
