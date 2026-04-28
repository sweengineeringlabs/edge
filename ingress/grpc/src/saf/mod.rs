//! SAF layer — gRPC inbound public facade.

pub use crate::api::handler_adapter::{DecodeFn, EncodeFn, GrpcHandlerAdapter};
pub use crate::api::interceptor::{GrpcInboundInterceptor, GrpcInboundInterceptorChain};
pub use crate::api::port::grpc_inbound::{GrpcInbound, GrpcInboundError, GrpcInboundResult, GrpcHealthCheck, GrpcMessageStream};
pub use crate::api::value_object::{
    is_reserved_peer_key, CompressionMode, GrpcMetadata, GrpcRequest, GrpcResponse,
    GrpcServerConfig, GrpcStatusCode, PeerIdentity, DEFAULT_MAX_CONCURRENT_STREAMS,
    DEFAULT_MAX_MESSAGE_BYTES, PEER_CERT_FINGERPRINT_SHA256, PEER_CN, PEER_IDENTITY,
    PEER_SAN_DNS, PEER_SAN_URI, RESERVED_PEER_PREFIXES,
};
pub use crate::core::grpc_timeout::{parse_grpc_timeout, DEFAULT_DEADLINE};
pub use crate::core::handler_dispatch::HandlerRegistryDispatcher;
pub use crate::core::health_service::{
    HealthAggregate, HealthService, ServingStatus, HEALTH_CHECK_METHOD, HEALTH_WATCH_METHOD,
    WATCH_CHANNEL_CAPACITY,
};
pub use crate::core::interceptor::{
    TraceContextInterceptor, EXTRACTED_TRACEPARENT, EXTRACTED_TRACESTATE, TRACEPARENT, TRACESTATE,
};
pub use crate::core::peer_identity::extract_peer_identity;
pub use crate::core::server::{
    GrpcServerConfigError, TonicGrpcServer, TonicServerError, MAX_MESSAGE_BYTES,
};
pub use crate::core::status_codes::{from_tonic_code, from_wire, map_inbound_error, to_tonic_code, to_wire, SANITIZED_INTERNAL_MSG};
pub use swe_edge_ingress_tls::{IngressTlsConfig, IngressTlsError};
