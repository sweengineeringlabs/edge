//! HTTP/2 keep-alive policy for gRPC channels.

use std::time::Duration;

use serde::{Deserialize, Serialize};

/// HTTP/2 keep-alive ping policy.
///
/// Defaults match gRPC's recommended client-side settings (10s
/// interval, 20s timeout).
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct KeepAliveConfig {
    /// Interval between PING frames.
    pub interval: Duration,
    /// Timeout for receiving a PING ACK.
    pub timeout: Duration,
    /// If `true`, send pings even when no streams are active.
    pub permit_without_calls: bool,
}

impl Default for KeepAliveConfig {
    fn default() -> Self {
        Self {
            interval: Duration::from_secs(10),
            timeout:  Duration::from_secs(20),
            permit_without_calls: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// @covers: KeepAliveConfig::default — interval 10s, timeout 20s.
    #[test]
    fn test_default_uses_recommended_grpc_intervals() {
        let cfg = KeepAliveConfig::default();
        assert_eq!(cfg.interval, Duration::from_secs(10));
        assert_eq!(cfg.timeout,  Duration::from_secs(20));
        assert!(!cfg.permit_without_calls);
    }
}
