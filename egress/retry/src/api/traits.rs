//! Primary trait re-export hub for `swe_edge_egress_retry`.

pub(crate) type HttpRetryTrait = dyn crate::api::http_retry::HttpRetry;
