//! Options for file upload operations.

use serde::{Deserialize, Serialize};

use super::file_metadata::FileMetadata;

/// Options controlling a file write/upload operation.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct UploadOptions {
    pub metadata: FileMetadata,
    pub overwrite: bool,
    pub part_size_bytes: Option<usize>,
}

impl UploadOptions {
    pub fn with_metadata(mut self, metadata: FileMetadata) -> Self {
        self.metadata = metadata; self
    }

    pub fn overwrite(mut self) -> Self {
        self.overwrite = true; self
    }

    pub fn with_part_size(mut self, bytes: usize) -> Self {
        self.part_size_bytes = Some(bytes); self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// @covers: overwrite
    #[test]
    fn test_overwrite_sets_overwrite_flag() {
        let opts = UploadOptions::default().overwrite();
        assert!(opts.overwrite);
    }

    /// @covers: with_part_size
    #[test]
    fn test_with_part_size_sets_part_size_bytes() {
        let opts = UploadOptions::default().with_part_size(5 * 1024 * 1024);
        assert_eq!(opts.part_size_bytes, Some(5 * 1024 * 1024));
    }
}
