//! Token bucket abstraction — counterpart for `core::token_bucket`.
//!
//! The concrete `TokenBucket` struct lives in `core::token_bucket`;
//! it implements [`RateBucketOps`](crate::api::traits::RateBucketOps)
//! for per-host token-based rate limiting.
