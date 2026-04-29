//! gRPC core adapter implementations.
pub(crate) mod client;
pub(crate) mod status_codes;
pub use client::TonicGrpcClient;
pub use status_codes::{from_tonic_code, from_wire, to_tonic_code, to_wire};
