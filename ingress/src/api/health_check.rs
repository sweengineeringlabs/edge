//! Health check types for gateway adapters.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Health status of a gateway.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum HealthStatus {
    Healthy,
    Degraded,
    Unhealthy,
    #[default]
    Unknown,
}

/// Health check result for a gateway.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheck {
    pub status: HealthStatus,
    pub message: Option<String>,
    pub latency_ms: Option<u64>,
    #[serde(default)]
    pub metadata: HashMap<String, serde_json::Value>,
    pub checked_at: DateTime<Utc>,
}

impl HealthCheck {
    pub fn healthy() -> Self {
        Self { status: HealthStatus::Healthy, message: None, latency_ms: None, metadata: HashMap::new(), checked_at: Utc::now() }
    }

    pub fn healthy_with_latency(latency_ms: u64) -> Self {
        Self { status: HealthStatus::Healthy, message: None, latency_ms: Some(latency_ms), metadata: HashMap::new(), checked_at: Utc::now() }
    }

    pub fn unhealthy(message: impl Into<String>) -> Self {
        Self { status: HealthStatus::Unhealthy, message: Some(message.into()), latency_ms: None, metadata: HashMap::new(), checked_at: Utc::now() }
    }

    pub fn degraded(message: impl Into<String>) -> Self {
        Self { status: HealthStatus::Degraded, message: Some(message.into()), latency_ms: None, metadata: HashMap::new(), checked_at: Utc::now() }
    }

    pub fn with_metadata(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.metadata.insert(key.into(), value);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// @covers: healthy
    #[test]
    fn test_healthy_returns_healthy_status() {
        let h = HealthCheck::healthy();
        assert_eq!(h.status, HealthStatus::Healthy);
        assert!(h.message.is_none());
    }

    /// @covers: healthy_with_latency
    #[test]
    fn test_healthy_with_latency_sets_latency_ms() {
        let h = HealthCheck::healthy_with_latency(42);
        assert_eq!(h.status, HealthStatus::Healthy);
        assert_eq!(h.latency_ms, Some(42));
    }

    /// @covers: unhealthy
    #[test]
    fn test_unhealthy_returns_unhealthy_status_with_message() {
        let h = HealthCheck::unhealthy("connection failed");
        assert_eq!(h.status, HealthStatus::Unhealthy);
        assert_eq!(h.message, Some("connection failed".to_string()));
    }

    /// @covers: degraded
    #[test]
    fn test_degraded_returns_degraded_status_with_message() {
        let h = HealthCheck::degraded("high latency");
        assert_eq!(h.status, HealthStatus::Degraded);
    }

    /// @covers: with_metadata
    #[test]
    fn test_with_metadata_inserts_key_value_pair() {
        let h = HealthCheck::healthy().with_metadata("version", serde_json::json!("1.0"));
        assert_eq!(h.metadata.get("version"), Some(&serde_json::json!("1.0")));
    }
}
