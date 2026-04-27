//! SAF layer — daemon public facade.

use std::sync::Arc;
use std::time::Duration;

use edge_proxy::LifecycleMonitor;

use crate::core::{DefaultConfigLoader, DefaultRuntimeManager};

pub use crate::api::config::ConfigError;
pub use crate::api::config_loader::ConfigLoader;
pub use crate::api::error::{RuntimeError, RuntimeResult};
pub use crate::api::runtime_manager::RuntimeManager;
pub use crate::api::types::{RuntimeConfig, RuntimeHealth, RuntimeStatus};
pub use crate::api::types::runtime_health::ComponentHealth;
pub use crate::api::input::Input;
pub use crate::api::output::Output;
pub use crate::core::input::DefaultInput;
pub use crate::core::output::DefaultOutput;
pub use swe_edge_egress::{GrpcMessageStream, GrpcOutbound, GrpcOutboundError, GrpcOutboundResult, TonicGrpcClient};

/// Load config using the default layered chain
/// (`default.toml` → `application.toml` → env vars).
///
/// The config directory is resolved from `SWE_EDGE_CONFIG_DIR` or
/// defaults to `config/` relative to the working directory.
/// Consumer apps should prefer [`load_config_from`] to supply their
/// own path explicitly.
pub fn load_config() -> Result<RuntimeConfig, ConfigError> {
    DefaultConfigLoader::new().load()
}

/// Load config from an explicit directory.
///
/// Identical layer chain to [`load_config`] but reads
/// `<dir>/application.toml` instead of relying on env or cwd.
pub fn load_config_from(dir: impl Into<std::path::PathBuf>) -> Result<RuntimeConfig, ConfigError> {
    DefaultConfigLoader::with_dir(dir).load()
}

/// Load config scoped to a tenant
/// (`default.toml` → `application.toml` → `tenants/<id>.toml` → env vars).
///
/// See [`load_tenant_config_from`] for the consumer-app variant.
pub fn load_tenant_config(tenant_id: &str) -> Result<RuntimeConfig, ConfigError> {
    DefaultConfigLoader::new().load_for_tenant(tenant_id)
}

/// Load tenant config from an explicit directory.
///
/// Reads `<dir>/application.toml` and `<dir>/tenants/<tenant_id>.toml`.
/// Intended for consumer apps that own their config directory layout.
pub fn load_tenant_config_from(
    tenant_id: &str,
    dir: impl Into<std::path::PathBuf>,
) -> Result<RuntimeConfig, ConfigError> {
    DefaultConfigLoader::with_dir(dir).load_for_tenant(tenant_id)
}

/// Load config following the XDG Base Directory specification.
///
/// Layer chain (last wins):
/// - `$XDG_CONFIG_DIRS/<app_name>/application.toml` (system-wide, default `/etc/xdg/`)
/// - `$XDG_CONFIG_HOME/<app_name>/application.toml` (user-level, default `~/.config/`)
/// - `$SWE_EDGE_CONFIG_DIR/application.toml` (explicit override, if set)
/// - `SWE_EDGE_*` environment variables (always top priority)
pub fn load_config_xdg(app_name: &str) -> Result<RuntimeConfig, ConfigError> {
    DefaultConfigLoader::xdg(app_name).load()
}

/// Load tenant config following the XDG Base Directory specification.
///
/// Same XDG layer chain as [`load_config_xdg`], with
/// `tenants/<tenant_id>.toml` applied on top at the highest-priority
/// directory where it exists.
pub fn load_tenant_config_xdg(
    app_name: &str,
    tenant_id: &str,
) -> Result<RuntimeConfig, ConfigError> {
    DefaultConfigLoader::xdg(app_name).load_for_tenant(tenant_id)
}

/// Assemble a [`RuntimeManager`] from the supplied config, ingress, egress,
/// and lifecycle monitor.
pub fn runtime_manager(
    config:    RuntimeConfig,
    ingress:   Arc<dyn Input>,
    egress:    Arc<dyn Output>,
    lifecycle: Arc<dyn LifecycleMonitor>,
) -> impl RuntimeManager {
    DefaultRuntimeManager::new(config, ingress, egress, lifecycle)
}

/// Start the daemon and block until a shutdown signal is received.
///
/// Calls [`RuntimeManager::start`], waits for `SIGTERM` or `SIGINT`
/// (Ctrl+C on all platforms), then calls [`RuntimeManager::shutdown`].
/// If shutdown does not complete within `config.shutdown_timeout_secs`,
/// returns [`RuntimeError::ShutdownTimeout`].
///
/// If `ingress` includes an HTTP transport, an Axum HTTP server is
/// spawned automatically against `config.http_bind`. If `ingress` includes
/// a gRPC transport, a gRPC server is spawned against `config.grpc_bind`.
/// Both servers are signalled to drain before `RuntimeManager::shutdown`.
pub async fn run(
    config:    RuntimeConfig,
    ingress:   Arc<dyn Input>,
    egress:    Arc<dyn Output>,
    lifecycle: Arc<dyn LifecycleMonitor>,
) -> RuntimeResult<()> {
    use swe_edge_ingress::{AxumHttpServer, TonicGrpcServer};
    use tokio::sync::oneshot;

    let timeout_secs = config.shutdown_timeout_secs;
    let http_bind    = config.http_bind.clone();
    let grpc_bind    = config.grpc_bind.clone();

    // Spawn the Axum HTTP server if an HTTP handler is wired in.
    let (http_shutdown_tx, http_shutdown_rx) = oneshot::channel::<()>();
    let http_task = ingress.http().map(|handler| {
        let server = AxumHttpServer::new(http_bind, handler);
        tokio::spawn(async move {
            let signal = async move { let _ = http_shutdown_rx.await; };
            if let Err(e) = server.serve(signal).await {
                tracing::error!("HTTP server error: {e}");
            }
        })
    });

    // Spawn the gRPC server if a gRPC handler is wired in.
    let (grpc_shutdown_tx, grpc_shutdown_rx) = oneshot::channel::<()>();
    let grpc_task = ingress.grpc().map(|handler| {
        let server = TonicGrpcServer::new(grpc_bind, handler);
        tokio::spawn(async move {
            let signal = async move { let _ = grpc_shutdown_rx.await; };
            if let Err(e) = server.serve(signal).await {
                tracing::error!("gRPC server error: {e}");
            }
        })
    });

    let mgr    = runtime_manager(config, ingress, egress, lifecycle);
    let result = run_until_signal(mgr, timeout_secs, wait_for_signal()).await;

    // Drain both servers and await their tasks.
    let _ = http_shutdown_tx.send(());
    let _ = grpc_shutdown_tx.send(());
    if let Some(task) = http_task {
        let _ = tokio::time::timeout(Duration::from_secs(5), task).await;
    }
    if let Some(task) = grpc_task {
        let _ = tokio::time::timeout(Duration::from_secs(5), task).await;
    }

    result
}

/// Testable core: start `manager`, await `signal`, then shut down within
/// `shutdown_timeout_secs`. Consumers should prefer [`run`].
pub(crate) async fn run_until_signal<F>(
    manager:              impl RuntimeManager,
    shutdown_timeout_secs: u64,
    signal:               F,
) -> RuntimeResult<()>
where
    F: std::future::Future<Output = ()>,
{
    manager.start().await?;
    tracing::info!("daemon ready — awaiting shutdown signal");
    signal.await;
    tracing::info!(timeout_secs = shutdown_timeout_secs, "shutdown signal received — draining");
    tokio::time::timeout(
        Duration::from_secs(shutdown_timeout_secs),
        manager.shutdown(),
    )
    .await
    .map_err(|_| RuntimeError::ShutdownTimeout(shutdown_timeout_secs))?
}

/// Wait for SIGTERM or SIGINT, whichever arrives first.
async fn wait_for_signal() {
    #[cfg(unix)]
    {
        use tokio::signal::unix::{signal, SignalKind};
        let mut sigterm = match signal(SignalKind::terminate()) {
            Ok(s) => s,
            Err(e) => {
                tracing::warn!("could not register SIGTERM handler: {e} — falling back to SIGINT only");
                let _ = tokio::signal::ctrl_c().await;
                return;
            }
        };
        tokio::select! {
            _ = sigterm.recv()           => tracing::info!("SIGTERM received"),
            _ = tokio::signal::ctrl_c() => tracing::info!("SIGINT received"),
        }
    }
    #[cfg(not(unix))]
    {
        let _ = tokio::signal::ctrl_c().await;
        tracing::info!("SIGINT received");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures::future::BoxFuture;

    use crate::api::types::{RuntimeHealth, RuntimeStatus};

    struct OkManager;
    impl RuntimeManager for OkManager {
        fn start(&self)    -> BoxFuture<'_, RuntimeResult<()>>  { Box::pin(async { Ok(()) }) }
        fn shutdown(&self) -> BoxFuture<'_, RuntimeResult<()>>  { Box::pin(async { Ok(()) }) }
        fn health(&self)   -> BoxFuture<'_, RuntimeHealth> {
            Box::pin(async { RuntimeHealth { status: RuntimeStatus::Running, components: vec![], uptime_secs: 0 } })
        }
    }

    struct FailStartManager;
    impl RuntimeManager for FailStartManager {
        fn start(&self) -> BoxFuture<'_, RuntimeResult<()>> {
            Box::pin(async { Err(RuntimeError::StartFailed("injected".into())) })
        }
        fn shutdown(&self) -> BoxFuture<'_, RuntimeResult<()>> { Box::pin(async { Ok(()) }) }
        fn health(&self)   -> BoxFuture<'_, RuntimeHealth> {
            Box::pin(async { RuntimeHealth { status: RuntimeStatus::Stopped, components: vec![], uptime_secs: 0 } })
        }
    }

    struct HangShutdownManager;
    impl RuntimeManager for HangShutdownManager {
        fn start(&self)    -> BoxFuture<'_, RuntimeResult<()>> { Box::pin(async { Ok(()) }) }
        fn shutdown(&self) -> BoxFuture<'_, RuntimeResult<()>> { Box::pin(std::future::pending()) }
        fn health(&self)   -> BoxFuture<'_, RuntimeHealth> {
            Box::pin(async { RuntimeHealth { status: RuntimeStatus::Running, components: vec![], uptime_secs: 0 } })
        }
    }

    /// @covers: run_until_signal
    #[tokio::test]
    async fn test_run_until_signal_starts_and_shuts_down_cleanly() {
        let result = run_until_signal(OkManager, 30, std::future::ready(())).await;
        assert!(result.is_ok());
    }

    /// @covers: run_until_signal
    #[tokio::test]
    async fn test_run_until_signal_propagates_start_failure() {
        let err = run_until_signal(FailStartManager, 30, std::future::ready(())).await.unwrap_err();
        assert!(matches!(err, RuntimeError::StartFailed(_)));
    }

    /// @covers: run_until_signal shutdown timeout
    #[tokio::test(start_paused = true)]
    async fn test_run_until_signal_returns_shutdown_timeout_when_drain_exceeds_limit() {
        let timeout_secs = 5_u64;
        let fut = run_until_signal(HangShutdownManager, timeout_secs, std::future::ready(()));
        tokio::pin!(fut);
        // Advance past the drain window — timeout must fire.
        tokio::time::advance(Duration::from_secs(timeout_secs + 1)).await;
        let err = fut.await.unwrap_err();
        assert!(matches!(err, RuntimeError::ShutdownTimeout(5)));
    }
}
