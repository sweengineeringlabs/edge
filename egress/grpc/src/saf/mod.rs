//! SAF layer — gRPC public facade.

pub use crate::api::port::{GrpcOutbound, GrpcOutboundError, GrpcOutboundResult};
pub use crate::api::value_object::{GrpcMetadata, GrpcRequest, GrpcResponse, GrpcStatusCode};
