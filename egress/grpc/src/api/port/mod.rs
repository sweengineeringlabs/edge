//! gRPC port traits.
pub mod grpc_outbound;
pub use grpc_outbound::{GrpcMessageStream, GrpcOutbound, GrpcOutboundError, GrpcOutboundResult};
