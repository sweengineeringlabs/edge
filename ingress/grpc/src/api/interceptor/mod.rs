//! gRPC inbound interceptor trait.

pub(crate) mod grpc_inbound_interceptor;
pub use grpc_inbound_interceptor::{
    AuthorizationInterceptor, GrpcInboundInterceptor, GrpcInboundInterceptorChain,
};
