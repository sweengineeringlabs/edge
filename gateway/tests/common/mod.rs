//! Shared test helpers for swe-gateway integration and e2e tests.
//!
//! Files under `tests/common/` are recognized as shared helpers, not
//! standalone test binaries. Cargo does not auto-discover them, and the
//! struct-audit rule 99 check excludes this directory from test-file
//! suffix requirements.

pub mod fixtures;
