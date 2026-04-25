//! File egress trait — counterpart for `core::file`.
//!
//! Concrete file writers live in `core::file`; they implement
//! [`OutboundSink`](crate::api::outbound_sink::OutboundSink).

use std::path::Path;

/// Extension trait for file-based outbound sinks.
#[allow(dead_code)]
pub trait FileOutbound: Send + Sync {
    /// Returns `true` when the path is writable on the local filesystem.
    fn path_writable(&self, path: &Path) -> bool;
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    struct AlwaysWritable;

    impl FileOutbound for AlwaysWritable {
        fn path_writable(&self, _path: &Path) -> bool {
            true
        }
    }

    #[test]
    fn test_file_outbound_path_writable_returns_true() {
        let sink = AlwaysWritable;
        assert!(sink.path_writable(Path::new(".")));
    }
}
