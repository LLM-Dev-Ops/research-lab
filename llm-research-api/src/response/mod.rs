//! Response optimization module for the LLM Research API.
//!
//! This module provides comprehensive utilities for optimizing API responses including:
//! - HTTP response compression (gzip, deflate, brotli)
//! - Offset-based and cursor-based pagination
//! - Query optimization and slow query logging
//! - Database indexing strategies and recommendations
//!
//! # Examples
//!
//! ## Using Compression
//!
//! ```rust
//! use llm_research_api::response::{CompressionConfig, create_compression_layer};
//!
//! let config = CompressionConfig::builder()
//!     .compression_level(6)
//!     .min_size_threshold(1024)
//!     .enable_gzip(true)
//!     .enable_brotli(true)
//!     .build();
//!
//! let compression_layer = create_compression_layer(config);
//! ```
//!
//! ## Using Pagination
//!
//! ```rust
//! use llm_research_api::response::{PaginationParams, PaginatedResponse};
//!
//! let params = PaginationParams::new()
//!     .with_page(1)
//!     .with_page_size(20);
//!
//! let data = vec![1, 2, 3, 4, 5];
//! let response = PaginatedResponse::new(data, &params, 100, Some("/api/items"));
//! ```
//!
//! ## Using Query Builder
//!
//! ```rust
//! use llm_research_api::response::{QueryBuilder, FilterSpec, SortSpec, SortDirection};
//!
//! let query = QueryBuilder::new("experiments")
//!     .select(vec!["id".to_string(), "name".to_string(), "status".to_string()])
//!     .filter(FilterSpec::eq("status", "active"))
//!     .sort(SortSpec::new("created_at", SortDirection::Desc))
//!     .limit(10)
//!     .build();
//! ```
//!
//! ## Using Index Patterns
//!
//! ```rust
//! use llm_research_api::response::{CommonIndexPatterns, generate_migration};
//!
//! let indexes = CommonIndexPatterns::experiments_indexes();
//! let migration_sql = generate_migration(&indexes);
//! ```

pub mod compression;
pub mod pagination;
pub mod query;
pub mod indexing;

// Re-export commonly used types and functions

// Compression exports
pub use compression::{
    CompressionAlgorithm, CompressionConfig, CompressionConfigBuilder, CompressionLayer,
    CompressionMiddleware, ContentTypePredicate, compression_middleware,
    create_compression_layer, parse_accept_encoding,
};

// Pagination exports
pub use pagination::{
    CursorPagination, CursorPaginatedResponse, PageInfo, PaginatedResponse,
    Paginator, PaginationError, PaginationLinks, PaginationParams, DEFAULT_PAGE_SIZE,
    MAX_PAGE_SIZE, MIN_PAGE_SIZE,
};

// Query optimization exports
pub use query::{
    FieldSelection, FilterOperator, FilterSpec, JoinClause, JoinType,
    OptimizationHint, QueryBuilder, QueryOptimizer, SlowQueryConfig, SlowQueryLogger, SortDirection,
    SortSpec,
};

// Indexing exports
pub use indexing::{
    CommonIndexPatterns, IndexAnalyzer, IndexDefinition, IndexRecommendation, IndexStrategy,
    SizeImpact, TableSchema, generate_migration,
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compression_config_creation() {
        let config = CompressionConfig::default();
        assert!(config.enable_gzip);
        assert!(config.enable_deflate);
    }

    #[test]
    fn test_pagination_params_creation() {
        let params = PaginationParams::new();
        assert_eq!(params.page, 1);
        assert_eq!(params.page_size, DEFAULT_PAGE_SIZE);
    }

    #[test]
    fn test_query_builder_creation() {
        let query = QueryBuilder::new("test_table");
        assert_eq!(query.table_name(), "test_table");
    }

    #[test]
    fn test_index_definition_creation() {
        let index = IndexDefinition::btree("test_table", vec!["id".to_string()]);
        assert_eq!(index.table, "test_table");
        assert_eq!(index.strategy, IndexStrategy::BTree);
    }

    #[test]
    fn test_common_index_patterns_experiments() {
        let indexes = CommonIndexPatterns::experiments_indexes();
        assert!(!indexes.is_empty());
    }

    #[test]
    fn test_common_index_patterns_models() {
        let indexes = CommonIndexPatterns::models_indexes();
        assert!(!indexes.is_empty());
    }

    #[test]
    fn test_common_index_patterns_datasets() {
        let indexes = CommonIndexPatterns::datasets_indexes();
        assert!(!indexes.is_empty());
    }

    #[test]
    fn test_common_index_patterns_all() {
        let indexes = CommonIndexPatterns::all_indexes();
        assert!(indexes.len() > 10);
    }
}
