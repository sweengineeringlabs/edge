//! Primary trait re-export hub for `swe_http_auth`.

pub(crate) type AuthStrategyTrait = dyn crate::api::auth_strategy::AuthStrategy;
pub(crate) type HttpAuthTrait = dyn crate::api::http_auth::HttpAuth;
