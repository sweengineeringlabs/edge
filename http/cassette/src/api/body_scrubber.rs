//! Body scrubbing abstraction — counterpart for `core::body_scrubber`.
//!
//! The concrete `scrub_body` function lives in `core::body_scrubber`.
//! It applies dot-path removal to JSON request bodies before hashing,
//! eliminating non-deterministic fields from cassette key computation.
