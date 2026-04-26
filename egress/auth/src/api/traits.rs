//! Primary trait re-export hub and trait definitions for `swe_edge_egress_auth`.

pub(crate) type AuthStrategyTrait = dyn crate::api::auth_strategy::AuthStrategy;
pub(crate) type HttpAuthTrait = dyn crate::api::http_auth::HttpAuth;

/// Groups types that are full HTTP auth processors — they
/// implement [`HttpAuth`][crate::api::http_auth::HttpAuth] and
/// can be placed behind an `Arc<dyn HttpAuth>` inside the
/// middleware. Core implementations declare `impl AuthProcessor`.
pub(crate) trait AuthProcessor: crate::api::http_auth::HttpAuth {}
