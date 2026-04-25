//! Daemon runner — shared startup sequence for gateway daemons.

use std::future::Future;
use std::net::SocketAddr;
use swe_observ_processes::{ObsrvProcess, DEFAULT_OBSRV_PORT};

/// Context passed to the user's server function.
pub struct DaemonContext {
    pub daemon_id: String,
    pub bind: SocketAddr,
    pub service_name: String,
    pub backend: String,
    pub obsrv_port: u16,
}

/// Builder for configuring and running a daemon.
pub struct DaemonRunner {
    service_name: String,
    bind: String,
    backend: String,
    obsrv_port: u16,
    skip_observability: bool,
}

impl DaemonRunner {
    pub fn new(service_name: impl Into<String>) -> Self {
        Self { service_name: service_name.into(), bind: "0.0.0.0:9000".into(), backend: "sidecar".into(), obsrv_port: DEFAULT_OBSRV_PORT, skip_observability: false }
    }

    pub fn with_bind(mut self, bind: impl Into<String>) -> Self { self.bind = bind.into(); self }

    pub fn with_backend(mut self, backend: impl Into<String>) -> Self { self.backend = backend.into(); self }

    pub fn with_obsrv_port(mut self, port: u16) -> Self { self.obsrv_port = port; self }

    pub fn without_observability(mut self) -> Self { self.skip_observability = true; self }

    pub fn observability_skipped(&self) -> bool { self.skip_observability }

    pub async fn run<F, Fut>(self, server_fn: F) -> Result<(), Box<dyn std::error::Error>>
    where
        F: FnOnce(DaemonContext) -> Fut,
        Fut: Future<Output = Result<(), Box<dyn std::error::Error>>>,
    {
        let bind: SocketAddr = self.bind.parse().map_err(|e| format!("Invalid bind address '{}': {}", self.bind, e))?;
        let daemon_id = uuid::Uuid::new_v4().to_string();

        if self.skip_observability {
            tracing::info!(daemon_id = %daemon_id, service = %self.service_name, "starting daemon (lightweight)");
            let ctx = DaemonContext { daemon_id, bind, service_name: self.service_name, backend: "none".into(), obsrv_port: 0 };
            return server_fn(ctx).await;
        }

        let log_ctx = swe_justobserv_context::LogContext::builder().session_id(daemon_id.clone()).agent_id(&self.service_name).build();
        swe_justobserv_context::with_log_context(log_ctx, async move {
            tracing::info!(daemon_id = %daemon_id, service = %self.service_name, "starting daemon");
            let _obsrv = match self.backend.as_str() {
                "sidecar" => {
                    let p = ObsrvProcess::spawn(self.obsrv_port);
                    if p.is_some() { tracing::info!(port = self.obsrv_port, "obsrv sidecar active"); }
                    else { tracing::info!("obsrv not found — fallback to in-memory"); }
                    p
                }
                _ => { tracing::info!(backend = %self.backend, "observability backend selected"); None }
            };
            let ctx = DaemonContext { daemon_id, bind, service_name: self.service_name, backend: self.backend, obsrv_port: self.obsrv_port };
            server_fn(ctx).await
        }).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_daemon_runner_new_sets_default_bind() {
        let r = DaemonRunner::new("svc");
        assert_eq!(r.bind, "0.0.0.0:9000");
        assert!(!r.skip_observability);
    }

    #[test]
    fn test_without_observability_sets_skip_flag() {
        let r = DaemonRunner::new("svc").without_observability();
        assert!(r.observability_skipped());
    }

    #[tokio::test]
    async fn test_run_without_observability_succeeds() {
        let result = DaemonRunner::new("test").with_bind("127.0.0.1:0").without_observability()
            .run(|ctx| async move {
                assert_eq!(ctx.backend, "none");
                assert_eq!(ctx.obsrv_port, 0);
                Ok(())
            }).await;
        assert!(result.is_ok());
    }
}
