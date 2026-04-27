//! SAF layer — gRPC inbound public facade.

pub use crate::api::value_object::{GrpcMetadata, GrpcRequest, GrpcResponse, GrpcStatusCode};
pub use crate::api::port::grpc_inbound::{GrpcInbound, GrpcInboundError, GrpcInboundResult, GrpcHealthCheck, GrpcMessageStream};
pub use crate::core::server::{TonicGrpcServer, TonicServerError, MAX_MESSAGE_BYTES};
pub use swe_edge_ingress_tls::{IngressTlsConfig, IngressTlsError};
