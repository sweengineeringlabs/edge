//! Factory functions — Static Access Facade (SAF).

use std::sync::Arc;

use crate::api::lifecycle_monitor::LifecycleMonitor;
use crate::core::null_lifecycle_monitor::NullLifecycleMonitor;

/// Construct a no-op [`LifecycleMonitor`] useful for tests or early bring-up.
///
/// Returned as `Arc<dyn LifecycleMonitor>` so the concrete impl type stays
/// private.
pub fn new_null_lifecycle_monitor() -> Arc<dyn LifecycleMonitor> {
    Arc::new(NullLifecycleMonitor::new())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::health::HealthStatus;

    #[tokio::test]
    async fn test_factory_returns_healthy_lifecycle_monitor() {
        let m = new_null_lifecycle_monitor();
        assert_eq!(m.health().await.overall, HealthStatus::Healthy);
    }
}
