//! List options for directory/prefix enumeration.

use serde::{Deserialize, Serialize};

/// Hard cap on entries returned in a single list call.
///
/// Prevents unbounded directory reads regardless of what the caller
/// passes in `max_results`. Implementations must enforce this ceiling.
pub const MAX_LIST_RESULTS: usize = 1_000;

/// Options for listing files.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ListOptions {
    /// Filter results to paths starting with this prefix.
    pub prefix: Option<String>,
    /// Hierarchy delimiter (e.g. `"/"` for directory-style listing).
    pub delimiter: Option<String>,
    /// Maximum number of entries to return (capped at [`MAX_LIST_RESULTS`]).
    pub max_results: Option<usize>,
    /// Opaque token from the previous page, for pagination.
    pub continuation_token: Option<String>,
    /// Whether to fetch per-file metadata in the listing.
    pub include_metadata: bool,
}

impl ListOptions {
    /// Create options filtered to `prefix`.
    pub fn with_prefix(prefix: impl Into<String>) -> Self {
        Self { prefix: Some(prefix.into()), ..Default::default() }
    }

    /// Set a result limit. Values above [`MAX_LIST_RESULTS`] are silently
    /// clamped — callers should not rely on receiving more than that.
    pub fn with_max_results(mut self, max: usize) -> Self {
        self.max_results = Some(max.min(MAX_LIST_RESULTS)); self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// @covers: with_prefix
    #[test]
    fn test_with_prefix_sets_prefix_filter() {
        let opts = ListOptions::with_prefix("docs/");
        assert_eq!(opts.prefix, Some("docs/".to_string()));
    }

    /// @covers: with_max_results
    #[test]
    fn test_with_max_results_limits_result_count() {
        let opts = ListOptions::with_prefix("x").with_max_results(50);
        assert_eq!(opts.max_results, Some(50));
    }

    /// @covers: with_max_results
    #[test]
    fn test_with_max_results_clamps_to_max_list_results() {
        let opts = ListOptions::default().with_max_results(usize::MAX);
        assert_eq!(opts.max_results, Some(MAX_LIST_RESULTS));
    }

    /// @covers: MAX_LIST_RESULTS
    #[test]
    fn test_max_list_results_constant_is_reasonable() {
        assert!(MAX_LIST_RESULTS >= 100);
        assert!(MAX_LIST_RESULTS <= 10_000);
    }
}
