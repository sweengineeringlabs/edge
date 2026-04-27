//! Pagination types for list operations.

use serde::{Deserialize, Serialize};

/// Pagination parameters for list operations.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Pagination {
    /// Zero-based index of the first item to return.
    pub offset: usize,
    /// Maximum number of items to return.
    pub limit: usize,
}

impl Pagination {
    /// Create pagination with explicit offset and limit.
    pub fn new(offset: usize, limit: usize) -> Self { Self { offset, limit } }

    /// Create pagination starting at offset 0 with the given limit.
    pub fn first(limit: usize) -> Self { Self { offset: 0, limit } }
}

/// A paginated response containing items and metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaginatedResponse<T> {
    /// Items in this page.
    pub items: Vec<T>,
    /// Total number of items across all pages.
    pub total: usize,
    /// Offset of the first item in this page.
    pub offset: usize,
    /// Maximum items per page requested.
    pub limit: usize,
    /// `true` when more items exist beyond this page.
    pub has_more: bool,
}

impl<T> PaginatedResponse<T> {
    /// Create a paginated response, computing [`has_more`](Self::has_more) automatically.
    pub fn new(items: Vec<T>, total: usize, offset: usize, limit: usize) -> Self {
        let has_more = offset + items.len() < total;
        Self { items, total, offset, limit, has_more }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// @covers: first
    #[test]
    fn test_first_creates_zero_offset_pagination() {
        let p = Pagination::first(50);
        assert_eq!(p.offset, 0);
        assert_eq!(p.limit, 50);
    }

    #[test]
    fn test_pagination_new_sets_offset_and_limit() {
        let p = Pagination::new(10, 25);
        assert_eq!(p.offset, 10);
        assert_eq!(p.limit, 25);
    }

    #[test]
    fn test_paginated_response_has_more_when_total_exceeds_page() {
        let r: PaginatedResponse<i32> = PaginatedResponse::new(vec![1, 2, 3], 10, 0, 3);
        assert!(r.has_more);
        let last: PaginatedResponse<i32> = PaginatedResponse::new(vec![10], 10, 9, 3);
        assert!(!last.has_more);
    }
}
