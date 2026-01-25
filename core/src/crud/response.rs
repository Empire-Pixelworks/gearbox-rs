//! Response types for CRUD operations.

use serde::Serialize;

/// A paginated response wrapper for list endpoints.
///
/// This wrapper includes metadata about the pagination state along with
/// the actual data items.
#[derive(Debug, Clone, Serialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct PagedResponse<T> {
    /// The data items for the current page.
    pub data: Vec<T>,
    /// Total count of items matching the query (before pagination).
    pub total: i64,
    /// The limit that was applied (if any).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<i64>,
    /// The offset that was applied (if any).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub offset: Option<i64>,
}

impl<T> PagedResponse<T> {
    /// Create a new paged response.
    pub fn new(data: Vec<T>, total: i64, limit: Option<i64>, offset: Option<i64>) -> Self {
        PagedResponse {
            data,
            total,
            limit,
            offset,
        }
    }

    /// Create a paged response from data without pagination.
    pub fn from_data(data: Vec<T>) -> Self {
        let total = data.len() as i64;
        PagedResponse {
            data,
            total,
            limit: None,
            offset: None,
        }
    }

    /// Map the data items to a different type.
    pub fn map<U, F>(self, f: F) -> PagedResponse<U>
    where
        F: FnMut(T) -> U,
    {
        PagedResponse {
            data: self.data.into_iter().map(f).collect(),
            total: self.total,
            limit: self.limit,
            offset: self.offset,
        }
    }

    /// Check if there are more items after this page.
    pub fn has_more(&self) -> bool {
        match (self.limit, self.offset) {
            (Some(limit), Some(offset)) => offset + limit < self.total,
            (Some(limit), None) => limit < self.total,
            _ => false,
        }
    }

    /// Get the current page number (0-indexed).
    pub fn page(&self) -> Option<i64> {
        match (self.limit, self.offset) {
            (Some(limit), Some(offset)) if limit > 0 => Some(offset / limit),
            (Some(limit), None) if limit > 0 => Some(0),
            _ => None,
        }
    }

    /// Get the total number of pages.
    pub fn total_pages(&self) -> Option<i64> {
        self.limit.map(|limit| {
            if limit > 0 {
                (self.total + limit - 1) / limit
            } else {
                1
            }
        })
    }
}

impl<T> Default for PagedResponse<T> {
    fn default() -> Self {
        PagedResponse {
            data: Vec::new(),
            total: 0,
            limit: None,
            offset: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_paged_response_has_more() {
        let response = PagedResponse::new(vec![1, 2, 3], 10, Some(3), Some(0));
        assert!(response.has_more());

        let response = PagedResponse::new(vec![8, 9, 10], 10, Some(3), Some(7));
        assert!(!response.has_more());
    }

    #[test]
    fn test_paged_response_page() {
        let response = PagedResponse::new(vec![1, 2, 3], 10, Some(3), Some(0));
        assert_eq!(response.page(), Some(0));

        let response = PagedResponse::new(vec![4, 5, 6], 10, Some(3), Some(3));
        assert_eq!(response.page(), Some(1));
    }

    #[test]
    fn test_paged_response_total_pages() {
        let response = PagedResponse::new(vec![1, 2, 3], 10, Some(3), Some(0));
        assert_eq!(response.total_pages(), Some(4));

        let response = PagedResponse::new(vec![1, 2, 3], 9, Some(3), Some(0));
        assert_eq!(response.total_pages(), Some(3));
    }

    #[test]
    fn test_paged_response_map() {
        let response = PagedResponse::new(vec![1, 2, 3], 10, Some(3), Some(0));
        let mapped = response.map(|x| x * 2);
        assert_eq!(mapped.data, vec![2, 4, 6]);
        assert_eq!(mapped.total, 10);
    }
}
