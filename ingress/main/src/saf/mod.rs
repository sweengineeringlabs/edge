//! SAF layer — inbound public facade.

mod builder;

pub use crate::api::builder::{build_file_input, Builder};
pub use crate::api::daemon::{DaemonContext, DaemonRunner};
pub use crate::api::health_check::{HealthCheck, HealthStatus};
pub use crate::api::ingress_error::{IngressError, IngressErrorCode, IngressResult, ResultIngressExt};
pub use crate::api::inbound_source::InboundSource;
pub use crate::api::metrics::{FieldExtractor, MetricFields, MetricsCollector};
pub use crate::api::middleware::{MiddlewareAction, RequestMiddleware, ResponseMiddleware};
pub use crate::api::pagination::{Pagination, PaginatedResponse};
pub use crate::api::pipeline::{Pipeline, Router};
pub use crate::api::rate_limiter::{RateLimiter, RateLimiterBuilder, RateLimiterSpec};
pub use builder::{file_input, passthrough_validator};

// Domain crate re-exports
pub use swe_edge_ingress_http::{HttpAuth, HttpBody, FormPart, HttpConfig, HttpHealthCheck, HttpMethod, HttpRequest, HttpResponse, HttpInbound, HttpInboundError, HttpInboundResult, AxumHttpServer, AxumServerError, MAX_BODY_BYTES};
pub use swe_edge_ingress_grpc::{GrpcMetadata, GrpcRequest, GrpcResponse, GrpcStatusCode, GrpcInbound, GrpcInboundError, GrpcInboundResult, GrpcHealthCheck, TonicGrpcServer, TonicServerError, MAX_MESSAGE_BYTES};
pub use swe_edge_ingress_file::{FileInfo, FileMetadata, FileStorageConfig, FileStorageType, ListOptions, ListResult, PresignedUrl, FileInbound, FileInboundError, FileInboundResult, FileHealthCheck};
