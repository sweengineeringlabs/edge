//! Primary trait re-export hub for `swe_http_cache`.

pub(crate) type HttpCacheTrait = dyn crate::api::http_cache::HttpCache;
