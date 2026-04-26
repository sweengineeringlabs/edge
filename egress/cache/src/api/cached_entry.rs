//! Cached response entry abstraction — counterpart for `core::cached_entry`.
//!
//! The concrete [`CachedEntry`](crate::core::cached_entry::CachedEntry) struct
//! lives in `core::cached_entry` and holds the in-memory shape for RFC 7234
//! response caching (status, headers, body, freshness, ETag, Vary, SWR).
