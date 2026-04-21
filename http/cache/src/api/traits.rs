//! Re-export hub — see rule 161's resolution note for rule 48.
//!
//! The re-export makes the primary trait discoverable at
//! `crate::api::traits::HttpCache` — which is where rule 48 +
//! rule 153 look. Consumers of the trait inside the crate may
//! import from either path; the direct path
//! (`crate::api::http_cache::HttpCache`) is preferred for
//! clarity, which is why this re-export is allowed to appear
//! \"unused\" to rustc.
#![allow(unused_imports)]

pub(crate) use crate::api::http_cache::HttpCache;
