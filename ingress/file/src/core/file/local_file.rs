//! Local filesystem FileInbound implementation.

use std::path::{Component, Path, PathBuf};
use std::pin::Pin;

use chrono::{DateTime, Utc};
use futures::future::BoxFuture;
use futures::stream::Stream;

use crate::api::port::file_inbound::{FileHealthCheck, FileInbound, FileInboundError, FileInboundResult};
use crate::api::value_object::{FileInfo, FileStorageConfig, ListOptions, ListResult, PresignedUrl, MAX_LIST_RESULTS};

/// Reads files from the local filesystem.
pub struct LocalFileSource {
    pub(crate) config: FileStorageConfig,
}

impl LocalFileSource {
    /// Create a new `LocalFileSource` with the given storage configuration.
    pub fn new(config: FileStorageConfig) -> Self {
        Self { config }
    }
}

impl FileInbound for LocalFileSource {
    fn read(&self, path: &str) -> BoxFuture<'_, FileInboundResult<Vec<u8>>> {
        let base = PathBuf::from(&self.config.base_path);
        let result = safe_join_existing(&base, path);
        Box::pin(async move {
            let full_path = result?;
            std::fs::read(&full_path).map_err(FileInboundError::IoError)
        })
    }

    fn metadata(&self, path: &str) -> BoxFuture<'_, FileInboundResult<FileInfo>> {
        let base = PathBuf::from(&self.config.base_path);
        let path_owned = path.to_string();
        let result = safe_join_existing(&base, path);
        Box::pin(async move {
            let full_path = result?;
            let meta = std::fs::metadata(&full_path).map_err(FileInboundError::IoError)?;
            let size = meta.len();
            let last_modified: DateTime<Utc> = meta.modified()
                .map(DateTime::from)
                .unwrap_or_else(|_| Utc::now());
            let is_directory = meta.is_dir();
            let mut info = FileInfo::new(path_owned, size);
            info.last_modified = last_modified;
            info.is_directory = is_directory;
            Ok(info)
        })
    }

    fn list(&self, options: ListOptions) -> BoxFuture<'_, FileInboundResult<ListResult>> {
        let base = PathBuf::from(&self.config.base_path);
        let prefix = options.prefix.clone().unwrap_or_default();
        // Honour the caller's limit but never exceed the hard cap.
        let effective_max = options.max_results
            .map(|n| n.min(MAX_LIST_RESULTS))
            .unwrap_or(MAX_LIST_RESULTS);
        Box::pin(async move {
            let root = safe_join_existing(&base, &prefix)?;
            let read_dir = std::fs::read_dir(&root).map_err(FileInboundError::IoError)?;
            let mut files: Vec<FileInfo> = Vec::with_capacity(effective_max);
            let mut is_truncated = false;
            for entry in read_dir {
                let entry = entry.map_err(FileInboundError::IoError)?;
                if files.len() >= effective_max {
                    is_truncated = true;
                    break;
                }
                let path = entry.path();
                let rel = path.to_string_lossy().into_owned();
                let meta = entry.metadata().map_err(FileInboundError::IoError)?;
                files.push(FileInfo::new(rel, meta.len()));
            }
            Ok(ListResult { files, prefixes: vec![], next_continuation_token: None, is_truncated })
        })
    }

    fn exists(&self, path: &str) -> BoxFuture<'_, FileInboundResult<bool>> {
        let base = PathBuf::from(&self.config.base_path);
        let result = safe_join(&base, path);
        Box::pin(async move {
            let full_path = result?;
            Ok(full_path.exists())
        })
    }

    fn presigned_read_url(&self, path: &str, expires_in_secs: u64) -> BoxFuture<'_, FileInboundResult<PresignedUrl>> {
        let base = PathBuf::from(&self.config.base_path);
        let result = safe_join(&base, path);
        Box::pin(async move {
            let full_path = result?;
            Ok(PresignedUrl {
                url: format!("file://{}", full_path.display()),
                expires_at: Utc::now() + chrono::Duration::seconds(expires_in_secs as i64),
                method: "GET".into(),
            })
        })
    }

    fn health_check(&self) -> BoxFuture<'_, FileInboundResult<FileHealthCheck>> {
        let base = self.config.base_path.clone();
        Box::pin(async move {
            if std::path::Path::new(&base).exists() {
                Ok(FileHealthCheck::healthy())
            } else {
                Ok(FileHealthCheck::unhealthy(format!("base path not found: {base}")))
            }
        })
    }

    #[allow(clippy::type_complexity)]
    fn list_stream(
        &self,
        options: ListOptions,
    ) -> BoxFuture<'_, FileInboundResult<Pin<Box<dyn Stream<Item = FileInboundResult<FileInfo>> + Send + '_>>>>
    {
        Box::pin(async move {
            let result = self.list(options).await?;
            let stream: Pin<Box<dyn Stream<Item = FileInboundResult<FileInfo>> + Send + '_>> =
                Box::pin(futures::stream::iter(result.files.into_iter().map(Ok)));
            Ok(stream)
        })
    }
}

/// Resolve `..` and `.` components without touching the filesystem.
fn normalize_path(path: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for component in path.components() {
        match component {
            Component::ParentDir => { out.pop(); }
            Component::CurDir    => {}
            c                    => out.push(c),
        }
    }
    out
}

/// Join `base` + `path` and verify the result stays inside `base`.
///
/// Does NOT require the target to exist and cannot catch symlink escapes —
/// use [`safe_join_existing`] for operations that require existence.
///
/// Security guarantee is strongest for absolute base paths. Relative bases
/// (e.g. `"."`) should only be used in tests; production callers must supply
/// absolute paths.
fn safe_join(base: &Path, path: &str) -> Result<PathBuf, FileInboundError> {
    let trimmed = path.trim_start_matches('/');
    // Empty path means "the base itself" — return it directly to avoid
    // normalize_path collapsing "." to an empty PathBuf.
    if trimmed.is_empty() {
        return Ok(base.to_path_buf());
    }
    let joined    = base.join(trimmed);
    let candidate = normalize_path(&joined);
    if !candidate.starts_with(normalize_path(base)) {
        return Err(FileInboundError::InvalidInput(format!(
            "path escapes the base directory: {path:?}"
        )));
    }
    Ok(candidate)
}

/// Join, verify containment, then canonicalize (follows symlinks, requires
/// the target to exist). Catches both `..` traversal and symlink escapes.
fn safe_join_existing(base: &Path, path: &str) -> Result<PathBuf, FileInboundError> {
    let candidate  = safe_join(base, path)?;
    let canon      = candidate.canonicalize().map_err(FileInboundError::IoError)?;
    let base_canon = base.canonicalize().map_err(FileInboundError::IoError)?;
    if !canon.starts_with(&base_canon) {
        return Err(FileInboundError::InvalidInput(format!(
            "path escapes the base directory via symlink: {path:?}"
        )));
    }
    Ok(canon)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::TempDir;
    use crate::api::value_object::FileStorageConfig;

    fn source_at(dir: &Path) -> LocalFileSource {
        LocalFileSource::new(FileStorageConfig::local(dir.to_string_lossy()))
    }

    fn write_file(dir: &Path, name: &str, content: &[u8]) {
        let path = dir.join(name);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::File::create(&path).unwrap().write_all(content).unwrap();
    }

    // ── existing tests ────────────────────────────────────────────────────

    #[tokio::test]
    async fn test_exists_returns_true_for_current_directory() {
        let dir = TempDir::new().unwrap();
        let src = source_at(dir.path());
        assert!(src.exists("").await.unwrap());
    }

    #[tokio::test]
    async fn test_read_returns_error_for_missing_file() {
        let dir = TempDir::new().unwrap();
        let src = source_at(dir.path());
        assert!(src.read("__nonexistent__.bin").await.is_err());
    }

    #[tokio::test]
    async fn test_health_check_returns_healthy_for_existing_base_path() {
        let dir = TempDir::new().unwrap();
        assert!(source_at(dir.path()).health_check().await.unwrap().healthy);
    }

    #[tokio::test]
    async fn test_health_check_returns_unhealthy_for_missing_base_path() {
        let src = LocalFileSource::new(FileStorageConfig::local("/__nonexistent_dir_xyz__"));
        assert!(!src.health_check().await.unwrap().healthy);
    }

    // ── path traversal ────────────────────────────────────────────────────

    /// @covers: safe_join
    #[tokio::test]
    async fn test_read_rejects_dotdot_traversal() {
        let dir = TempDir::new().unwrap();
        let err = source_at(dir.path()).read("../../etc/passwd").await.unwrap_err();
        assert!(matches!(err, FileInboundError::InvalidInput(_)));
    }

    /// @covers: safe_join
    #[tokio::test]
    async fn test_metadata_rejects_dotdot_traversal() {
        let dir = TempDir::new().unwrap();
        let err = source_at(dir.path()).metadata("../escape").await.unwrap_err();
        assert!(matches!(err, FileInboundError::InvalidInput(_)));
    }

    /// @covers: safe_join
    #[tokio::test]
    async fn test_exists_rejects_dotdot_traversal() {
        let dir = TempDir::new().unwrap();
        let err = source_at(dir.path()).exists("../../sensitive").await.unwrap_err();
        assert!(matches!(err, FileInboundError::InvalidInput(_)));
    }

    /// @covers: safe_join
    #[tokio::test]
    async fn test_list_rejects_dotdot_prefix() {
        let dir = TempDir::new().unwrap();
        let opts = ListOptions::with_prefix("../../etc");
        let err = source_at(dir.path()).list(opts).await.unwrap_err();
        assert!(matches!(err, FileInboundError::InvalidInput(_)));
    }

    /// @covers: safe_join
    #[tokio::test]
    async fn test_presigned_read_url_rejects_dotdot_traversal() {
        let dir = TempDir::new().unwrap();
        let err = source_at(dir.path()).presigned_read_url("../../sensitive", 300).await.unwrap_err();
        assert!(matches!(err, FileInboundError::InvalidInput(_)));
    }

    /// @covers: safe_join — valid nested path stays within base
    #[tokio::test]
    async fn test_read_accepts_nested_path_within_base() {
        let dir = TempDir::new().unwrap();
        write_file(dir.path(), "sub/file.txt", b"hello");
        let data = source_at(dir.path()).read("sub/file.txt").await.unwrap();
        assert_eq!(data, b"hello");
    }

    // ── list cap ──────────────────────────────────────────────────────────

    /// @covers: list effective_max
    #[tokio::test]
    async fn test_list_respects_explicit_max_results() {
        let dir = TempDir::new().unwrap();
        for i in 0..5 { write_file(dir.path(), &format!("f{i}.txt"), b"x"); }
        let opts = ListOptions::default().with_max_results(3);
        let result = source_at(dir.path()).list(opts).await.unwrap();
        assert!(result.files.len() <= 3);
        assert!(result.is_truncated);
    }

    /// @covers: list effective_max default
    #[tokio::test]
    async fn test_list_without_max_results_applies_default_cap() {
        let dir = TempDir::new().unwrap();
        write_file(dir.path(), "a.txt", b"x");
        let opts = ListOptions::default(); // no max_results set
        let result = source_at(dir.path()).list(opts).await.unwrap();
        assert!(result.files.len() <= MAX_LIST_RESULTS);
    }

    /// @covers: list is_truncated false when all results fit
    #[tokio::test]
    async fn test_list_is_not_truncated_when_all_results_fit() {
        let dir = TempDir::new().unwrap();
        write_file(dir.path(), "only.txt", b"x");
        let result = source_at(dir.path()).list(ListOptions::default()).await.unwrap();
        assert!(!result.is_truncated);
    }

    // ── presigned URL expiry ──────────────────────────────────────────────

    /// @covers: presigned_read_url expires_in_secs honoured
    #[tokio::test]
    async fn test_presigned_read_url_uses_provided_expiry() {
        let dir = TempDir::new().unwrap();
        let before = Utc::now();
        let url = source_at(dir.path()).presigned_read_url("", 300).await.unwrap();
        let delta = (url.expires_at - before).num_seconds();
        assert!(delta >= 299 && delta <= 301, "expiry delta {delta}s should be ~300s");
    }

    /// @covers: presigned_read_url zero expiry
    #[tokio::test]
    async fn test_presigned_read_url_zero_expiry_is_valid() {
        let dir = TempDir::new().unwrap();
        let url = source_at(dir.path()).presigned_read_url("", 0).await.unwrap();
        let delta = (url.expires_at - Utc::now()).num_seconds();
        assert!(delta.abs() <= 1, "zero expiry should be at or near now");
    }
}
