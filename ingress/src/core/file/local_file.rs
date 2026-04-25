//! Local filesystem inbound source.

use std::path::{Path, PathBuf};

use crate::api::ingress_error::IngressError;
use crate::api::inbound_source::InboundSource;

/// Reads files from the local filesystem.
pub(crate) struct LocalFileSource;

impl InboundSource for LocalFileSource {
    fn scan_files(&self, root: &Path) -> Result<Vec<PathBuf>, IngressError> {
        let mut paths = Vec::new();
        for entry in std::fs::read_dir(root)? {
            let entry = entry?;
            paths.push(entry.path());
        }
        Ok(paths)
    }

    fn read_file(&self, path: &Path) -> Result<Vec<u8>, IngressError> {
        Ok(std::fs::read(path)?)
    }

    fn file_exists(&self, path: &Path) -> Result<bool, IngressError> {
        Ok(path.exists())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_file_exists_returns_false_for_nonexistent_path() {
        let src = LocalFileSource;
        let result = src.file_exists(Path::new("/tmp/__swe_edge_ingress_nonexistent__"));
        assert!(matches!(result, Ok(false)));
    }

    #[test]
    fn test_read_file_returns_io_error_for_missing_file() {
        let src = LocalFileSource;
        let result = src.read_file(Path::new("/tmp/__swe_edge_ingress_nonexistent__"));
        assert!(matches!(result, Err(IngressError::IoError(_))));
    }
}
