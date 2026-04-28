//! SAF layer — gRPC public facade.

pub use crate::api::port::{GrpcMessageStream, GrpcOutbound, GrpcOutboundError, GrpcOutboundResult};
pub use crate::api::value_object::{GrpcMetadata, GrpcRequest, GrpcResponse, GrpcStatusCode};
pub use crate::core::{from_tonic_code, from_wire, to_tonic_code, to_wire, TonicGrpcClient};
