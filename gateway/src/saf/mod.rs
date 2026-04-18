//! Service Abstraction Framework (SAF) module.
//!
//! This module is the **only** public surface of the crate.
//! All public types, traits, and factory functions are re-exported here.

pub mod builders;
pub mod config;

pub use builders::*;
pub use config::{
    expand_env_vars, load_config, load_config_from, load_config_from_str, ConfigError,
    GatewayConfig, SinkConfig, SinkFormat, SinkType,
};

/// Validate a [`GatewayConfig`] — verifies all contextually required fields
/// are present per configuration rules.
///
/// Returns `Ok(())` on success, or [`ConfigError::Validation`] with an
/// actionable message listing every missing field.
///
/// This is the public entry point; the underlying implementation is
/// `pub(crate)` on [`GatewayConfig`] (rule 122).
pub fn validate_config(config: &GatewayConfig) -> Result<(), ConfigError> {
    config.validate()
}

// ── Unified process gateway (from api layer) ──
pub use crate::api::process::{
    Gateway, InputRequest, OutputResponse, PipelineGateway, PipelineReq, PipelineResp,
    ProcessStatus, RequestMetadata, ResponseMetadata,
};

// ── Input/output traits (from api layer) ──
pub use crate::api::input::InputSource;
pub use crate::api::output::OutputSink;

// ── Gateway traits (from api layer) ──
pub use crate::api::traits::DatabaseGateway;
pub use crate::api::traits::DatabaseInbound;
pub use crate::api::traits::DatabaseOutbound;
pub use crate::api::traits::FileGateway;
pub use crate::api::traits::FileInbound;
pub use crate::api::traits::FileOutbound;
pub use crate::api::traits::HttpGateway;
pub use crate::api::traits::HttpInbound;
pub use crate::api::traits::HttpOutbound;
pub use crate::api::traits::NotificationGateway;
pub use crate::api::traits::NotificationInbound;
pub use crate::api::traits::NotificationOutbound;
pub use crate::api::traits::PaymentGateway;
pub use crate::api::traits::PaymentInbound;
pub use crate::api::traits::PaymentOutbound;
pub use crate::api::traits::GrpcGateway;
pub use crate::api::traits::GrpcInbound;
pub use crate::api::traits::GrpcOutbound;

// ── Middleware traits (from api layer) ──
pub use crate::api::middleware::MiddlewareAction;
pub use crate::api::middleware::RequestMiddleware;
pub use crate::api::middleware::ResponseMiddleware;

// ── Daemon runner (from api layer) ──
pub use crate::api::daemon::{DaemonContext, DaemonRunner};

// ── Retry middleware ─────────────────────────────────────────────────────
//
// Public contract types live in crate::api::retry (rule 160); the
// runtime impl stays pub(crate) in crate::core::retry (rule 50).
// saf/ re-exports the api/ types for ergonomics and exposes a factory
// that returns `impl RequestMiddleware` so consumers never name the
// core type (rules 47, 159).

pub use crate::api::retry::{
    default_retry_predicate, BackoffStrategy, RetryMiddlewareBuilder, RetryMiddlewareSpec,
    RetryPredicate,
};

use std::sync::Arc;

/// Attach retry behavior to a middleware pipeline.
///
/// Consumers construct a [`RetryMiddlewareSpec`] via
/// [`RetryMiddlewareBuilder`] and pass it here with an inner
/// middleware. The returned middleware implements
/// [`crate::api::middleware::RequestMiddleware`] and can be slotted
/// into any pipeline.
pub fn wrap_with_retry(
    spec: RetryMiddlewareSpec,
    inner: Arc<dyn crate::api::middleware::RequestMiddleware>,
) -> impl crate::api::middleware::RequestMiddleware {
    crate::core::retry::build_retry_middleware(spec, inner)
}

// ── Rate limiter ─────────────────────────────────────────────────────────
//
// Trait + builder + spec live in crate::api::rate_limit. Concrete bucket
// stays pub(crate) in crate::core::rate_limit. saf re-exports the api/
// types and exposes a factory returning `impl RateLimiter`.
pub use crate::api::rate_limit::{RateLimiter, RateLimiterBuilder, RateLimiterSpec};

/// Construct a rate limiter from a spec. Returned as `impl RateLimiter`
/// so consumers never name the concrete core type.
pub fn make_rate_limiter(spec: RateLimiterSpec) -> impl RateLimiter {
    crate::core::rate_limit::build_rate_limiter(spec)
}

// ── Pipeline (from api layer) ──
//
// Traits live in api/pipeline; default impls stay pub(crate) in core/.
// Factories below return `impl Pipeline` / `impl Router` so consumers
// never name the concrete core types (rules 47, 159).
pub use crate::api::pipeline::{Pipeline, Router};

// ── Metrics (from api layer) ──
//
// Contract types live in api/metrics; the bridge middleware is
// pub(crate) in core/metrics_bridge. `metrics_middleware` returns
// `impl ResponseMiddleware`.
pub use crate::api::metrics::{FieldExtractor, MetricFields, MetricsCollector};

/// Construct the default pipeline: pre-middleware, router, post-middleware.
///
/// Returns `impl Pipeline` so the concrete core type is never named.
pub fn default_pipeline<
    Req: Send + Sync + 'static,
    Resp: Send + Sync + 'static,
    Err: Send + Sync + 'static,
>(
    pre: Vec<Arc<dyn crate::api::middleware::RequestMiddleware<Req, Err, Resp>>>,
    router: Arc<dyn Router<Req, Resp, Err>>,
    post: Vec<Arc<dyn crate::api::middleware::ResponseMiddleware<Resp, Err>>>,
) -> impl Pipeline<Req, Resp, Err> {
    crate::core::pipeline::DefaultPipeline::new(pre, router, post)
}

/// Wrap an async closure as a [`Router`].
///
/// Returns `impl Router` so the concrete core type is never named.
pub fn closure_router<F, Req, Resp, Err>(handler: F) -> impl Router<Req, Resp, Err>
where
    F: for<'a> Fn(&'a Req) -> futures::future::BoxFuture<'a, Result<Resp, Err>>
        + Send
        + Sync
        + 'static,
    Req: Send + Sync + 'static,
    Resp: Send + Sync + 'static,
    Err: Send + Sync + 'static,
{
    crate::core::pipeline::PipelineRouter::new(handler)
}

/// Wrap a synchronous closure as a [`Router`].
///
/// The closure receives a request reference and returns a `Result<Resp, Err>`
/// directly. Use this when dispatch logic doesn't require awaiting —
/// the factory wraps the result in a ready future internally.
///
/// Returns `impl Router` so the concrete core type is never named.
pub fn sync_closure_router<F, Req, Resp, Err>(handler: F) -> impl Router<Req, Resp, Err>
where
    F: Fn(&Req) -> Result<Resp, Err> + Send + Sync + 'static,
    Req: Send + Sync + 'static,
    Resp: Send + Sync + 'static,
    Err: Send + Sync + 'static,
{
    crate::core::pipeline::PipelineRouter::new(move |req: &Req| {
        let result = handler(req);
        Box::pin(async move { result })
    })
}

/// Construct a metrics response middleware that records fields extracted
/// from each response via the given collector.
///
/// Returns `impl ResponseMiddleware` so the concrete core type is never named.
pub fn metrics_middleware(
    collector: Arc<dyn MetricsCollector>,
    extractor: FieldExtractor,
) -> impl crate::api::middleware::ResponseMiddleware {
    crate::core::metrics_bridge::MetricsResponseMiddleware::new(collector, extractor)
}

// ── Common types (from api layer) ──
pub use crate::api::types::GatewayError;
pub use crate::api::types::GatewayErrorCode;
pub use crate::api::types::GatewayResult;

// ── HTTP value types (from api layer) ──
// Required to construct HttpRequest / inspect HttpResponse from outside
// the crate. Without these the HttpOutbound / HttpInbound traits are
// callable but their parameter and return types are unnameable.
pub use crate::api::http::{
    HttpAuth, HttpBody, HttpConfig, HttpMethod, HttpRequest, HttpResponse,
};
pub use crate::api::types::HealthCheck;
pub use crate::api::types::HealthStatus;
pub use crate::api::types::IntoGatewayError;
pub use crate::api::types::MockFailureMode;
pub use crate::api::types::PaginatedResponse;
pub use crate::api::types::Pagination;
pub use crate::api::types::ResultGatewayExt;

// ── Domain types (from api layer) ──
pub mod database {
    //! Database domain types.
    pub use crate::api::database::*;
}

pub mod file {
    //! File domain types.
    pub use crate::api::file::*;
}

pub mod http {
    //! HTTP domain types.
    pub use crate::api::http::*;
}

pub mod notification {
    //! Notification domain types.
    pub use crate::api::notification::*;
}

pub mod payment {
    //! Payment domain types.
    pub use crate::api::payment::*;
}

pub mod grpc {
    //! gRPC domain types.
    pub use crate::api::grpc::*;
}

// ── Provider traits ──
pub use crate::provider::{LazyInit, LazyInitWithConfig, StatefulProvider, StatelessProvider};

// ── State management ──
pub use crate::state::{CachedService, ConfiguredCache};

// ── Async-to-sync bridge ──

/// Run an async future synchronously on a shared single-threaded tokio runtime.
///
/// This is the canonical async→sync bridge for consumer crates that use
/// `OutputSink` or other async gateway traits from synchronous code.
/// The runtime is created once and reused for the lifetime of the process.
pub fn block_on<F: std::future::Future>(f: F) -> F::Output {
    use std::sync::OnceLock;
    static RT: OnceLock<tokio::runtime::Runtime> = OnceLock::new();
    let rt = RT.get_or_init(|| {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("Failed to create swe-gateway tokio runtime")
    });
    rt.block_on(f)
}

// ── Streaming support ──
/// A boxed async stream of gateway results.
pub type GatewayStream<'a, T> =
    std::pin::Pin<Box<dyn futures::stream::Stream<Item = GatewayResult<T>> + Send + 'a>>;
pub use futures::stream::Stream;
pub use futures::stream::StreamExt;

// ── Async trait re-export ──
pub use async_trait::async_trait;

// ── Auth (sst-sdk backed) ──
#[cfg(feature = "auth")]
pub use crate::api::auth::{AuthClaims, CredentialExtractor};
#[cfg(feature = "auth")]
pub use crate::core::auth_middleware::AuthMiddleware;
#[cfg(feature = "auth")]
pub use sst_sdk::{Authenticator, Authorizer, Credentials, AuthnResult, AuthContext, Permission, AuthResult};
