//! Builder type declaration (SEA rule 160 — public types live in api/).

use crate::api::retry_config::GrpcRetryConfig;

/// Opaque builder for the retry decorator.
///
/// Construct via [`builder()`](crate::builder) (loads SWE
/// baseline) or
/// [`Builder::with_config`](crate::Builder::with_config) (caller-supplied).
/// Wrap an inner [`GrpcOutbound`](swe_edge_egress_grpc::GrpcOutbound)
/// with [`Builder::wrap`](crate::Builder::wrap) to finalize.
#[derive(Debug)]
pub struct Builder {
    pub(crate) config: GrpcRetryConfig,
}
