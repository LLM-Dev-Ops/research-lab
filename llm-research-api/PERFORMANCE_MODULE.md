# Performance Module Documentation

## Overview

The performance module provides comprehensive performance optimization capabilities for the LLM Research API, including connection pool management and query result caching.

## Module Structure

```
src/performance/
├── mod.rs          (132 lines)  - Module exports and documentation
├── pool.rs         (757 lines)  - Connection pool configuration
└── cache.rs        (864 lines)  - Query result caching
```

**Total:** 1,753 lines of code including comprehensive tests and documentation.

## Features

### 1. Connection Pool Management (`pool.rs`)

#### Key Components

- **PoolConfig**: Configurable connection pool settings
  - Min/max connections
  - Acquire timeout
  - Idle timeout
  - Max connection lifetime
  - Health check settings

- **PoolConfigBuilder**: Fluent API for building pool configurations
  ```rust
  let config = PoolConfig::builder()
      .min_connections(10)
      .max_connections(50)
      .acquire_timeout(Duration::from_secs(10))
      .build()?;
  ```

- **Predefined Configurations**:
  - `PoolConfig::high_throughput()` - For high-traffic scenarios
  - `PoolConfig::low_latency()` - For latency-sensitive operations
  - `PoolConfig::development()` - For development environments

- **Pool Functions**:
  - `create_postgres_pool()` - Creates optimized PostgreSQL connection pool
  - `create_clickhouse_pool()` - Creates ClickHouse client with pool settings
  - `get_postgres_pool_stats()` - Retrieves pool statistics

- **PoolMonitor**: Health monitoring and statistics tracking
  - Records pool statistics over time
  - Tracks health status (Healthy, Degraded, Unhealthy)
  - Maintains history of pool metrics

- **PoolStatistics**: Comprehensive pool metrics
  - Active, idle, and waiting connections
  - Utilization percentage
  - Health indicators

#### Example Usage

```rust
use llm_research_api::performance::{PoolConfig, create_postgres_pool};

// Create a pool with custom configuration
let config = PoolConfig::builder()
    .min_connections(10)
    .max_connections(50)
    .build()?;

let pool = create_postgres_pool(
    "postgresql://localhost/llm_research",
    config
).await?;

// Get pool statistics
let stats = get_postgres_pool_stats(&pool);
println!("Pool utilization: {:.1}%", stats.utilization_percent());
```

#### Tests

**20 comprehensive tests** covering:
- Default configurations
- Builder pattern validation
- Predefined configurations (high-throughput, low-latency, development)
- Pool statistics calculations
- Health monitoring
- Statistics history management

### 2. Query Result Caching (`cache.rs`)

#### Key Components

- **CacheConfig**: Cache configuration
  - Default TTL (Time To Live)
  - Maximum entries
  - Eviction policy (LRU, LFU, FIFO, TTL-only)
  - Auto-cleanup settings
  - Statistics tracking

- **CacheConfigBuilder**: Fluent API for cache configuration
  ```rust
  let config = CacheConfig::builder()
      .default_ttl(Duration::from_secs(300))
      .max_entries(10000)
      .eviction_policy(EvictionPolicy::LRU)
      .build()?;
  ```

- **CacheKey**: Strongly-typed cache key namespaces
  - `Experiment(Uuid)` - Individual experiments
  - `ExperimentList { page, limit, filter }` - Paginated lists
  - `Model(Uuid)` - Model data
  - `Dataset(Uuid)` - Dataset data
  - `Metrics { experiment_id, metric_name }` - Metrics data
  - `Statistics { resource_type, aggregation }` - Aggregated stats
  - `Custom(String)` - Custom keys

- **InMemoryCache<K, V>**: High-performance in-memory cache
  - Generic over key and value types
  - Automatic TTL expiration
  - Multiple eviction policies
  - Background cleanup task
  - Thread-safe using DashMap

- **CacheService**: Trait for cache implementations
  - `get()`, `insert()`, `remove()`, `clear()`
  - `insert_with_ttl()` - Custom TTL per entry
  - `len()`, `is_empty()`
  - `statistics()` - Get cache metrics

- **CachedResult<T>**: Wrapper for cached values
  - Indicates cache hit/miss
  - Includes timestamp metadata
  - Helper methods: `hit()`, `miss()`

- **Helper Functions**:
  - `cached()` - Function wrapper for caching query results
  - `invalidate_matching()` - Invalidate entries by predicate

- **CacheStatistics**: Cache performance metrics
  - Hits, misses, evictions
  - Current entries
  - Hit rate calculation

#### Example Usage

```rust
use llm_research_api::performance::{
    CacheConfig, InMemoryCache, CacheKey, cached
};

// Create cache
let config = CacheConfig::builder()
    .default_ttl(Duration::from_secs(300))
    .max_entries(10000)
    .build()?;

let cache = InMemoryCache::<String, String>::new(config);

// Basic usage
let key = CacheKey::Experiment(experiment_id).to_string();
cache.insert(key.clone(), data).await;

// Use cached wrapper
let result = cached(&cache, key, || async {
    // This only runs on cache miss
    fetch_from_database().await
}).await?;

if result.was_cached {
    println!("Cache hit!");
}

// Invalidate matching entries
cache.invalidate_matching(|key| key.starts_with("user:")).await;

// Get statistics
let stats = cache.statistics().await;
println!("Hit rate: {:.2}%", stats.hit_rate * 100.0);
```

#### Tests

**16 comprehensive tests** covering:
- Configuration and builder pattern
- Cache key types and display
- Basic cache operations (insert, get, remove, clear)
- TTL expiration
- Custom TTL per entry
- LRU eviction policy
- Statistics tracking
- Cached result wrappers
- Function wrapper for caching
- Pattern-based invalidation

## Integration with API

The performance module is fully integrated into the main API:

```rust
// In lib.rs
pub use performance::{
    // Connection Pool Management
    PoolConfig, PoolConfigBuilder, PoolError, PoolHealth, PoolMonitor, PoolStatistics,
    create_clickhouse_pool, create_postgres_pool, get_postgres_pool_stats,
    // Query Result Caching
    CacheConfig, CacheConfigBuilder, CacheError, CacheKey, CacheService, CacheStatistics,
    CachedResult, EvictionPolicy, InMemoryCache, cached,
};
```

## Usage Patterns

### 1. Write-Through Cache

```rust
async fn update_experiment(
    cache: &InMemoryCache<String, Experiment>,
    pool: &PgPool,
    id: Uuid,
    data: &Experiment,
) -> Result<()> {
    // Update database first
    sqlx::query!("UPDATE experiments SET ... WHERE id = $1", id)
        .execute(pool)
        .await?;

    // Then update cache
    let key = CacheKey::Experiment(id).to_string();
    cache.insert(key, data.clone()).await;

    Ok(())
}
```

### 2. Cache-Aside Pattern

```rust
async fn get_experiment(
    cache: &InMemoryCache<String, Experiment>,
    pool: &PgPool,
    id: Uuid,
) -> Result<Experiment> {
    let key = CacheKey::Experiment(id).to_string();

    // Try cache first
    if let Some(exp) = cache.get(&key).await {
        return Ok(exp);
    }

    // Fetch from database
    let exp = sqlx::query_as!("SELECT * FROM experiments WHERE id = $1", id)
        .fetch_one(pool)
        .await?;

    // Store in cache
    cache.insert(key, exp.clone()).await;

    Ok(exp)
}
```

### 3. Using the Cached Wrapper

```rust
let result = cached(&cache, key, || async {
    sqlx::query_as!("SELECT * FROM experiments WHERE id = $1", id)
        .fetch_one(pool)
        .await
        .map_err(|e| CacheError::Custom(e.to_string()))
}).await?;
```

## Performance Considerations

### Pool Configuration

- **High Throughput**: Use higher max connections (50+)
- **Low Latency**: Keep more idle connections ready (min = 15-20)
- **Development**: Lower settings to conserve resources (max = 10)

### Cache Configuration

- **TTL**: Balance freshness vs. hit rate
  - Short TTL (60s): Frequently changing data
  - Medium TTL (300s): Stable data with occasional updates
  - Long TTL (3600s): Reference data that rarely changes

- **Max Entries**: Consider memory usage
  - Each entry consumes memory
  - Monitor with `cache.statistics().current_entries`

- **Eviction Policy**:
  - **LRU**: Best for general use, evicts least recently accessed
  - **LFU**: Good for workloads with clear hot/cold data
  - **FIFO**: Simplest, good for time-series data
  - **TTL-only**: No eviction, only expiration

## Dependencies

The module uses the following key dependencies:

- `sqlx` - PostgreSQL connection pooling
- `clickhouse` - ClickHouse client
- `dashmap` - Concurrent hash map for cache storage
- `tokio` - Async runtime
- `chrono` - DateTime handling
- `serde` - Serialization for cache entries
- `thiserror` - Error handling

## Error Handling

Both pool and cache modules use custom error types:

- **PoolError**: Connection pool errors (creation, acquisition, health checks)
- **CacheError**: Cache errors (not found, full, expired, serialization)

All errors implement `std::error::Error` and can be converted to API errors.

## Testing

Run tests with:

```bash
# Test the entire API including performance module
cargo test -p llm-research-api

# Run performance module tests specifically
cargo test -p llm-research-api performance

# Run with output
cargo test -p llm-research-api performance -- --nocapture
```

## Example

See `examples/performance_usage.rs` for a comprehensive example demonstrating all features.

Run with:
```bash
cargo run --example performance_usage
```

## Future Enhancements

Potential improvements for future versions:

1. **Distributed Caching**: Redis/Memcached backends
2. **Cache Warming**: Preload frequently accessed data
3. **Smart Prefetching**: Predictive cache population
4. **Circuit Breaker**: Protect against database overload
5. **Adaptive TTL**: Automatically adjust TTL based on access patterns
6. **Cache Compression**: Reduce memory usage for large values
7. **Metrics Integration**: Export cache/pool metrics to Prometheus

## License

This module is part of the LLM Research API and is licensed under the same terms.
