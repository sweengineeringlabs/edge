//! SAF layer — gRPC public facade.

pub use crate::api::port::{GrpcMessageStream, GrpcOutbound, GrpcOutboundError, GrpcOutboundResult};
pub use crate::api::value_object::{GrpcMetadata, GrpcRequest, GrpcResponse, GrpcStatusCode};
pub use crate::core::TonicGrpcClient;
