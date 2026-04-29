//! Primary trait re-export hub for `swe_edge_egress_grpc_breaker`.
//!
//! This crate's primary trait is
//! [`GrpcOutbound`](swe_edge_egress_grpc::GrpcOutbound) — declared
//! upstream in `swe-edge-egress-grpc`.  We re-export it
//! `pub(crate)` here so the SEA layer-boundary checker can find
//! a trait declaration in `api/traits.rs`.  Consumers of this
//! crate should depend on `swe-edge-egress-grpc` directly for the
//! trait — this crate's job is to wrap implementors, not to
//! re-publish the contract.

#[allow(unused_imports)]
pub(crate) use swe_edge_egress_grpc::GrpcOutbound;
