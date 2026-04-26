//! SAF layer — gRPC inbound public facade.

pub use crate::api::value_object::{GrpcMetadata, GrpcRequest, GrpcResponse, GrpcStatusCode};
pub use crate::api::port::grpc_inbound::{GrpcInbound, GrpcInboundError, GrpcInboundResult, GrpcHealthCheck};
