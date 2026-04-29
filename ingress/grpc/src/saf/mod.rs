//! SAF layer — gRPC inbound public facade.

pub use crate::api::value_object::{GrpcMetadata, GrpcRequest, GrpcResponse, GrpcStatusCode};
pub use crate::api::port::grpc_inbound::{GrpcInbound, GrpcInboundError, GrpcInboundResult, GrpcHealthCheck, GrpcMessageStream};
pub use crate::core::server::{TonicGrpcServer, TonicServerError, MAX_MESSAGE_BYTES};
pub use crate::core::status_codes::{from_tonic_code, from_wire, map_inbound_error, to_tonic_code, to_wire, SANITIZED_INTERNAL_MSG};
pub use crate::core::grpc_timeout::{parse_grpc_timeout, DEFAULT_DEADLINE};
pub use swe_edge_ingress_tls::{IngressTlsConfig, IngressTlsError};
