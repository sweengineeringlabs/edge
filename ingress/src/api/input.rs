//! Inbound source trait — reads files and scans directories.

use std::path::{Path, PathBuf};

use crate::api::error::IngressError;

/// Reads inbound data from a source (filesystem, cloud storage, etc.).
pub trait InboundSource: Send + Sync {
    /// Recursively scan a directory and return all file paths.
    fn scan_files(&self, root: &Path) -> Result<Vec<PathBuf>, IngressError>;

    /// Read the contents of a file as raw bytes.
    fn read_file(&self, path: &Path) -> Result<Vec<u8>, IngressError>;

    /// Check whether a file exists at the given path.
    fn file_exists(&self, path: &Path) -> Result<bool, IngressError>;
}
