//! RuntimeHealth — aggregate health snapshot of the daemon.

use serde::{Deserialize, Serialize};

use crate::api::types::runtime_status::RuntimeStatus;

/// Health of a single named component within the runtime.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentHealth {
    pub name:    String,
    pub healthy: bool,
    pub detail:  Option<String>,
}

impl ComponentHealth {
    pub fn healthy(name: impl Into<String>) -> Self {
        Self { name: name.into(), healthy: true, detail: None }
    }

    pub fn unhealthy(name: impl Into<String>, detail: impl Into<String>) -> Self {
        Self { name: name.into(), healthy: false, detail: Some(detail.into()) }
    }
}

/// Aggregate health snapshot of the runtime manager.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeHealth {
    pub status:     RuntimeStatus,
    pub components: Vec<ComponentHealth>,
    pub uptime_secs: u64,
}

impl RuntimeHealth {
    pub fn is_healthy(&self) -> bool {
        self.status.is_healthy() && self.components.iter().all(|c| c.healthy)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_component_healthy_has_no_detail() {
        let c = ComponentHealth::healthy("http-server");
        assert!(c.healthy);
        assert!(c.detail.is_none());
    }

    #[test]
    fn test_component_unhealthy_has_detail() {
        let c = ComponentHealth::unhealthy("database", "connection refused");
        assert!(!c.healthy);
        assert_eq!(c.detail.as_deref(), Some("connection refused"));
    }

    #[test]
    fn test_runtime_health_is_healthy_all_components_up() {
        let h = RuntimeHealth {
            status:      RuntimeStatus::Running,
            components:  vec![ComponentHealth::healthy("http"), ComponentHealth::healthy("grpc")],
            uptime_secs: 120,
        };
        assert!(h.is_healthy());
    }

    #[test]
    fn test_runtime_health_is_unhealthy_if_any_component_down() {
        let h = RuntimeHealth {
            status:      RuntimeStatus::Running,
            components:  vec![ComponentHealth::healthy("http"), ComponentHealth::unhealthy("grpc", "timeout")],
            uptime_secs: 5,
        };
        assert!(!h.is_healthy());
    }

    #[test]
    fn test_runtime_health_is_unhealthy_if_status_degraded() {
        let h = RuntimeHealth {
            status:      RuntimeStatus::Degraded,
            components:  vec![ComponentHealth::healthy("http")],
            uptime_secs: 10,
        };
        assert!(!h.is_healthy());
    }
}
