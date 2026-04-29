//! Builder type declaration (SEA rule 160 — public types live in api/).

use crate::api::breaker_config::GrpcBreakerConfig;

/// Opaque builder for the breaker decorator.
///
/// Construct via [`builder()`](crate::builder) (loads SWE
/// baseline) or
/// [`Builder::with_config`](crate::Builder::with_config) (caller-supplied).
/// Wrap an inner [`GrpcOutbound`](swe_edge_egress_grpc::GrpcOutbound)
/// with [`Builder::wrap`](crate::Builder::wrap) to finalize.
#[derive(Debug)]
pub struct Builder {
    pub(crate) config: GrpcBreakerConfig,
}
