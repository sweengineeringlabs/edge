//! Primary trait re-export hub for `swe_http_breaker`.

pub(crate) type HttpBreakerTrait = dyn crate::api::http_breaker::HttpBreaker;
