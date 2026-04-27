//! Result of a file listing operation.

use serde::{Deserialize, Serialize};

use super::file_info::FileInfo;

/// Result of a list operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListResult {
    /// Files (or objects) in the listing.
    pub files: Vec<FileInfo>,
    /// Common prefixes (virtual directories) in the listing.
    pub prefixes: Vec<String>,
    /// Token for the next page, or `None` when no more pages exist.
    pub next_continuation_token: Option<String>,
    /// `true` when more results are available beyond this page.
    pub is_truncated: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_list_result_files_is_empty_by_default_on_construction() {
        let r = ListResult { files: vec![], prefixes: vec![], next_continuation_token: None, is_truncated: false };
        assert!(r.files.is_empty());
        assert!(!r.is_truncated);
    }
}
