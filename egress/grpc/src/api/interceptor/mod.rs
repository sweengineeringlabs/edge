//! gRPC outbound interceptor trait.

pub mod grpc_outbound_interceptor;
pub use grpc_outbound_interceptor::{GrpcOutboundInterceptor, GrpcOutboundInterceptorChain};
