//! `swe-edge-egress-grpc-auth-mtls` — mTLS-based [`GrpcInboundInterceptor`].
//!
//! The ingress `TonicGrpcServer` already performs the mTLS
//! handshake, parses the client cert, and injects identity into
//! request metadata under documented `x-edge-peer-*` keys.  This
//! crate ships the interceptor that **enforces** that an identity
//! actually arrived: any request that reaches `before_dispatch`
//! without a peer-cert fingerprint is rejected with `Unauthenticated`
//! before the handler runs.
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
