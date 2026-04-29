//! `swe_edge_egress_grpc_breaker` — circuit-breaker decorator
//! around any [`GrpcOutbound`](swe_edge_egress_grpc::GrpcOutbound).
//!
//! ## State machine
//!
//! Standard three-state breaker:
//!
//! - **Closed**: requests pass through; consecutive
//!   `Unavailable`/`Internal` failures are counted.  At
//!   `failure_threshold` the breaker trips Open.
//! - **Open**: requests short-circuit with
//!   `GrpcOutboundError::Unavailable` *without* calling the
//!   inner client.  After `cool_down`, the next request promotes
//!   to HalfOpen.
//! - **HalfOpen**: a small number of probe requests are admitted.
//!   `half_open_probe_count` consecutive successes → Closed.
//!   Any failure during probing → back to Open.
//!
//! ## Failure classification
//!
//! Only `Unavailable` (status or transport variant) and
//! `Internal` count as breaker failures.  `ResourceExhausted` is
//! a rate-limit signal that the retry layer handles, not a
//! reason to shed traffic.  `Unauthenticated` / `PermissionDenied`
//! reflect a bad credential, not an unhealthy upstream — the
//! upstream is fine; the call is just unauthorized.
//!
//! ## Observability
//!
//! State transitions emit `tracing` events at INFO (Open, Closed)
//! and DEBUG (HalfOpen probe).  No internal metrics framework —
//! consumers wire `tracing` into whatever they already use.

#![warn(missing_docs)]
#![deny(unsafe_code)]
#![warn(clippy::all)]

mod api;
mod core;
mod gateway;
mod saf;

pub use gateway::*;
