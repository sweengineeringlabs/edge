//! SAF layer — gRPC public facade.

pub use crate::api::interceptor::{GrpcOutboundInterceptor, GrpcOutboundInterceptorChain};
pub use crate::api::port::{GrpcMessageStream, GrpcOutbound, GrpcOutboundError, GrpcOutboundResult};
pub use crate::api::value_object::{
    CompressionMode, GrpcChannelConfig, GrpcMetadata, GrpcRequest, GrpcResponse,
    GrpcStatusCode, KeepAliveConfig, MtlsConfig, DEFAULT_MAX_MESSAGE_BYTES,
};
pub use crate::core::{
    from_tonic_code, from_wire, to_tonic_code, to_wire, GrpcChannelConfigError, TonicGrpcClient,
    TraceContextInterceptor,
};
