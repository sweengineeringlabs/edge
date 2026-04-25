//! Local filesystem inbound source.

use std::path::{Path, PathBuf};

use crate::api::error::IngressError;
use crate::api::input::InboundSource;
use crate::api::traits::IngressAdapter;

/// Reads files from the local filesystem.
pub(crate) struct LocalFileSource;

impl InboundSource for LocalFileSource {
    fn scan_files(&self, root: &Path) -> Result<Vec<PathBuf>, IngressError> {
        let mut paths = Vec::new();
        for entry in std::fs::read_dir(root).map_err(IngressError::Io)? {
            let entry = entry.map_err(IngressError::Io)?;
            paths.push(entry.path());
        }
        Ok(paths)
    }

    fn read_file(&self, path: &Path) -> Result<Vec<u8>, IngressError> {
        std::fs::read(path).map_err(IngressError::Io)
    }

    fn file_exists(&self, path: &Path) -> Result<bool, IngressError> {
        Ok(path.exists())
    }
}

impl IngressAdapter for LocalFileSource {}

#[cfg(test)]
mod tests {
    use super::*;

    /// @covers: LocalFileSource::file_exists
    #[test]
    fn test_file_exists_returns_false_for_nonexistent_path() {
        let src = LocalFileSource;
        let result = src.file_exists(Path::new("/tmp/__swe_edge_ingress_nonexistent__"));
        assert!(matches!(result, Ok(false)));
    }

    /// @covers: LocalFileSource::read_file
    #[test]
    fn test_read_file_returns_io_error_for_missing_file() {
        let src = LocalFileSource;
        let result = src.read_file(Path::new("/tmp/__swe_edge_ingress_nonexistent__"));
        assert!(matches!(result, Err(IngressError::Io(_))));
    }
}
