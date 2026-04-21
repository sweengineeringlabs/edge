//! Primary trait re-export hub for `swe_http_retry`.

pub(crate) type HttpRetryTrait = dyn crate::api::http_retry::HttpRetry;
