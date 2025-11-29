//! Pagination utilities for API responses.
//!
//! This module provides comprehensive pagination support including both
//! offset-based and cursor-based pagination strategies. It includes
//! utilities for extracting pagination parameters from requests and
//! building paginated responses with metadata and navigation links.

use axum::{
    async_trait,
    extract::{FromRequestParts, Query},
    http::{request::Parts, StatusCode},
    response::{IntoResponse, Response},
};
use serde::{Deserialize, Serialize};
use std::fmt;

/// Default page size when not specified.
pub const DEFAULT_PAGE_SIZE: usize = 20;

/// Maximum allowed page size to prevent resource exhaustion.
pub const MAX_PAGE_SIZE: usize = 100;

/// Minimum page size.
pub const MIN_PAGE_SIZE: usize = 1;

/// Pagination parameters extracted from query string.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PaginationParams {
    /// Page number (1-indexed). Used for offset-based pagination.
    #[serde(default = "default_page")]
    pub page: usize,

    /// Number of items per page.
    #[serde(default = "default_page_size")]
    pub page_size: usize,

    /// Optional offset for manual offset-based pagination.
    pub offset: Option<usize>,

    /// Optional cursor for cursor-based pagination.
    pub cursor: Option<String>,
}

fn default_page() -> usize {
    1
}

fn default_page_size() -> usize {
    DEFAULT_PAGE_SIZE
}

impl Default for PaginationParams {
    fn default() -> Self {
        Self {
            page: 1,
            page_size: DEFAULT_PAGE_SIZE,
            offset: None,
            cursor: None,
        }
    }
}

impl PaginationParams {
    /// Creates a new pagination params with default values.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the page number.
    pub fn with_page(mut self, page: usize) -> Self {
        self.page = page.max(1);
        self
    }

    /// Sets the page size.
    pub fn with_page_size(mut self, page_size: usize) -> Self {
        self.page_size = page_size.clamp(MIN_PAGE_SIZE, MAX_PAGE_SIZE);
        self
    }

    /// Sets the cursor.
    pub fn with_cursor(mut self, cursor: impl Into<String>) -> Self {
        self.cursor = Some(cursor.into());
        self
    }

    /// Validates and normalizes pagination parameters.
    pub fn validate(&mut self) -> Result<(), PaginationError> {
        // Ensure page is at least 1
        if self.page == 0 {
            self.page = 1;
        }

        // Clamp page size to valid range
        if self.page_size < MIN_PAGE_SIZE {
            self.page_size = MIN_PAGE_SIZE;
        } else if self.page_size > MAX_PAGE_SIZE {
            return Err(PaginationError::PageSizeTooLarge {
                requested: self.page_size,
                max: MAX_PAGE_SIZE,
            });
        }

        Ok(())
    }

    /// Calculates the offset for database queries.
    pub fn offset(&self) -> usize {
        if let Some(offset) = self.offset {
            offset
        } else {
            (self.page.saturating_sub(1)).saturating_mul(self.page_size)
        }
    }

    /// Returns the limit for database queries.
    pub fn limit(&self) -> usize {
        self.page_size
    }

    /// Checks if this is cursor-based pagination.
    pub fn is_cursor_based(&self) -> bool {
        self.cursor.is_some()
    }
}

#[async_trait]
impl<S> FromRequestParts<S> for PaginationParams
where
    S: Send + Sync,
{
    type Rejection = PaginationError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let Query(mut params) = Query::<PaginationParams>::from_request_parts(parts, state)
            .await
            .map_err(|_| PaginationError::InvalidParams)?;

        params.validate()?;
        Ok(params)
    }
}

/// Information about the current page in a paginated response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageInfo {
    /// Current page number (1-indexed).
    pub current_page: usize,

    /// Total number of pages.
    pub total_pages: usize,

    /// Number of items per page.
    pub per_page: usize,

    /// Total number of items across all pages.
    pub total_count: usize,

    /// Whether there is a previous page.
    pub has_previous: bool,

    /// Whether there is a next page.
    pub has_next: bool,
}

impl PageInfo {
    /// Creates page info from pagination parameters and total count.
    pub fn new(params: &PaginationParams, total_count: usize) -> Self {
        let total_pages = if params.page_size == 0 {
            0
        } else {
            (total_count + params.page_size - 1) / params.page_size
        };

        let current_page = params.page;
        let has_previous = current_page > 1;
        let has_next = current_page < total_pages;

        Self {
            current_page,
            total_pages,
            per_page: params.page_size,
            total_count,
            has_previous,
            has_next,
        }
    }

    /// Returns the start index (0-based) of items on the current page.
    pub fn start_index(&self) -> usize {
        (self.current_page.saturating_sub(1)).saturating_mul(self.per_page)
    }

    /// Returns the end index (exclusive, 0-based) of items on the current page.
    pub fn end_index(&self) -> usize {
        (self.start_index() + self.per_page).min(self.total_count)
    }

    /// Returns the number of items on the current page.
    pub fn items_on_page(&self) -> usize {
        if self.current_page > self.total_pages {
            0
        } else if self.current_page == self.total_pages {
            let remainder = self.total_count % self.per_page;
            if remainder == 0 {
                self.per_page
            } else {
                remainder
            }
        } else {
            self.per_page
        }
    }
}

/// Navigation links for paginated responses.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaginationLinks {
    /// URL to the first page.
    pub first: String,

    /// URL to the previous page (if available).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prev: Option<String>,

    /// URL to the next page (if available).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next: Option<String>,

    /// URL to the last page.
    pub last: String,

    /// URL to the current page.
    pub self_link: String,
}

impl PaginationLinks {
    /// Creates pagination links from a base URL and page info.
    pub fn new(base_url: &str, page_info: &PageInfo, params: &PaginationParams) -> Self {
        let base_url = base_url.trim_end_matches('?');
        let query_separator = if base_url.contains('?') { '&' } else { '?' };

        let first = format!(
            "{}{}page=1&page_size={}",
            base_url, query_separator, params.page_size
        );

        let last = format!(
            "{}{}page={}&page_size={}",
            base_url, query_separator, page_info.total_pages, params.page_size
        );

        let self_link = format!(
            "{}{}page={}&page_size={}",
            base_url, query_separator, page_info.current_page, params.page_size
        );

        let prev = if page_info.has_previous {
            Some(format!(
                "{}{}page={}&page_size={}",
                base_url,
                query_separator,
                page_info.current_page - 1,
                params.page_size
            ))
        } else {
            None
        };

        let next = if page_info.has_next {
            Some(format!(
                "{}{}page={}&page_size={}",
                base_url,
                query_separator,
                page_info.current_page + 1,
                params.page_size
            ))
        } else {
            None
        };

        Self {
            first,
            prev,
            next,
            last,
            self_link,
        }
    }
}

/// A paginated response wrapper.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaginatedResponse<T> {
    /// The data items for the current page.
    pub data: Vec<T>,

    /// Pagination metadata.
    pub page_info: PageInfo,

    /// Navigation links.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub links: Option<PaginationLinks>,
}

impl<T> PaginatedResponse<T> {
    /// Creates a new paginated response.
    pub fn new(
        data: Vec<T>,
        params: &PaginationParams,
        total_count: usize,
        base_url: Option<&str>,
    ) -> Self {
        let page_info = PageInfo::new(params, total_count);
        let links = base_url.map(|url| PaginationLinks::new(url, &page_info, params));

        Self {
            data,
            page_info,
            links,
        }
    }

    /// Creates a paginated response without links.
    pub fn without_links(data: Vec<T>, params: &PaginationParams, total_count: usize) -> Self {
        Self::new(data, params, total_count, None)
    }

    /// Maps the data items to a different type.
    pub fn map<U, F>(self, f: F) -> PaginatedResponse<U>
    where
        F: FnMut(T) -> U,
    {
        PaginatedResponse {
            data: self.data.into_iter().map(f).collect(),
            page_info: self.page_info,
            links: self.links,
        }
    }

    /// Returns the number of items in the current page.
    pub fn len(&self) -> usize {
        self.data.len()
    }

    /// Checks if the current page is empty.
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }
}

impl<T: Serialize> IntoResponse for PaginatedResponse<T> {
    fn into_response(self) -> Response {
        axum::Json(self).into_response()
    }
}

/// Cursor-based pagination for efficient iteration over large datasets.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CursorPagination {
    /// The cursor for the next page.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_cursor: Option<String>,

    /// The cursor for the previous page.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prev_cursor: Option<String>,

    /// Whether there are more items after this page.
    pub has_more: bool,
}

impl CursorPagination {
    /// Creates cursor pagination metadata.
    pub fn new(next_cursor: Option<String>, prev_cursor: Option<String>, has_more: bool) -> Self {
        Self {
            next_cursor,
            prev_cursor,
            has_more,
        }
    }
}

/// A cursor-based paginated response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CursorPaginatedResponse<T> {
    /// The data items for the current page.
    pub data: Vec<T>,

    /// Cursor pagination metadata.
    pub pagination: CursorPagination,
}

impl<T> CursorPaginatedResponse<T> {
    /// Creates a new cursor-based paginated response.
    pub fn new(data: Vec<T>, next_cursor: Option<String>, prev_cursor: Option<String>) -> Self {
        let has_more = next_cursor.is_some();

        Self {
            data,
            pagination: CursorPagination::new(next_cursor, prev_cursor, has_more),
        }
    }

    /// Maps the data items to a different type.
    pub fn map<U, F>(self, f: F) -> CursorPaginatedResponse<U>
    where
        F: FnMut(T) -> U,
    {
        CursorPaginatedResponse {
            data: self.data.into_iter().map(f).collect(),
            pagination: self.pagination,
        }
    }
}

impl<T: Serialize> IntoResponse for CursorPaginatedResponse<T> {
    fn into_response(self) -> Response {
        axum::Json(self).into_response()
    }
}

/// Trait for types that can be paginated.
#[async_trait]
pub trait Paginator<T> {
    /// Returns a page of items based on pagination parameters.
    async fn paginate(&self, params: &PaginationParams) -> Result<(Vec<T>, usize), PaginationError>;
}

/// Pagination-related errors.
#[derive(Debug, thiserror::Error)]
pub enum PaginationError {
    /// Invalid pagination parameters.
    #[error("Invalid pagination parameters")]
    InvalidParams,

    /// Page size exceeds maximum allowed.
    #[error("Page size {requested} exceeds maximum of {max}")]
    PageSizeTooLarge { requested: usize, max: usize },

    /// Invalid cursor.
    #[error("Invalid pagination cursor: {0}")]
    InvalidCursor(String),

    /// Database error during pagination.
    #[error("Database error: {0}")]
    DatabaseError(String),
}

impl IntoResponse for PaginationError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            PaginationError::InvalidParams => (StatusCode::BAD_REQUEST, self.to_string()),
            PaginationError::PageSizeTooLarge { .. } => (StatusCode::BAD_REQUEST, self.to_string()),
            PaginationError::InvalidCursor(_) => (StatusCode::BAD_REQUEST, self.to_string()),
            PaginationError::DatabaseError(_) => {
                (StatusCode::INTERNAL_SERVER_ERROR, "Internal server error".to_string())
            }
        };

        (status, message).into_response()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pagination_params_default() {
        let params = PaginationParams::default();
        assert_eq!(params.page, 1);
        assert_eq!(params.page_size, DEFAULT_PAGE_SIZE);
        assert!(params.offset.is_none());
        assert!(params.cursor.is_none());
    }

    #[test]
    fn test_pagination_params_builder() {
        let params = PaginationParams::new()
            .with_page(5)
            .with_page_size(50)
            .with_cursor("abc123");

        assert_eq!(params.page, 5);
        assert_eq!(params.page_size, 50);
        assert_eq!(params.cursor.as_deref(), Some("abc123"));
    }

    #[test]
    fn test_pagination_params_validate_clamps_page_size() {
        let mut params = PaginationParams::new().with_page_size(0);
        params.validate().unwrap();
        assert_eq!(params.page_size, MIN_PAGE_SIZE);
    }

    #[test]
    fn test_pagination_params_validate_rejects_too_large() {
        // Note: with_page_size() clamps to MAX_PAGE_SIZE, so we set directly
        let mut params = PaginationParams::new();
        params.page_size = 200; // Bypass clamping by setting directly
        let result = params.validate();
        assert!(matches!(result, Err(PaginationError::PageSizeTooLarge { .. })));
    }

    #[test]
    fn test_pagination_params_offset_calculation() {
        let params = PaginationParams::new().with_page(1).with_page_size(20);
        assert_eq!(params.offset(), 0);

        let params = PaginationParams::new().with_page(2).with_page_size(20);
        assert_eq!(params.offset(), 20);

        let params = PaginationParams::new().with_page(5).with_page_size(25);
        assert_eq!(params.offset(), 100);
    }

    #[test]
    fn test_pagination_params_manual_offset() {
        let mut params = PaginationParams::new();
        params.offset = Some(50);
        assert_eq!(params.offset(), 50);
    }

    #[test]
    fn test_page_info_creation() {
        let params = PaginationParams::new().with_page(2).with_page_size(20);
        let page_info = PageInfo::new(&params, 100);

        assert_eq!(page_info.current_page, 2);
        assert_eq!(page_info.total_pages, 5);
        assert_eq!(page_info.per_page, 20);
        assert_eq!(page_info.total_count, 100);
        assert!(page_info.has_previous);
        assert!(page_info.has_next);
    }

    #[test]
    fn test_page_info_first_page() {
        let params = PaginationParams::new().with_page(1).with_page_size(20);
        let page_info = PageInfo::new(&params, 100);

        assert!(!page_info.has_previous);
        assert!(page_info.has_next);
    }

    #[test]
    fn test_page_info_last_page() {
        let params = PaginationParams::new().with_page(5).with_page_size(20);
        let page_info = PageInfo::new(&params, 100);

        assert!(page_info.has_previous);
        assert!(!page_info.has_next);
    }

    #[test]
    fn test_page_info_items_on_page_full_page() {
        let params = PaginationParams::new().with_page(1).with_page_size(20);
        let page_info = PageInfo::new(&params, 100);

        assert_eq!(page_info.items_on_page(), 20);
    }

    #[test]
    fn test_page_info_items_on_page_partial_last_page() {
        let params = PaginationParams::new().with_page(3).with_page_size(20);
        let page_info = PageInfo::new(&params, 45);

        assert_eq!(page_info.items_on_page(), 5);
    }

    #[test]
    fn test_page_info_items_on_page_exact_multiple() {
        let params = PaginationParams::new().with_page(5).with_page_size(20);
        let page_info = PageInfo::new(&params, 100);

        assert_eq!(page_info.items_on_page(), 20);
    }

    #[test]
    fn test_pagination_links_creation() {
        let params = PaginationParams::new().with_page(2).with_page_size(20);
        let page_info = PageInfo::new(&params, 100);
        let links = PaginationLinks::new("/api/items", &page_info, &params);

        assert_eq!(links.first, "/api/items?page=1&page_size=20");
        assert_eq!(links.prev, Some("/api/items?page=1&page_size=20".to_string()));
        assert_eq!(links.next, Some("/api/items?page=3&page_size=20".to_string()));
        assert_eq!(links.last, "/api/items?page=5&page_size=20");
        assert_eq!(links.self_link, "/api/items?page=2&page_size=20");
    }

    #[test]
    fn test_pagination_links_no_prev_on_first_page() {
        let params = PaginationParams::new().with_page(1).with_page_size(20);
        let page_info = PageInfo::new(&params, 100);
        let links = PaginationLinks::new("/api/items", &page_info, &params);

        assert!(links.prev.is_none());
        assert!(links.next.is_some());
    }

    #[test]
    fn test_pagination_links_no_next_on_last_page() {
        let params = PaginationParams::new().with_page(5).with_page_size(20);
        let page_info = PageInfo::new(&params, 100);
        let links = PaginationLinks::new("/api/items", &page_info, &params);

        assert!(links.prev.is_some());
        assert!(links.next.is_none());
    }

    #[test]
    fn test_paginated_response_creation() {
        let data = vec![1, 2, 3, 4, 5];
        let params = PaginationParams::new().with_page(1).with_page_size(5);
        let response = PaginatedResponse::new(data, &params, 25, Some("/api/items"));

        assert_eq!(response.data.len(), 5);
        assert_eq!(response.page_info.total_count, 25);
        assert!(response.links.is_some());
    }

    #[test]
    fn test_paginated_response_without_links() {
        let data = vec![1, 2, 3];
        let params = PaginationParams::new().with_page(1).with_page_size(3);
        let response = PaginatedResponse::without_links(data, &params, 10);

        assert_eq!(response.data.len(), 3);
        assert!(response.links.is_none());
    }

    #[test]
    fn test_paginated_response_map() {
        let data = vec![1, 2, 3];
        let params = PaginationParams::new().with_page(1).with_page_size(3);
        let response = PaginatedResponse::without_links(data, &params, 10);

        let mapped = response.map(|x| x * 2);
        assert_eq!(mapped.data, vec![2, 4, 6]);
    }

    #[test]
    fn test_cursor_pagination_creation() {
        let cursor = CursorPagination::new(
            Some("next_cursor".to_string()),
            Some("prev_cursor".to_string()),
            true,
        );

        assert_eq!(cursor.next_cursor.as_deref(), Some("next_cursor"));
        assert_eq!(cursor.prev_cursor.as_deref(), Some("prev_cursor"));
        assert!(cursor.has_more);
    }

    #[test]
    fn test_cursor_paginated_response_creation() {
        let data = vec![1, 2, 3];
        let response = CursorPaginatedResponse::new(
            data,
            Some("next".to_string()),
            Some("prev".to_string()),
        );

        assert_eq!(response.data.len(), 3);
        assert!(response.pagination.has_more);
        assert_eq!(response.pagination.next_cursor.as_deref(), Some("next"));
    }
}
