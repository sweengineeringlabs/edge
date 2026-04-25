//! Pagination types for list operations.

use serde::{Deserialize, Serialize};

/// Pagination parameters for list operations.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Pagination {
    pub offset: usize,
    pub limit: usize,
}

impl Pagination {
    pub fn new(offset: usize, limit: usize) -> Self { Self { offset, limit } }

    pub fn first(limit: usize) -> Self { Self { offset: 0, limit } }
}

/// A paginated response containing items and metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaginatedResponse<T> {
    pub items: Vec<T>,
    pub total: usize,
    pub offset: usize,
    pub limit: usize,
    pub has_more: bool,
}

impl<T> PaginatedResponse<T> {
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
