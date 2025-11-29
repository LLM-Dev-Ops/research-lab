# Response Optimization Module

This module provides comprehensive utilities for optimizing API responses in the LLM Research API. It includes compression, pagination, query optimization, and database indexing strategies.

## Table of Contents

- [Compression](#compression)
- [Pagination](#pagination)
- [Query Optimization](#query-optimization)
- [Database Indexing](#database-indexing)

## Compression

The compression module provides automatic HTTP response compression using gzip, deflate, and brotli algorithms.

### Features

- Multiple compression algorithms (gzip, deflate, brotli)
- Configurable compression levels (0-11)
- Minimum size threshold to avoid compressing small responses
- Content-type based exclusions (skips already compressed formats)
- Tower layer integration for seamless middleware usage

### Usage

```rust
use llm_research_api::response::{CompressionConfig, create_compression_layer};

// Create configuration
let config = CompressionConfig::builder()
    .compression_level(6)           // Default quality
    .min_size_threshold(1024)       // 1KB minimum
    .enable_gzip(true)
    .enable_deflate(true)
    .enable_brotli(true)
    .exclude_content_type("image/jpeg")
    .build();

// Create Tower layer
let compression_layer = create_compression_layer(config);

// Add to your router
let app = Router::new()
    .route("/api/data", get(handler))
    .layer(compression_layer);
```

### Configuration Options

- `compression_level`: 0-11 (0=fastest, 11=best compression)
- `min_size_threshold`: Minimum response size in bytes to compress
- `excluded_content_types`: Set of MIME types to skip compression
- `enable_gzip`, `enable_deflate`, `enable_brotli`: Enable/disable algorithms

### Default Excluded Content Types

The default configuration excludes:
- Images: JPEG, PNG, GIF, WebP
- Videos: MP4, WebM
- Audio: MP3, OGG
- Compressed archives: ZIP, GZIP, BZIP2, 7Z, RAR
- PDF documents

## Pagination

The pagination module supports both offset-based and cursor-based pagination strategies.

### Offset-Based Pagination

Best for traditional page-based navigation with total counts.

```rust
use llm_research_api::response::{PaginationParams, PaginatedResponse};

// Extract pagination params from request
async fn list_items(
    params: PaginationParams,
) -> Result<PaginatedResponse<Item>, ApiError> {
    // Query database with limit and offset
    let items = query_items(params.limit(), params.offset()).await?;
    let total_count = count_items().await?;

    // Create paginated response with links
    Ok(PaginatedResponse::new(
        items,
        &params,
        total_count,
        Some("/api/items"),
    ))
}
```

### Cursor-Based Pagination

Best for large datasets and real-time feeds where total counts are expensive.

```rust
use llm_research_api::response::{CursorPaginatedResponse, PaginationParams};

async fn list_items_cursor(
    params: PaginationParams,
) -> Result<CursorPaginatedResponse<Item>, ApiError> {
    let cursor = params.cursor.clone();
    let items = query_items_after_cursor(&cursor, params.limit()).await?;

    let next_cursor = items.last().map(|item| item.id.to_string());

    Ok(CursorPaginatedResponse::new(
        items,
        next_cursor,
        cursor,
    ))
}
```

### Response Format

**Offset-based:**
```json
{
  "data": [...],
  "page_info": {
    "current_page": 2,
    "total_pages": 10,
    "per_page": 20,
    "total_count": 195,
    "has_previous": true,
    "has_next": true
  },
  "links": {
    "first": "/api/items?page=1&page_size=20",
    "prev": "/api/items?page=1&page_size=20",
    "next": "/api/items?page=3&page_size=20",
    "last": "/api/items?page=10&page_size=20",
    "self_link": "/api/items?page=2&page_size=20"
  }
}
```

**Cursor-based:**
```json
{
  "data": [...],
  "pagination": {
    "next_cursor": "eyJpZCI6MTIzfQ==",
    "prev_cursor": "eyJpZCI6MTAwfQ==",
    "has_more": true
  }
}
```

### Constants

- `DEFAULT_PAGE_SIZE`: 20
- `MAX_PAGE_SIZE`: 100
- `MIN_PAGE_SIZE`: 1

## Query Optimization

Tools for building optimized SQL queries with dynamic filtering, sorting, and performance tracking.

### QueryBuilder

```rust
use llm_research_api::response::{
    QueryBuilder, FilterSpec, SortSpec, SortDirection, JoinClause
};

let query = QueryBuilder::new("experiments")
    .select(vec![
        "id".to_string(),
        "name".to_string(),
        "status".to_string(),
    ])
    .filter(FilterSpec::eq("status", "active"))
    .filter(FilterSpec::gt("created_at", "2024-01-01"))
    .sort(SortSpec::new("created_at", SortDirection::Desc))
    .join(JoinClause::inner("users", "experiments.user_id = users.id"))
    .limit(20)
    .offset(0)
    .build();

// Outputs: SELECT id, name, status FROM experiments
//          INNER JOIN users ON experiments.user_id = users.id
//          WHERE status = 'active' AND created_at > '2024-01-01'
//          ORDER BY created_at DESC LIMIT 20 OFFSET 0
```

### Filter Operators

- `Eq`: Equal to
- `Ne`: Not equal to
- `Gt`: Greater than
- `Gte`: Greater than or equal
- `Lt`: Less than
- `Lte`: Less than or equal
- `Like`: SQL LIKE pattern matching
- `In`: IN clause
- `NotIn`: NOT IN clause
- `IsNull`: IS NULL
- `IsNotNull`: IS NOT NULL

### Field Selection

Allow clients to specify which fields to return:

```rust
use llm_research_api::response::FieldSelection;

async fn get_item(
    selection: FieldSelection,
) -> Result<Json<serde_json::Value>, ApiError> {
    // Client can request: /api/items?fields=id,name,email
    // selection.contains("id") == true
    // selection.contains("password") == false

    let fields = selection.to_vec(); // ["id", "name", "email"]
    let query = QueryBuilder::new("users")
        .select(fields)
        .build();

    // Execute query...
}
```

### Slow Query Logger

Track and log slow database queries:

```rust
use llm_research_api::response::SlowQueryLogger;
use std::time::Duration;

let logger = SlowQueryLogger::new(Duration::from_secs(1));

// Track query execution
let result = logger.track(
    "SELECT * FROM experiments WHERE status = 'active'",
    async {
        // Execute your query
        database.query("SELECT * FROM experiments WHERE status = 'active'").await
    }
).await?;

// Automatically logs if query takes > 1 second
```

### Query Optimizer

Analyze queries for potential performance issues:

```rust
use llm_research_api::response::QueryOptimizer;
use std::time::Duration;

let optimizer = QueryOptimizer::new(Duration::from_secs(1));
let hints = optimizer.analyze("SELECT * FROM users");

// hints may contain:
// - OptimizationHint::SelectStar
// - OptimizationHint::MissingWhereClause
// - OptimizationHint::MissingLimit
// - OptimizationHint::OrCondition
```

## Database Indexing

Utilities for defining, analyzing, and generating database indexes.

### Index Definitions

```rust
use llm_research_api::response::{IndexDefinition, IndexStrategy};

// B-Tree index (default, good for range queries)
let index = IndexDefinition::btree("experiments", vec!["status".to_string()])
    .where_clause("status != 'completed'")
    .with_comment("Partial index for active experiments");

// Unique index
let unique_index = IndexDefinition::btree("users", vec!["email".to_string()])
    .unique();

// GIN index for full-text search
let gin_index = IndexDefinition::gin("posts", vec!["content".to_string()])
    .with_comment("Full-text search on post content");

// Composite index
let composite_index = IndexDefinition::btree(
    "experiments",
    vec!["user_id".to_string(), "created_at".to_string()],
);
```

### Common Index Patterns

Pre-defined indexes for common tables:

```rust
use llm_research_api::response::CommonIndexPatterns;

// Get all indexes for experiments table
let indexes = CommonIndexPatterns::experiments_indexes();

// Get all indexes for models table
let indexes = CommonIndexPatterns::models_indexes();

// Get all indexes for datasets table
let indexes = CommonIndexPatterns::datasets_indexes();

// Get all common indexes
let all_indexes = CommonIndexPatterns::all_indexes();
```

### Generate Migration

Create SQL migration files from index definitions:

```rust
use llm_research_api::response::{CommonIndexPatterns, generate_migration};

let indexes = CommonIndexPatterns::experiments_indexes();
let migration_sql = generate_migration(&indexes);

// Write to file
std::fs::write("migrations/add_experiment_indexes.sql", migration_sql)?;
```

Migration format:
```sql
-- Migration: Create indexes for performance optimization
-- Generated at: 2024-01-15T10:30:00Z

-- ==== UP Migration ====

-- Index for filtering active experiments
CREATE INDEX idx_experiments_status ON experiments USING btree (status) WHERE status != 'completed';
COMMENT ON INDEX idx_experiments_status IS 'Index for filtering active experiments';

-- ==== DOWN Migration ====

DROP INDEX IF EXISTS idx_experiments_status;
```

### Index Strategy Types

- `BTree`: Default PostgreSQL index, good for comparisons and range queries
- `Hash`: For equality comparisons only, faster than B-Tree for exact matches
- `GiST`: Generalized Search Tree, for geometric and full-text data
- `GIN`: Generalized Inverted Index, for array and JSONB columns

### Index Analyzer

Analyze query patterns and recommend indexes:

```rust
use llm_research_api::response::{IndexAnalyzer, TableSchema};

let mut analyzer = IndexAnalyzer::new();

// Add table schemas
let schema = TableSchema::new("experiments")
    .add_column("id")
    .add_column("status")
    .add_column("created_at");
analyzer.add_table(schema);

// Analyze a query
let query = "SELECT * FROM experiments WHERE status = 'active'";
let recommendations = analyzer.analyze_query(query);

// recommendations contains IndexRecommendation objects with:
// - index: IndexDefinition
// - reason: String explaining why this index is recommended
// - priority: 1-5 (5 is highest)
// - query_patterns: Vec of query patterns that would benefit
```

## Integration Example

Complete example integrating all modules:

```rust
use axum::Router;
use llm_research_api::response::{
    create_compression_layer, CompressionConfig,
    PaginationParams, PaginatedResponse,
    QueryBuilder, FilterSpec, SortSpec, SortDirection,
    SlowQueryLogger,
};

async fn list_experiments(
    params: PaginationParams,
) -> Result<PaginatedResponse<Experiment>, ApiError> {
    // Build optimized query
    let query = QueryBuilder::new("experiments")
        .select(vec!["id".to_string(), "name".to_string(), "status".to_string()])
        .filter(FilterSpec::eq("status", "active"))
        .sort(SortSpec::new("created_at", SortDirection::Desc))
        .limit(params.limit())
        .offset(params.offset())
        .build();

    // Track query performance
    let logger = SlowQueryLogger::new(Duration::from_secs(1));
    let items = logger.track(&query, async {
        database.query(&query).await
    }).await?;

    let total_count = database.count("experiments").await?;

    // Return paginated response
    Ok(PaginatedResponse::new(
        items,
        &params,
        total_count,
        Some("/api/experiments"),
    ))
}

#[tokio::main]
async fn main() {
    // Setup compression
    let compression_config = CompressionConfig::builder()
        .compression_level(6)
        .build();

    let app = Router::new()
        .route("/api/experiments", get(list_experiments))
        .layer(create_compression_layer(compression_config));

    // Run server...
}
```

## Performance Considerations

### Compression

- **CPU vs Bandwidth**: Higher compression levels use more CPU but reduce bandwidth
- **Size Threshold**: Set appropriately to avoid compressing tiny responses
- **Already Compressed**: Automatically skips images, videos, and archives
- **Compression Level 6**: Good balance for most use cases

### Pagination

- **Offset-based**:
  - Pros: Simple, supports jumping to any page, shows total pages
  - Cons: Performance degrades with large offsets, inconsistent under writes
- **Cursor-based**:
  - Pros: Consistent performance, handles real-time updates well
  - Cons: Can't jump to arbitrary pages, no total count
- **Recommendation**: Use cursor-based for large datasets (>10k rows)

### Indexing

- **Index Overhead**: Each index adds overhead to writes (INSERT, UPDATE, DELETE)
- **Index Selectivity**: High cardinality columns benefit more from indexes
- **Composite Indexes**: Column order matters (most selective first)
- **Partial Indexes**: Use WHERE clauses to index subset of rows
- **Monitor Usage**: Use `pg_stat_user_indexes` to identify unused indexes

## Testing

All modules include comprehensive unit tests:

```bash
# Run all tests
cargo test --package llm-research-api

# Run specific module tests
cargo test --package llm-research-api response::compression
cargo test --package llm-research-api response::pagination
cargo test --package llm-research-api response::query
cargo test --package llm-research-api response::indexing
```

Test coverage:
- `compression.rs`: 13 tests
- `pagination.rs`: 20 tests
- `query.rs`: 18 tests
- `indexing.rs`: 16 tests
- `mod.rs`: 8 integration tests

Total: **75 tests**

## License

This code is part of the LLM Research Lab project and is licensed under the LLM Dev Ops Permanent Source-Available License.
