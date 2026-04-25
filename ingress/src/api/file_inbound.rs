//! File inbound trait — counterpart for `core::file`.
//!
//! Concrete file readers live in `core::file`; they implement
//! [`InboundSource`](crate::api::inbound_source::InboundSource).

use std::path::Path;

/// Extension trait for file-based inbound sources.
#[allow(dead_code)]
pub trait FileInbound: Send + Sync {
    /// Returns `true` when the path exists on the local filesystem.
    fn path_exists(&self, path: &Path) -> bool;
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    struct AlwaysPresent;

    impl FileInbound for AlwaysPresent {
        fn path_exists(&self, _path: &Path) -> bool {
            true
        }
    }

    #[test]
    fn test_file_inbound_path_exists_returns_true() {
        let src = AlwaysPresent;
        assert!(src.path_exists(Path::new(".")));
    }
}
