//! `swe-edge-egress-grpc-auth-bearer` — symmetric JWT bearer
//! interceptors for the gRPC stack.
//!
//! Two interceptors ship in this crate:
//!
//! - [`BearerOutboundInterceptor`] — implements
//!   [`swe_edge_egress_grpc::GrpcOutboundInterceptor`].  Signs a JWT
//!   from configured claims (or accepts a pre-minted token) and
//!   injects it into the `authorization` request metadata.
//!
//! - [`BearerInboundInterceptor`] — implements
//!   [`swe_edge_ingress_grpc::GrpcInboundInterceptor`].  Validates
//!   incoming `authorization: Bearer <jwt>` against a configured
//!   secret/key, then surfaces the validated `sub` claim under the
//!   internal metadata key
//!   [`crate::EXTRACTED_BEARER_SUBJECT`] for downstream authz
//!   policies (and *only* after successful verification).
//!
//! Constant-time comparisons (`subtle`) are used for any symmetric
//! shared-secret material in the configuration loaders.
//!
//! See `docs/threat_model.md` for the STRIDE breakdown.

#![warn(missing_docs)]
#![deny(unsafe_code)]
#![warn(clippy::all)]

mod api;
mod core;
mod gateway;
mod saf;

pub use gateway::*;
