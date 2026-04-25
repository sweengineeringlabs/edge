//! Primary trait re-export hub and trait definitions for `swe_http_cache`.

pub(crate) type HttpCacheTrait = dyn crate::api::http_cache::HttpCache;

/// Contract for types that capture the parts of an outbound
/// request needed to key and validate cache entries.
pub(crate) trait RequestCapture {
    /// The HTTP method of the captured request.
    fn captured_method(&self) -> &reqwest::Method;
    /// The URL of the captured request.
    fn captured_url(&self) -> &reqwest::Url;
}
