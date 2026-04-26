use std::sync::Arc;
use std::time::Instant;

use futures::future::BoxFuture;
use parking_lot::Mutex;

use edge_controller::{HealthStatus, LifecycleMonitor};

use crate::api::error::{RuntimeError, RuntimeResult};
use crate::api::traits::RuntimeManager;
use crate::api::types::{RuntimeConfig, RuntimeHealth, RuntimeStatus};
use crate::api::types::runtime_health::ComponentHealth;

pub(crate) struct DefaultRuntimeManager {
    config:     RuntimeConfig,
    lifecycle:  Arc<dyn LifecycleMonitor>,
    status:     Arc<Mutex<RuntimeStatus>>,
    started_at: Arc<Mutex<Option<Instant>>>,
}

impl DefaultRuntimeManager {
    pub(crate) fn new(config: RuntimeConfig, lifecycle: Arc<dyn LifecycleMonitor>) -> Self {
        Self {
            config,
            lifecycle,
            status:     Arc::new(Mutex::new(RuntimeStatus::Stopped)),
            started_at: Arc::new(Mutex::new(None)),
        }
    }
}

impl RuntimeManager for DefaultRuntimeManager {
    fn start(&self) -> BoxFuture<'_, RuntimeResult<()>> {
        Box::pin(async move {
            {
                let mut s = self.status.lock();
                if *s == RuntimeStatus::Running {
                    return Ok(());
                }
                *s = RuntimeStatus::Starting;
            }

            self.lifecycle.start_background_tasks().await;

            {
                let mut s = self.status.lock();
                *s = RuntimeStatus::Running;
                *self.started_at.lock() = Some(Instant::now());
            }

            if self.config.systemd_notify {
                tracing::info!("READY=1");
            }

            tracing::info!(
                service = %self.config.service_name,
                http    = %self.config.http_bind,
                grpc    = %self.config.grpc_bind,
                "runtime started"
            );

            Ok(())
        })
    }

    fn shutdown(&self) -> BoxFuture<'_, RuntimeResult<()>> {
        Box::pin(async move {
            {
                let mut s = self.status.lock();
                if *s == RuntimeStatus::Stopped {
                    return Ok(());
                }
                *s = RuntimeStatus::Stopping;
            }

            if self.config.systemd_notify {
                tracing::info!("STOPPING=1");
            }

            self.lifecycle
                .shutdown()
                .await
                .map_err(|e| RuntimeError::ShutdownFailed(e.to_string()))?;

            *self.status.lock() = RuntimeStatus::Stopped;

            tracing::info!(service = %self.config.service_name, "runtime stopped");

            Ok(())
        })
    }

    fn health(&self) -> BoxFuture<'_, RuntimeHealth> {
        Box::pin(async move {
            let status = *self.status.lock();
            let report = self.lifecycle.health().await;

            let uptime_secs = self
                .started_at
                .lock()
                .map(|t| t.elapsed().as_secs())
                .unwrap_or(0);

            let components = report
                .components
                .iter()
                .map(|c| match c.status {
                    HealthStatus::Healthy => ComponentHealth::healthy(&c.id),
                    _ => ComponentHealth::unhealthy(
                        &c.id,
                        c.message.as_deref().unwrap_or("degraded"),
                    ),
                })
                .collect();

            RuntimeHealth { status, components, uptime_secs }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use edge_controller::{HealthReport, LifecycleError};

    struct StubLifecycle;

    #[async_trait]
    impl LifecycleMonitor for StubLifecycle {
        async fn health(&self) -> HealthReport {
            HealthReport::from_components(vec![])
        }
        async fn start_background_tasks(&self) {}
        async fn shutdown(&self) -> Result<(), LifecycleError> {
            Ok(())
        }
    }

    fn make_manager() -> DefaultRuntimeManager {
        DefaultRuntimeManager::new(
            RuntimeConfig::default().with_systemd_notify(false),
            Arc::new(StubLifecycle),
        )
    }

    #[tokio::test]
    async fn test_start_transitions_status_to_running() {
        let m = make_manager();
        m.start().await.expect("start ok");
        assert_eq!(*m.status.lock(), RuntimeStatus::Running);
    }

    #[tokio::test]
    async fn test_start_is_idempotent() {
        let m = make_manager();
        m.start().await.expect("first start ok");
        m.start().await.expect("second start ok");
        assert_eq!(*m.status.lock(), RuntimeStatus::Running);
    }

    #[tokio::test]
    async fn test_shutdown_transitions_status_to_stopped() {
        let m = make_manager();
        m.start().await.expect("start ok");
        m.shutdown().await.expect("shutdown ok");
        assert_eq!(*m.status.lock(), RuntimeStatus::Stopped);
    }

    #[tokio::test]
    async fn test_shutdown_is_idempotent() {
        let m = make_manager();
        m.shutdown().await.expect("first shutdown ok");
        m.shutdown().await.expect("second shutdown ok");
    }

    #[tokio::test]
    async fn test_health_reports_running_after_start() {
        let m = make_manager();
        m.start().await.expect("start ok");
        let h = m.health().await;
        assert_eq!(h.status, RuntimeStatus::Running);
        assert!(h.is_healthy());
    }

    #[tokio::test]
    async fn test_health_reports_stopped_before_start() {
        let m = make_manager();
        let h = m.health().await;
        assert_eq!(h.status, RuntimeStatus::Stopped);
    }
}
