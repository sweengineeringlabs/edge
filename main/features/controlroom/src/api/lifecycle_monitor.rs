//! LifecycleMonitor trait — **Lifecycle** concern of the 5-Concern ControlRoom
//! pattern.
//!
//! Runtime state management: aggregate health, background tasks, and graceful
//! shutdown. Kept separate from `Job` so the operator surface (health, shutdown)
//! can be exposed without granting execution privileges.

use async_trait::async_trait;

use super::error::LifecycleError;
use super::health::HealthReport;

/// Runtime lifecycle management for a controlroom instance.
#[async_trait]
pub trait LifecycleMonitor: Send + Sync {
    /// Aggregate health across all handlers and subsystems.
    async fn health(&self) -> HealthReport;

    /// Start any long-running background tasks (watchdogs, pollers).
    ///
    /// Idempotent: implementations must tolerate repeated calls by no-op'ing
    /// after the first successful start.
    async fn start_background_tasks(&self);

    /// Gracefully shut down: stop accepting new work, drain in-flight work,
    /// release resources. After `shutdown` returns, `health` is expected to
    /// report a non-healthy status.
    async fn shutdown(&self) -> Result<(), LifecycleError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lifecycle_monitor_is_object_safe() {
        fn _accept(_l: &dyn LifecycleMonitor) {}
    }
}
