//! List options for directory/prefix enumeration.

use serde::{Deserialize, Serialize};

/// Options for listing files.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ListOptions {
    pub prefix: Option<String>,
    pub delimiter: Option<String>,
    pub max_results: Option<usize>,
    pub continuation_token: Option<String>,
    pub include_metadata: bool,
}

impl ListOptions {
    pub fn with_prefix(prefix: impl Into<String>) -> Self {
        Self { prefix: Some(prefix.into()), ..Default::default() }
    }

    pub fn with_max_results(mut self, max: usize) -> Self {
        self.max_results = Some(max); self
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
}
