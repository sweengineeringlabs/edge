//! Inbound server configuration — TLS-by-default, fail-closed.

use std::net::SocketAddr;

use serde::{Deserialize, Serialize};
use swe_edge_ingress_tls::IngressTlsConfig;

use super::compression_mode::CompressionMode;

/// Default ceiling for inbound message bytes (4 MiB).
pub const DEFAULT_MAX_MESSAGE_BYTES: usize = 4 * 1024 * 1024;

/// Default cap on concurrent HTTP/2 streams per connection.
pub const DEFAULT_MAX_CONCURRENT_STREAMS: u32 = 100;

/// Configuration for an inbound gRPC server.
///
/// **TLS-by-default**.  Plaintext servers must explicitly call
/// [`GrpcServerConfig::allow_plaintext`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GrpcServerConfig {
    /// Address to bind.
    pub bind: SocketAddr,
    /// Require TLS on the wire.
    pub tls_required: bool,
    /// TLS configuration.  When set with `client_ca_pem_path`, mTLS.
    pub tls: Option<IngressTlsConfig>,
    /// Hard cap on a single inbound message in bytes.
    pub max_message_bytes: usize,
    /// HTTP/2 SETTINGS_MAX_CONCURRENT_STREAMS advertised to clients.
    pub max_concurrent_streams: u32,
    /// Phase 3 hook — Phase 2 wires the field but does not enforce it.
    pub allow_unauthenticated: bool,
    /// Compression negotiation mode.
    pub compression: CompressionMode,
}

impl GrpcServerConfig {
    /// Construct a server config bound to `bind` with TLS required and
    /// all other knobs at fail-closed defaults.
    pub fn new(bind: SocketAddr) -> Self {
        Self {
            bind,
            tls_required:           true,
            tls:                    None,
            max_message_bytes:      DEFAULT_MAX_MESSAGE_BYTES,
            max_concurrent_streams: DEFAULT_MAX_CONCURRENT_STREAMS,
            allow_unauthenticated:  false,
            compression:            CompressionMode::None,
        }
    }

    /// Explicitly relax the TLS requirement.
    pub fn allow_plaintext(mut self) -> Self {
        self.tls_required = false;
        self
    }

    /// Attach a TLS / mTLS configuration.
    pub fn with_tls(mut self, tls: IngressTlsConfig) -> Self {
        self.tls = Some(tls);
        self
    }

    /// Override the max-message-bytes cap.
    pub fn with_max_message_bytes(mut self, bytes: usize) -> Self {
        self.max_message_bytes = bytes;
        self
    }

    /// Override the max-concurrent-streams cap.
    pub fn with_max_concurrent_streams(mut self, streams: u32) -> Self {
        self.max_concurrent_streams = streams;
        self
    }

    /// Set the compression mode.
    pub fn with_compression(mut self, mode: CompressionMode) -> Self {
        self.compression = mode;
        self
    }

    /// Phase 3 hook: opt out of authentication.
    pub fn allow_unauthenticated(mut self) -> Self {
        self.allow_unauthenticated = true;
        self
    }
}

impl Default for GrpcServerConfig {
    /// Defaults: bind to `0.0.0.0:0`, TLS required, no TLS material,
    /// 4 MiB message cap, 100 concurrent streams, authenticated,
    /// no compression.
    ///
    /// `bind = 0.0.0.0:0` means "OS-assigned port on all interfaces" —
    /// callers MUST override the address before [`crate::TonicGrpcServer::serve`]
    /// in production deployments.
    fn default() -> Self {
        Self {
            bind:                   "0.0.0.0:0".parse().expect("static literal"),
            tls_required:           true,
            tls:                    None,
            max_message_bytes:      DEFAULT_MAX_MESSAGE_BYTES,
            max_concurrent_streams: DEFAULT_MAX_CONCURRENT_STREAMS,
            allow_unauthenticated:  false,
            compression:            CompressionMode::None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn addr() -> SocketAddr {
        "127.0.0.1:0".parse().unwrap()
    }

    /// @covers: GrpcServerConfig::new — TLS-by-default.
    #[test]
    fn test_new_sets_tls_required_to_true() {
        let cfg = GrpcServerConfig::new(addr());
        assert!(cfg.tls_required);
    }

    /// @covers: GrpcServerConfig::new — allow_unauthenticated false.
    #[test]
    fn test_new_sets_allow_unauthenticated_to_false() {
        let cfg = GrpcServerConfig::new(addr());
        assert!(!cfg.allow_unauthenticated);
    }

    /// @covers: GrpcServerConfig::new — message cap default.
    #[test]
    fn test_new_max_message_bytes_default_is_four_mib() {
        let cfg = GrpcServerConfig::new(addr());
        assert_eq!(cfg.max_message_bytes, 4 * 1024 * 1024);
    }

    /// @covers: GrpcServerConfig::new — concurrent streams default.
    #[test]
    fn test_new_max_concurrent_streams_default_is_one_hundred() {
        let cfg = GrpcServerConfig::new(addr());
        assert_eq!(cfg.max_concurrent_streams, 100);
    }

    /// @covers: GrpcServerConfig::allow_plaintext — relaxes TLS only.
    #[test]
    fn test_allow_plaintext_only_relaxes_tls_requirement() {
        let cfg = GrpcServerConfig::new(addr()).allow_plaintext();
        assert!(!cfg.tls_required);
        assert!(!cfg.allow_unauthenticated);
    }

    /// @covers: GrpcServerConfig::default — `tls_required` is true.
    /// Issue #5 acceptance gate.
    #[test]
    fn test_default_sets_tls_required_to_true() {
        let cfg = GrpcServerConfig::default();
        assert!(cfg.tls_required, "TLS-by-default invariant must hold");
    }

    /// @covers: GrpcServerConfig::default — `allow_unauthenticated` false.
    #[test]
    fn test_default_disallows_unauthenticated_callers() {
        let cfg = GrpcServerConfig::default();
        assert!(!cfg.allow_unauthenticated);
    }

    /// @covers: GrpcServerConfig::default — message cap defaults to 4 MiB.
    #[test]
    fn test_default_max_message_bytes_is_four_mib() {
        let cfg = GrpcServerConfig::default();
        assert_eq!(cfg.max_message_bytes, 4 * 1024 * 1024);
    }

    /// @covers: GrpcServerConfig::default — concurrent streams defaults to 100.
    #[test]
    fn test_default_max_concurrent_streams_is_one_hundred() {
        let cfg = GrpcServerConfig::default();
        assert_eq!(cfg.max_concurrent_streams, 100);
    }
}
