//! Health types shared by the `LifecycleMonitor` and `Handler` concerns.

use serde::{Deserialize, Serialize};

/// Terminal health classification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum HealthStatus {
    /// All subsystems responsive and within operating parameters.
    Healthy,
    /// One or more subsystems impaired but the Controller can still serve.
    Degraded,
    /// Cannot serve requests; operator intervention expected.
    Unhealthy,
}

/// Health snapshot for a single component (handler, subsystem, backend).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentHealth {
    /// Stable identifier matching `Handler::id` for handler components.
    pub id: String,
    /// Classification at the moment the probe ran.
    pub status: HealthStatus,
    /// Optional human-readable reason when status is not `Healthy`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

impl ComponentHealth {
    /// Construct a `Healthy` component entry with no message.
    pub fn healthy(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            status: HealthStatus::Healthy,
            message: None,
        }
    }

    /// Construct a non-healthy component entry with a reason.
    pub fn with_status(
        id: impl Into<String>,
        status: HealthStatus,
        message: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            status,
            message: Some(message.into()),
        }
    }
}

/// Aggregated health report returned by `LifecycleMonitor::health`.
///
/// `overall` summarizes the per-component results. Convention: if any
/// component is `Unhealthy`, overall is `Unhealthy`; else if any is
/// `Degraded`, overall is `Degraded`; else `Healthy`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthReport {
    pub overall: HealthStatus,
    pub components: Vec<ComponentHealth>,
}

impl HealthReport {
    /// Derive `overall` from the component list per the aggregation rule.
    pub fn from_components(components: Vec<ComponentHealth>) -> Self {
        let overall = if components.iter().any(|c| c.status == HealthStatus::Unhealthy) {
            HealthStatus::Unhealthy
        } else if components.iter().any(|c| c.status == HealthStatus::Degraded) {
            HealthStatus::Degraded
        } else {
            HealthStatus::Healthy
        };
        Self { overall, components }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_aggregate_all_healthy() {
        let report = HealthReport::from_components(vec![
            ComponentHealth::healthy("a"),
            ComponentHealth::healthy("b"),
        ]);
        assert_eq!(report.overall, HealthStatus::Healthy);
    }

    #[test]
    fn test_aggregate_any_unhealthy_wins() {
        let report = HealthReport::from_components(vec![
            ComponentHealth::healthy("a"),
            ComponentHealth::with_status("b", HealthStatus::Degraded, "slow"),
            ComponentHealth::with_status("c", HealthStatus::Unhealthy, "dead"),
        ]);
        assert_eq!(report.overall, HealthStatus::Unhealthy);
    }

    #[test]
    fn test_aggregate_degraded_without_unhealthy() {
        let report = HealthReport::from_components(vec![
            ComponentHealth::healthy("a"),
            ComponentHealth::with_status("b", HealthStatus::Degraded, "slow"),
        ]);
        assert_eq!(report.overall, HealthStatus::Degraded);
    }

    #[test]
    fn test_aggregate_empty_is_healthy() {
        let report = HealthReport::from_components(vec![]);
        assert_eq!(report.overall, HealthStatus::Healthy);
    }
}
