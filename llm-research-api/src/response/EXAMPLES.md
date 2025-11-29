# Response Module Examples

This document provides comprehensive examples of using the response optimization module.

## Table of Contents

1. [Compression Examples](#compression-examples)
2. [Pagination Examples](#pagination-examples)
3. [Query Optimization Examples](#query-optimization-examples)
4. [Database Indexing Examples](#database-indexing-examples)
5. [Real-World Integration](#real-world-integration)

## Compression Examples

### Basic Compression Setup

```rust
use llm_research_api::response::{CompressionConfig, create_compression_layer};
use axum::Router;

// Create compression configuration
let compression_config = CompressionConfig::builder()
    .compression_level(6)           // Balanced compression
    .min_size_threshold(1024)       // Only compress responses > 1KB
    .enable_gzip(true)
    .enable_deflate(true)
    .enable_brotli(true)
    .build();

// Create the compression layer
let compression_layer = create_compression_layer(compression_config);

// Add to router
let app = Router::new()
    .route("/api/experiments", get(list_experiments))
    .layer(compression_layer);
```

### Custom Content Type Exclusions

```rust
use llm_research_api::response::CompressionConfig;
use std::collections::HashSet;

let mut excluded_types = HashSet::new();
excluded_types.insert("application/wasm".to_string());
excluded_types.insert("application/protobuf".to_string());

let config = CompressionConfig::builder()
    .excluded_content_types(excluded_types)
    .build();
```

### High-Performance API Configuration

```rust
// Maximum compression for slow networks
let config = CompressionConfig::builder()
    .compression_level(9)           // Best compression
    .min_size_threshold(512)        // Compress smaller responses
    .enable_brotli(true)            // Best compression ratio
    .build();
```

## Pagination Examples

### Offset-Based Pagination

```rust
use axum::extract::Query;
use llm_research_api::response::{PaginationParams, PaginatedResponse};

#[derive(Deserialize)]
struct Experiment {
    id: Uuid,
    name: String,
    status: String,
}

async fn list_experiments(
    pagination: PaginationParams,
    State(state): State<AppState>,
) -> Result<PaginatedResponse<Experiment>, ApiError> {
    // Get total count
    let total_count = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM experiments"
    )
    .fetch_one(&state.db_pool)
    .await?
    .unwrap_or(0) as usize;

    // Query with pagination
    let experiments = sqlx::query_as!(
        Experiment,
        "SELECT id, name, status FROM experiments ORDER BY created_at DESC LIMIT $1 OFFSET $2",
        pagination.limit() as i64,
        pagination.offset() as i64
    )
    .fetch_all(&state.db_pool)
    .await?;

    // Return paginated response with navigation links
    Ok(PaginatedResponse::new(
        experiments,
        &pagination,
        total_count,
        Some("/api/experiments"),
    ))
}
```

### Cursor-Based Pagination

```rust
use llm_research_api::response::{CursorPaginatedResponse, PaginationParams};
use base64::{Engine as _, engine::general_purpose};

async fn list_experiments_cursor(
    pagination: PaginationParams,
    State(state): State<AppState>,
) -> Result<CursorPaginatedResponse<Experiment>, ApiError> {
    // Decode cursor (if present)
    let after_id = if let Some(cursor) = &pagination.cursor {
        let decoded = general_purpose::STANDARD.decode(cursor)?;
        let id_str = String::from_utf8(decoded)?;
        Some(Uuid::parse_str(&id_str)?)
    } else {
        None
    };

    // Query with cursor
    let mut experiments = if let Some(id) = after_id {
        sqlx::query_as!(
            Experiment,
            "SELECT id, name, status FROM experiments
             WHERE id > $1
             ORDER BY id ASC
             LIMIT $2",
            id,
            (pagination.limit() + 1) as i64
        )
        .fetch_all(&state.db_pool)
        .await?
    } else {
        sqlx::query_as!(
            Experiment,
            "SELECT id, name, status FROM experiments
             ORDER BY id ASC
             LIMIT $1",
            (pagination.limit() + 1) as i64
        )
        .fetch_all(&state.db_pool)
        .await?
    };

    // Check if there are more results
    let has_more = experiments.len() > pagination.limit();
    if has_more {
        experiments.pop();
    }

    // Generate next cursor
    let next_cursor = if has_more {
        experiments.last().map(|exp| {
            general_purpose::STANDARD.encode(exp.id.to_string().as_bytes())
        })
    } else {
        None
    };

    Ok(CursorPaginatedResponse::new(
        experiments,
        next_cursor,
        pagination.cursor.clone(),
    ))
}
```

### Field Selection

```rust
use llm_research_api::response::FieldSelection;

async fn get_experiment(
    Path(id): Path<Uuid>,
    selection: FieldSelection,
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, ApiError> {
    // Build dynamic query based on requested fields
    let fields = if selection.is_all() {
        "id, name, description, status, created_at, updated_at".to_string()
    } else {
        selection.to_vec().join(", ")
    };

    let query = format!(
        "SELECT {} FROM experiments WHERE id = $1",
        fields
    );

    let row = sqlx::query(&query)
        .bind(id)
        .fetch_one(&state.db_pool)
        .await?;

    // Convert row to JSON...
    Ok(Json(row_to_json(row)))
}
```

## Query Optimization Examples

### Basic Query Building

```rust
use llm_research_api::response::{
    QueryBuilder, FilterSpec, SortSpec, SortDirection
};

// Simple filtered query
let query = QueryBuilder::new("experiments")
    .select(vec!["id".to_string(), "name".to_string(), "status".to_string()])
    .filter(FilterSpec::eq("status", "active"))
    .sort(SortSpec::new("created_at", SortDirection::Desc))
    .limit(20)
    .build();

// Outputs:
// SELECT id, name, status FROM experiments
// WHERE status = 'active'
// ORDER BY created_at DESC
// LIMIT 20
```

### Complex Query with Joins

```rust
use llm_research_api::response::JoinClause;

let query = QueryBuilder::new("experiments")
    .select(vec![
        "experiments.id".to_string(),
        "experiments.name".to_string(),
        "users.email".to_string(),
        "models.name as model_name".to_string(),
    ])
    .join(JoinClause::inner("users", "experiments.user_id = users.id"))
    .join(JoinClause::left("models", "experiments.model_id = models.id"))
    .filter(FilterSpec::eq("experiments.status", "running"))
    .filter(FilterSpec::gt("experiments.created_at", "2024-01-01"))
    .sort(SortSpec::desc("experiments.created_at"))
    .limit(50)
    .build();
```

### Dynamic Filtering from Query Parameters

```rust
#[derive(Deserialize)]
struct ExperimentFilters {
    status: Option<String>,
    user_id: Option<Uuid>,
    created_after: Option<String>,
    model_provider: Option<String>,
}

async fn list_experiments_filtered(
    Query(filters): Query<ExperimentFilters>,
    pagination: PaginationParams,
) -> Result<PaginatedResponse<Experiment>, ApiError> {
    let mut query_builder = QueryBuilder::new("experiments")
        .select(vec!["id".to_string(), "name".to_string(), "status".to_string()]);

    // Add filters dynamically
    if let Some(status) = filters.status {
        query_builder = query_builder.filter(FilterSpec::eq("status", status));
    }

    if let Some(user_id) = filters.user_id {
        query_builder = query_builder.filter(FilterSpec::eq("user_id", user_id.to_string()));
    }

    if let Some(created_after) = filters.created_after {
        query_builder = query_builder.filter(FilterSpec::gt("created_at", created_after));
    }

    let query = query_builder
        .sort(SortSpec::desc("created_at"))
        .limit(pagination.limit())
        .offset(pagination.offset())
        .build();

    // Execute query...
}
```

### Slow Query Logging

```rust
use llm_research_api::response::SlowQueryLogger;
use std::time::Duration;

async fn execute_complex_query(
    state: &AppState,
) -> Result<Vec<Experiment>, ApiError> {
    let logger = SlowQueryLogger::new(Duration::from_millis(500));

    let query = "SELECT e.*, u.email, COUNT(r.id) as run_count
                 FROM experiments e
                 JOIN users u ON e.user_id = u.id
                 LEFT JOIN experiment_runs r ON e.id = r.experiment_id
                 WHERE e.status = 'active'
                 GROUP BY e.id, u.email
                 ORDER BY e.created_at DESC";

    // Automatically logs if query takes > 500ms
    let result = logger.track(query, async {
        sqlx::query_as::<_, Experiment>(query)
            .fetch_all(&state.db_pool)
            .await
    }).await?;

    Ok(result)
}
```

### Query Analysis

```rust
use llm_research_api::response::QueryOptimizer;

let optimizer = QueryOptimizer::new(Duration::from_secs(1));

let query = "SELECT * FROM users WHERE email LIKE '%@example.com'";
let hints = optimizer.analyze(query);

for hint in hints {
    println!("Optimization hint: {}", hint);
}

// Output:
// Optimization hint: Avoid SELECT * and specify only needed columns
// Optimization hint: Consider adding a LIMIT clause to constrain results
```

## Database Indexing Examples

### Creating Indexes

```rust
use llm_research_api::response::IndexDefinition;

// Simple B-Tree index
let status_index = IndexDefinition::btree(
    "experiments",
    vec!["status".to_string()],
);

// Composite index for common query pattern
let user_date_index = IndexDefinition::btree(
    "experiments",
    vec!["user_id".to_string(), "created_at".to_string()],
);

// Partial index for active records
let active_index = IndexDefinition::btree(
    "experiments",
    vec!["status".to_string()],
)
.where_clause("status IN ('running', 'pending')")
.with_comment("Index for active experiments only");

// Full-text search index
let description_index = IndexDefinition::gin(
    "experiments",
    vec!["description".to_string()],
)
.with_comment("Full-text search on experiment descriptions");

// Unique constraint via index
let email_index = IndexDefinition::btree(
    "users",
    vec!["email".to_string()],
)
.unique()
.with_comment("Ensure email uniqueness");
```

### Generating Migrations

```rust
use llm_research_api::response::{CommonIndexPatterns, generate_migration};
use std::fs;

// Generate migration for all common indexes
let all_indexes = CommonIndexPatterns::all_indexes();
let migration = generate_migration(&all_indexes);

// Write to migration file
fs::write(
    "migrations/001_add_performance_indexes.sql",
    migration
)?;
```

### Custom Index Patterns

```rust
use llm_research_api::response::IndexDefinition;

fn api_key_indexes() -> Vec<IndexDefinition> {
    vec![
        // Fast lookup by key hash
        IndexDefinition::btree(
            "api_keys",
            vec!["key_hash".to_string()],
        )
        .unique()
        .with_comment("Primary lookup by API key hash"),

        // Filter by user and status
        IndexDefinition::btree(
            "api_keys",
            vec!["user_id".to_string(), "is_active".to_string()],
        )
        .where_clause("is_active = true")
        .with_comment("Active keys by user"),

        // Expiration cleanup
        IndexDefinition::btree(
            "api_keys",
            vec!["expires_at".to_string()],
        )
        .where_clause("expires_at IS NOT NULL")
        .with_comment("Index for expiration cleanup"),
    ]
}
```

### Index Analysis

```rust
use llm_research_api::response::{IndexAnalyzer, TableSchema};

let mut analyzer = IndexAnalyzer::new();

// Add table schema
let schema = TableSchema::new("experiments")
    .add_columns(vec![
        "id".to_string(),
        "user_id".to_string(),
        "status".to_string(),
        "created_at".to_string(),
    ]);

analyzer.add_table(schema);

// Analyze query
let query = "SELECT * FROM experiments WHERE user_id = $1 ORDER BY created_at DESC";
let recommendations = analyzer.analyze_query(query);

for rec in recommendations {
    println!("Priority {}: {}", rec.priority, rec.reason);
    println!("  Index: {}", rec.index.to_sql());
}
```

## Real-World Integration

### Complete API Endpoint

```rust
use axum::{Router, routing::get};
use llm_research_api::response::*;
use std::time::Duration;

#[derive(Serialize, Deserialize)]
struct Experiment {
    id: Uuid,
    name: String,
    description: String,
    status: String,
    created_at: DateTime<Utc>,
}

async fn list_experiments(
    pagination: PaginationParams,
    selection: FieldSelection,
    Query(filters): Query<HashMap<String, String>>,
    State(state): State<AppState>,
) -> Result<PaginatedResponse<Experiment>, ApiError> {
    // 1. Build optimized query
    let mut query_builder = QueryBuilder::new("experiments");

    // Apply field selection
    if !selection.is_all() {
        query_builder = query_builder.select(selection.to_vec());
    }

    // Apply dynamic filters
    for (field, value) in filters {
        query_builder = query_builder.filter(FilterSpec::eq(&field, value));
    }

    // Add pagination
    let query = query_builder
        .sort(SortSpec::desc("created_at"))
        .limit(pagination.limit())
        .offset(pagination.offset())
        .build();

    // 2. Track query performance
    let logger = SlowQueryLogger::new(Duration::from_millis(500));

    let experiments = logger.track(&query, async {
        sqlx::query_as::<_, Experiment>(&query)
            .fetch_all(&state.db_pool)
            .await
    }).await?;

    // 3. Get total count
    let total_count = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM experiments"
    )
    .fetch_one(&state.db_pool)
    .await?
    .unwrap_or(0) as usize;

    // 4. Return paginated response (will be compressed by middleware)
    Ok(PaginatedResponse::new(
        experiments,
        &pagination,
        total_count,
        Some("/api/experiments"),
    ))
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Setup compression
    let compression_config = CompressionConfig::builder()
        .compression_level(6)
        .min_size_threshold(1024)
        .build();

    // Build router
    let app = Router::new()
        .route("/api/experiments", get(list_experiments))
        .layer(create_compression_layer(compression_config))
        .with_state(app_state);

    // Generate database indexes
    let indexes = CommonIndexPatterns::experiments_indexes();
    let migration = generate_migration(&indexes);
    println!("Run this migration:\n{}", migration);

    // Start server
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await?;
    axum::serve(listener, app).await?;

    Ok(())
}
```

### Performance Optimized Setup

```rust
use llm_research_api::response::*;

// Maximum performance configuration
pub fn create_optimized_router(state: AppState) -> Router {
    // High compression for bandwidth savings
    let compression = CompressionConfig::builder()
        .compression_level(9)
        .min_size_threshold(512)
        .enable_brotli(true)
        .build();

    Router::new()
        .route("/api/experiments", get(list_experiments))
        .route("/api/models", get(list_models))
        .route("/api/datasets", get(list_datasets))
        .layer(create_compression_layer(compression))
        .with_state(state)
}

// Setup database indexes
pub async fn setup_database_indexes(pool: &PgPool) -> Result<(), sqlx::Error> {
    let indexes = CommonIndexPatterns::all_indexes();

    for index in indexes {
        let sql = index.to_sql();
        sqlx::query(&sql).execute(pool).await?;
        println!("Created index: {}", index.name);
    }

    Ok(())
}

// Configure slow query logging
pub fn slow_query_config() -> SlowQueryConfig {
    SlowQueryConfig {
        threshold_ms: 500,
        enabled: true,
    }
}
```

## Sample Migration Output

Here's what a generated migration looks like:

```sql
-- Migration: Create indexes for performance optimization
-- Generated at: 2024-01-15T10:30:00.000Z

-- ==== UP Migration ====

-- Primary lookup by experiment ID
CREATE UNIQUE INDEX idx_experiments_id ON experiments USING btree (id);

-- Filter by experiment status
CREATE INDEX idx_experiments_status ON experiments USING btree (status) WHERE status != 'completed';
COMMENT ON INDEX idx_experiments_status IS 'Index for filtering active experiments';

-- Date-based queries
CREATE INDEX idx_experiments_created_at ON experiments USING btree (created_at);

-- User's experiments ordered by creation date
CREATE INDEX idx_experiments_user_id_created_at ON experiments USING btree (user_id, created_at);
COMMENT ON INDEX idx_experiments_user_id_created_at IS 'User''s experiments ordered by creation date';

-- Full-text search on model descriptions
CREATE INDEX idx_models_description_gin ON models USING gin (description);
COMMENT ON INDEX idx_models_description_gin IS 'Full-text search on model descriptions';

-- ==== DOWN Migration ====

DROP INDEX IF EXISTS idx_experiments_id;
DROP INDEX IF EXISTS idx_experiments_status;
DROP INDEX IF EXISTS idx_experiments_created_at;
DROP INDEX IF EXISTS idx_experiments_user_id_created_at;
DROP INDEX IF EXISTS idx_models_description_gin;
```
