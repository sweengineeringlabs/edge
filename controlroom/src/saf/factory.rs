//! Factory functions — Static Access Facade (SAF).
//!
//! Top-level `fn` entrypoints for constructing controlroom building blocks.
//! Kept deliberately small: each factory wraps a single `new` so downstream
//! crates have a stable entry point even if internal constructors change.

use std::sync::Arc;

use crate::api::handler_registry::HandlerRegistry;
use crate::api::lifecycle_monitor::LifecycleMonitor;
use crate::core::null_lifecycle_monitor::NullLifecycleMonitor;

/// Construct a fresh empty `HandlerRegistry`.
///
/// Returned as `Arc<_>` because the registry is shared between the
/// controlroom's `Job` impl and any operator tooling that lists or
/// mutates the handler set.
pub fn new_handler_registry<Req, Resp>() -> Arc<HandlerRegistry<Req, Resp>>
where
    Req: Send + 'static,
    Resp: Send + 'static,
{
    Arc::new(HandlerRegistry::new())
}

/// Construct a no-op `LifecycleMonitor` useful for tests or early bring-up.
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

    #[test]
    fn test_factory_returns_empty_registry() {
        let reg: Arc<HandlerRegistry<String, String>> = new_handler_registry();
        assert!(reg.is_empty());
    }

    #[tokio::test]
    async fn test_factory_returns_healthy_lifecycle_monitor() {
        let m = new_null_lifecycle_monitor();
        assert_eq!(m.health().await.overall, HealthStatus::Healthy);
    }
}
