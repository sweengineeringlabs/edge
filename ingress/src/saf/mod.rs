//! SAF layer — inbound public facade.

mod builder;

pub use crate::api::builder::{build_file_input, Builder};
pub use crate::api::daemon::{DaemonContext, DaemonRunner};
pub use crate::api::file::{FileInfo, FileStorageConfig, FileStorageType, ListOptions, ListResult, PresignedUrl};
pub use crate::api::grpc::{GrpcMetadata, GrpcRequest, GrpcResponse, GrpcStatusCode};
pub use crate::api::health_check::{HealthCheck, HealthStatus};
pub use crate::api::http::{HttpAuth, HttpBody, HttpConfig, HttpMethod, HttpRequest, HttpResponse};
pub use crate::api::ingress_error::{IngressError, IngressErrorCode, IngressResult, ResultIngressExt};
pub use crate::api::inbound_source::InboundSource;
pub use crate::api::http_inbound::HttpInbound;
pub use crate::api::metrics::{FieldExtractor, MetricFields, MetricsCollector};
pub use crate::api::middleware::{MiddlewareAction, RequestMiddleware, ResponseMiddleware};
pub use crate::api::pagination::{Pagination, PaginatedResponse};
pub use crate::api::pipeline::{Pipeline, Router};
pub use crate::api::rate_limiter::{RateLimiter, RateLimiterBuilder, RateLimiterSpec};
pub use builder::{file_input, passthrough_validator};
