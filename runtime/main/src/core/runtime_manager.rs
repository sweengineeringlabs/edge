use std::sync::Arc;
use std::time::Instant;

use futures::future::BoxFuture;
use parking_lot::Mutex;

use edge_proxy::{HealthStatus, LifecycleMonitor};

use crate::api::error::{RuntimeError, RuntimeResult};
use crate::api::runtime_manager::RuntimeManager;
use crate::api::types::{RuntimeConfig, RuntimeHealth, RuntimeStatus};
use crate::api::types::runtime_health::ComponentHealth;
use crate::api::input::Input;
use crate::api::output::Output;

pub(crate) struct DefaultRuntimeManager {
    config:     RuntimeConfig,
    ingress:    Arc<dyn Input>,
    egress:     Arc<dyn Output>,
    lifecycle:  Arc<dyn LifecycleMonitor>,
    status:     Arc<Mutex<RuntimeStatus>>,
    started_at: Arc<Mutex<Option<Instant>>>,
}

impl DefaultRuntimeManager {
    pub(crate) fn new(
        config:    RuntimeConfig,
        ingress:   Arc<dyn Input>,
        egress:    Arc<dyn Output>,
        lifecycle: Arc<dyn LifecycleMonitor>,
    ) -> Self {
        Self {
            config,
            ingress,
            egress,
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

            if !self.ingress.has_any() {
                return Err(RuntimeError::StartFailed(
                    "no ingress transport configured — add http, grpc, or file".into(),
                ));
            }

            self.lifecycle.start_background_tasks().await;

            // Probe each configured ingress transport to surface misconfigurations early.
            if let Some(h) = self.ingress.http() { let _ = h.health_check().await; }
            if let Some(g) = self.ingress.grpc() { let _ = g.health_check().await; }
            if let Some(f) = self.ingress.file() { let _ = f.health_check().await; }

            // Probe egress.
            let _ = self.egress.http().health_check().await;
            if let Some(g) = self.egress.grpc() { let _ = g.health_check().await; }

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

            let mut components: Vec<ComponentHealth> = report
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

            // Report health for each configured ingress transport.
            if let Some(h) = self.ingress.http() {
                match h.health_check().await {
                    Ok(_)  => components.push(ComponentHealth::healthy("ingress.http")),
                    Err(e) => components.push(ComponentHealth::unhealthy("ingress.http", e.to_string())),
                }
            }
            if let Some(g) = self.ingress.grpc() {
                match g.health_check().await {
                    Ok(_)  => components.push(ComponentHealth::healthy("ingress.grpc")),
                    Err(e) => components.push(ComponentHealth::unhealthy("ingress.grpc", e.to_string())),
                }
            }
            if let Some(f) = self.ingress.file() {
                match f.health_check().await {
                    Ok(_)  => components.push(ComponentHealth::healthy("ingress.file")),
                    Err(e) => components.push(ComponentHealth::unhealthy("ingress.file", e.to_string())),
                }
            }

            // Report egress transport health.
            match self.egress.http().health_check().await {
                Ok(_)  => components.push(ComponentHealth::healthy("egress.http")),
                Err(e) => components.push(ComponentHealth::unhealthy("egress.http", e.to_string())),
            }
            if let Some(g) = self.egress.grpc() {
                match g.health_check().await {
                    Ok(_)  => components.push(ComponentHealth::healthy("egress.grpc")),
                    Err(e) => components.push(ComponentHealth::unhealthy("egress.grpc", e.to_string())),
                }
            }

            RuntimeHealth { status, components, uptime_secs }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use async_trait::async_trait;
    use edge_proxy::{HealthReport, LifecycleError};
    use futures::future::BoxFuture;
    use swe_edge_ingress::{
        HttpHealthCheck, HttpInboundResult, HttpRequest, HttpResponse,
        GrpcInbound, GrpcInboundResult, GrpcHealthCheck, GrpcRequest, GrpcResponse, GrpcMetadata,
        FileInbound, FileInboundResult, FileHealthCheck, FileInfo, ListOptions, ListResult, PresignedUrl,
    };
    use swe_edge_egress::{
        GrpcOutbound, GrpcOutboundError, GrpcOutboundResult,
        GrpcRequest as EgressGrpcRequest, GrpcResponse as EgressGrpcResponse,
        GrpcMetadata as EgressGrpcMetadata,
        HttpOutboundResult, HttpRequest as EgressReq, HttpResponse as EgressResp,
    };
    use chrono::Utc;
    use super::super::input::DefaultInput;
    use super::super::output::DefaultOutput;

    struct StubLifecycle;

    #[async_trait]
    impl LifecycleMonitor for StubLifecycle {
        async fn health(&self) -> HealthReport { HealthReport::from_components(vec![]) }
        async fn start_background_tasks(&self) {}
        async fn shutdown(&self) -> Result<(), LifecycleError> { Ok(()) }
    }

    struct StubHttpInbound;
    impl swe_edge_ingress::HttpInbound for StubHttpInbound {
        fn handle(&self, _: HttpRequest) -> BoxFuture<'_, HttpInboundResult<HttpResponse>> {
            Box::pin(async { Ok(HttpResponse::new(200, vec![])) })
        }
        fn health_check(&self) -> BoxFuture<'_, HttpInboundResult<HttpHealthCheck>> {
            Box::pin(async { Ok(HttpHealthCheck::healthy()) })
        }
    }

    struct StubGrpcInbound;
    impl GrpcInbound for StubGrpcInbound {
        fn handle_unary(&self, _: GrpcRequest) -> BoxFuture<'_, GrpcInboundResult<GrpcResponse>> {
            Box::pin(async {
                Ok(GrpcResponse { body: vec![], metadata: GrpcMetadata { headers: HashMap::new() } })
            })
        }
        fn health_check(&self) -> BoxFuture<'_, GrpcInboundResult<GrpcHealthCheck>> {
            Box::pin(async { Ok(GrpcHealthCheck { healthy: true, message: None }) })
        }
    }

    struct StubFileInbound;
    impl FileInbound for StubFileInbound {
        fn read(&self, _: &str) -> BoxFuture<'_, FileInboundResult<Vec<u8>>> {
            Box::pin(async { Ok(vec![]) })
        }
        fn metadata(&self, path: &str) -> BoxFuture<'_, FileInboundResult<FileInfo>> {
            let p = path.to_string();
            Box::pin(async move { Ok(FileInfo::new(p, 0)) })
        }
        fn list(&self, _: ListOptions) -> BoxFuture<'_, FileInboundResult<ListResult>> {
            Box::pin(async {
                Ok(ListResult { files: vec![], prefixes: vec![], next_continuation_token: None, is_truncated: false })
            })
        }
        fn exists(&self, _: &str) -> BoxFuture<'_, FileInboundResult<bool>> {
            Box::pin(async { Ok(false) })
        }
        fn presigned_read_url(&self, _: &str, expires_in_secs: u64) -> BoxFuture<'_, FileInboundResult<PresignedUrl>> {
            Box::pin(async move {
                Ok(PresignedUrl { url: "file:///stub".into(), expires_at: Utc::now() + chrono::Duration::seconds(expires_in_secs as i64), method: "GET".into() })
            })
        }
        fn health_check(&self) -> BoxFuture<'_, FileInboundResult<FileHealthCheck>> {
            Box::pin(async { Ok(FileHealthCheck::healthy()) })
        }
    }

    struct StubHttpOutbound;
    impl swe_edge_egress::HttpOutbound for StubHttpOutbound {
        fn send(&self, _: EgressReq) -> BoxFuture<'_, HttpOutboundResult<EgressResp>> {
            Box::pin(async { Ok(EgressResp::new(200, vec![])) })
        }
        fn health_check(&self) -> BoxFuture<'_, HttpOutboundResult<()>> {
            Box::pin(async { Ok(()) })
        }
    }

    struct StubGrpcOutbound;
    impl GrpcOutbound for StubGrpcOutbound {
        fn call_unary(&self, _: EgressGrpcRequest) -> BoxFuture<'_, GrpcOutboundResult<EgressGrpcResponse>> {
            Box::pin(async { Ok(EgressGrpcResponse { body: vec![], metadata: EgressGrpcMetadata::default() }) })
        }
        fn health_check(&self) -> BoxFuture<'_, GrpcOutboundResult<()>> {
            Box::pin(async { Ok(()) })
        }
    }

    struct DownGrpcOutbound;
    impl GrpcOutbound for DownGrpcOutbound {
        fn call_unary(&self, _: EgressGrpcRequest) -> BoxFuture<'_, GrpcOutboundResult<EgressGrpcResponse>> {
            Box::pin(async { Err(GrpcOutboundError::Unavailable("down".into())) })
        }
        fn health_check(&self) -> BoxFuture<'_, GrpcOutboundResult<()>> {
            Box::pin(async { Err(GrpcOutboundError::Unavailable("unreachable".into())) })
        }
    }

    fn make_manager() -> DefaultRuntimeManager {
        DefaultRuntimeManager::new(
            RuntimeConfig::default().with_systemd_notify(false),
            Arc::new(DefaultInput::new_http(Arc::new(StubHttpInbound))),
            Arc::new(DefaultOutput::new_http(Arc::new(StubHttpOutbound))),
            Arc::new(StubLifecycle),
        )
    }

    /// @covers: new
    #[test]
    fn test_new_creates_stopped_status() {
        let m = make_manager();
        assert_eq!(*m.status.lock(), RuntimeStatus::Stopped);
        assert!(m.started_at.lock().is_none());
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
        m.shutdown().await.expect("first");
        m.shutdown().await.expect("second");
    }

    /// @covers: start no-ingress guard
    #[tokio::test]
    async fn test_start_fails_when_no_ingress_configured() {
        let m = DefaultRuntimeManager::new(
            RuntimeConfig::default(),
            Arc::new(DefaultInput::empty()),
            Arc::new(DefaultOutput::new_http(Arc::new(StubHttpOutbound))),
            Arc::new(StubLifecycle),
        );
        let err = m.start().await.unwrap_err();
        assert!(matches!(err, RuntimeError::StartFailed(_)));
        assert!(err.to_string().contains("no ingress transport"));
    }

    #[tokio::test]
    async fn test_health_includes_ingress_http_and_egress_components() {
        let m = make_manager();
        m.start().await.expect("start ok");
        let h = m.health().await;
        let names: Vec<&str> = h.components.iter().map(|c| c.name.as_str()).collect();
        assert!(names.contains(&"ingress.http"));
        assert!(names.contains(&"egress.http"));
    }

    /// @covers: grpc-only ingress
    #[tokio::test]
    async fn test_health_reports_grpc_when_only_grpc_configured() {
        let m = DefaultRuntimeManager::new(
            RuntimeConfig::default(),
            Arc::new(DefaultInput::new_grpc(Arc::new(StubGrpcInbound))),
            Arc::new(DefaultOutput::new_http(Arc::new(StubHttpOutbound))),
            Arc::new(StubLifecycle),
        );
        m.start().await.expect("start ok");
        let h = m.health().await;
        let names: Vec<&str> = h.components.iter().map(|c| c.name.as_str()).collect();
        assert!(names.contains(&"ingress.grpc"));
        assert!(!names.contains(&"ingress.http"));
        assert!(!names.contains(&"ingress.file"));
    }

    /// @covers: all-transport ingress
    #[tokio::test]
    async fn test_health_reports_all_configured_transports() {
        let m = DefaultRuntimeManager::new(
            RuntimeConfig::default(),
            Arc::new(
                DefaultInput::new_http(Arc::new(StubHttpInbound))
                    .with_grpc(Arc::new(StubGrpcInbound))
                    .with_file(Arc::new(StubFileInbound)),
            ),
            Arc::new(DefaultOutput::new_http(Arc::new(StubHttpOutbound))),
            Arc::new(StubLifecycle),
        );
        m.start().await.expect("start ok");
        let h = m.health().await;
        let names: Vec<&str> = h.components.iter().map(|c| c.name.as_str()).collect();
        assert!(names.contains(&"ingress.http"));
        assert!(names.contains(&"ingress.grpc"));
        assert!(names.contains(&"ingress.file"));
    }

    /// @covers: egress.grpc healthy
    #[tokio::test]
    async fn test_health_reports_egress_grpc_when_configured() {
        let m = DefaultRuntimeManager::new(
            RuntimeConfig::default(),
            Arc::new(DefaultInput::new_http(Arc::new(StubHttpInbound))),
            Arc::new(DefaultOutput::new_http(Arc::new(StubHttpOutbound))
                .with_grpc(Arc::new(StubGrpcOutbound))),
            Arc::new(StubLifecycle),
        );
        m.start().await.expect("start ok");
        let h = m.health().await;
        let names: Vec<&str> = h.components.iter().map(|c| c.name.as_str()).collect();
        assert!(names.contains(&"egress.grpc"), "egress.grpc must appear in health report");
        assert!(names.contains(&"egress.http"));
    }

    /// @covers: egress.grpc unhealthy
    #[tokio::test]
    async fn test_health_reports_egress_grpc_unhealthy_when_down() {
        let m = DefaultRuntimeManager::new(
            RuntimeConfig::default(),
            Arc::new(DefaultInput::new_http(Arc::new(StubHttpInbound))),
            Arc::new(DefaultOutput::new_http(Arc::new(StubHttpOutbound))
                .with_grpc(Arc::new(DownGrpcOutbound))),
            Arc::new(StubLifecycle),
        );
        m.start().await.expect("start ok");
        let h = m.health().await;
        let grpc_comp = h.components.iter().find(|c| c.name == "egress.grpc")
            .expect("egress.grpc component must be present");
        assert!(
            !grpc_comp.healthy,
            "egress.grpc must be unhealthy when health_check returns Err"
        );
    }
}
