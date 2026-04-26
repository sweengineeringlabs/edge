//! RuntimeConfig — process-level configuration for the daemon.

use serde::{Deserialize, Serialize};

/// Configuration for the runtime manager.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct RuntimeConfig {
    /// Service name reported to observability and systemd.
    pub service_name: String,
    /// Address to bind the primary HTTP ingress server.
    pub http_bind: String,
    /// Address to bind the gRPC ingress server.
    pub grpc_bind: String,
    /// Seconds to wait for in-flight requests to drain on shutdown.
    pub shutdown_timeout_secs: u64,
    /// Emit systemd sd_notify signals (READY=1, STOPPING=1).
    pub systemd_notify: bool,
}

impl Default for RuntimeConfig {
    fn default() -> Self {
        Self {
            service_name:          "swe-edge".into(),
            http_bind:             "0.0.0.0:8080".into(),
            grpc_bind:             "0.0.0.0:50051".into(),
            shutdown_timeout_secs: 30,
            systemd_notify:        false,
        }
    }
}

impl RuntimeConfig {
    pub fn with_service_name(mut self, name: impl Into<String>) -> Self {
        self.service_name = name.into();
        self
    }

    pub fn with_http_bind(mut self, addr: impl Into<String>) -> Self {
        self.http_bind = addr.into();
        self
    }

    pub fn with_grpc_bind(mut self, addr: impl Into<String>) -> Self {
        self.grpc_bind = addr.into();
        self
    }

    pub fn with_shutdown_timeout(mut self, secs: u64) -> Self {
        self.shutdown_timeout_secs = secs;
        self
    }

    pub fn with_systemd_notify(mut self, enabled: bool) -> Self {
        self.systemd_notify = enabled;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_runtime_config_has_expected_values() {
        let cfg = RuntimeConfig::default();
        assert_eq!(cfg.http_bind, "0.0.0.0:8080");
        assert_eq!(cfg.grpc_bind, "0.0.0.0:50051");
        assert_eq!(cfg.shutdown_timeout_secs, 30);
        assert!(!cfg.systemd_notify);
    }

    #[test]
    fn test_with_service_name_sets_name() {
        let cfg = RuntimeConfig::default().with_service_name("my-svc");
        assert_eq!(cfg.service_name, "my-svc");
    }

    #[test]
    fn test_with_http_bind_sets_addr() {
        let cfg = RuntimeConfig::default().with_http_bind("127.0.0.1:9090");
        assert_eq!(cfg.http_bind, "127.0.0.1:9090");
    }

    #[test]
    fn test_with_shutdown_timeout_sets_secs() {
        let cfg = RuntimeConfig::default().with_shutdown_timeout(60);
        assert_eq!(cfg.shutdown_timeout_secs, 60);
    }

    #[test]
    fn test_with_systemd_notify_enables_flag() {
        let cfg = RuntimeConfig::default().with_systemd_notify(true);
        assert!(cfg.systemd_notify);
    }
}
