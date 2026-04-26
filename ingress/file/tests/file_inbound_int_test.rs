//! Integration tests for the file inbound domain.

use swe_edge_ingress_file::{
    FileInfo, FileStorageConfig, FileStorageType, ListOptions,
    FileInbound, FileInboundError, FileHealthCheck, local_file_source,
};

#[tokio::test]
async fn test_local_file_source_exists_returns_true_for_current_dir() {
    let src = local_file_source(".");
    let exists = src.exists("").await.unwrap();
    assert!(exists);
}

#[tokio::test]
async fn test_local_file_source_exists_returns_false_for_missing_file() {
    let src = local_file_source(".");
    let exists = src.exists("__no_such_file_xyz__.bin").await.unwrap();
    assert!(!exists);
}

#[tokio::test]
async fn test_local_file_source_read_returns_error_for_missing_file() {
    let src = local_file_source(".");
    let result = src.read("__no_such_file_xyz__.bin").await;
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), FileInboundError::IoError(_)));
}

#[tokio::test]
async fn test_local_file_source_health_check_healthy_on_existing_base() {
    let src = local_file_source(".");
    let h = src.health_check().await.unwrap();
    assert!(h.healthy);
}

#[tokio::test]
async fn test_local_file_source_health_check_unhealthy_on_missing_base() {
    let src = local_file_source("/__nonexistent_base_dir_xyz__");
    let h = src.health_check().await.unwrap();
    assert!(!h.healthy);
    assert!(h.message.is_some());
}

#[tokio::test]
async fn test_local_file_source_list_returns_entries_for_current_dir() {
    let src = local_file_source(".");
    let opts = ListOptions::default();
    let result = src.list(opts).await.unwrap();
    // Current dir has entries (at minimum Cargo.toml)
    assert!(!result.files.is_empty());
}

#[test]
fn test_file_storage_config_local_creates_local_type() {
    let cfg = FileStorageConfig::local("/tmp");
    assert_eq!(cfg.storage_type, FileStorageType::Local);
    assert_eq!(cfg.base_path, "/tmp");
}

#[test]
fn test_file_info_new_not_directory() {
    let info = FileInfo::new("test.txt", 100);
    assert!(!info.is_directory);
    assert_eq!(info.size, 100);
}

#[test]
fn test_list_options_with_max_results_limits_count() {
    let opts = ListOptions::with_prefix("prefix/").with_max_results(5);
    assert_eq!(opts.max_results, Some(5));
}
