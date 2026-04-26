//! Integration tests for the file outbound domain.

use swe_edge_egress_file::{
    FileInfo, FileStorageConfig, FileStorageType, ListOptions, UploadOptions,
};

/// @covers: FileInfo::new — creates non-directory entry.
#[test]
fn test_file_info_new_creates_file_entry() {
    let info = FileInfo::new("docs/readme.md", 1024);
    assert_eq!(info.path, "docs/readme.md");
    assert_eq!(info.size, 1024);
    assert!(!info.is_directory);
}

/// @covers: FileInfo::directory — creates directory entry.
#[test]
fn test_file_info_directory_is_flagged_as_directory() {
    let info = FileInfo::directory("uploads/");
    assert!(info.is_directory);
    assert_eq!(info.size, 0);
}

/// @covers: FileStorageConfig::memory.
#[test]
fn test_file_storage_config_memory_returns_memory_type() {
    let cfg = FileStorageConfig::memory();
    assert_eq!(cfg.storage_type, FileStorageType::Memory);
    assert!(cfg.root.is_none());
}

/// @covers: UploadOptions::overwrite — flag is set.
#[test]
fn test_upload_options_overwrite_sets_flag() {
    let opts = UploadOptions::default().overwrite();
    assert!(opts.overwrite);
}

/// @covers: ListOptions::with_prefix — prefix is stored.
#[test]
fn test_list_options_with_prefix_stores_prefix() {
    let opts = ListOptions::with_prefix("images/");
    assert_eq!(opts.prefix, Some("images/".to_string()));
}
