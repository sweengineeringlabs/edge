//! Inbound source trait — reads files and scans directories.

use std::path::{Path, PathBuf};

use crate::api::ingress_error::IngressError;

/// Reads inbound data from a source (filesystem, cloud storage, etc.).
pub trait InboundSource: Send + Sync {
    /// Recursively scan a directory and return all file paths.
    fn scan_files(&self, root: &Path) -> Result<Vec<PathBuf>, IngressError>;

    /// Read the contents of a file as raw bytes.
    fn read_file(&self, path: &Path) -> Result<Vec<u8>, IngressError>;

    /// Check whether a file exists at the given path.
    fn file_exists(&self, path: &Path) -> Result<bool, IngressError>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    struct AlwaysEmpty;
    impl InboundSource for AlwaysEmpty {
        fn scan_files(&self, _root: &Path) -> Result<Vec<PathBuf>, IngressError> { Ok(vec![]) }
        fn read_file(&self, _path: &Path) -> Result<Vec<u8>, IngressError> { Ok(vec![]) }
        fn file_exists(&self, _path: &Path) -> Result<bool, IngressError> { Ok(false) }
    }

    #[test]
    fn test_inbound_source_file_exists_returns_false_for_stub() {
        let src = AlwaysEmpty;
        assert_eq!(src.file_exists(Path::new("no_such_file")).unwrap(), false);
    }

    #[test]
    fn test_inbound_source_scan_files_returns_empty_for_stub() {
        let src = AlwaysEmpty;
        assert!(src.scan_files(Path::new(".")).unwrap().is_empty());
    }

    #[test]
    fn test_inbound_source_read_file_returns_empty_for_stub() {
        let src = AlwaysEmpty;
        assert!(src.read_file(Path::new("x")).unwrap().is_empty());
    }
}
