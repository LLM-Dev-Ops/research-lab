//! Performance optimization module for the LLM Research API.
//!
//! This module provides comprehensive performance optimization capabilities including:
//!
//! - **Connection Pool Management**: Optimized connection pooling for PostgreSQL and ClickHouse
//!   with health monitoring, statistics tracking, and configurable settings.
//!
//! - **Query Result Caching**: Flexible caching system with TTL support, multiple eviction policies,
//!   and automatic cleanup of expired entries.
//!
//! # Examples
//!
//! ## Setting up a connection pool
//!
//! ```no_run
//! use llm_research_api::performance::pool::{PoolConfig, create_postgres_pool};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Create a high-throughput pool configuration
//!     let config = PoolConfig::high_throughput();
//!
//!     // Create the pool
//!     let pool = create_postgres_pool(
//!         "postgresql://localhost/llm_research",
//!         config
//!     ).await?;
//!
//!     // Use the pool for queries...
//!     Ok(())
//! }
//! ```
//!
//! ## Using the cache
//!
//! ```no_run
//! use llm_research_api::performance::cache::{
//!     CacheConfig, InMemoryCache, CacheService, CacheKey
//! };
//! use std::time::Duration;
//! use uuid::Uuid;
//!
//! #[tokio::main]
//! async fn main() {
//!     // Create cache configuration
//!     let config = CacheConfig::builder()
//!         .default_ttl(Duration::from_secs(300))
//!         .max_entries(10000)
//!         .build()
//!         .unwrap();
//!
//!     // Create cache instance
//!     let cache = InMemoryCache::new(config);
//!
//!     // Cache experiment data
//!     let key = CacheKey::Experiment(Uuid::new_v4());
//!     cache.insert(key.to_string(), "experiment_data".to_string()).await;
//!
//!     // Retrieve from cache
//!     if let Some(data) = cache.get(&key.to_string()).await {
//!         println!("Cache hit: {}", data);
//!     }
//!
//!     // Get statistics
//!     let stats = cache.statistics().await;
//!     println!("Cache hit rate: {:.2}%", stats.hit_rate * 100.0);
//! }
//! ```
//!
//! ## Combining pool and cache
//!
//! ```no_run
//! use llm_research_api::performance::{
//!     pool::{PoolConfig, create_postgres_pool},
//!     cache::{CacheConfig, InMemoryCache, cached, CacheKey},
//! };
//! use uuid::Uuid;
//! use std::time::Duration;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Set up pool
//!     let pool_config = PoolConfig::low_latency();
//!     let pool = create_postgres_pool(
//!         "postgresql://localhost/llm_research",
//!         pool_config
//!     ).await?;
//!
//!     // Set up cache
//!     let cache_config = CacheConfig::builder()
//!         .default_ttl(Duration::from_secs(300))
//!         .build()
//!         .unwrap();
//!     let cache = InMemoryCache::new(cache_config);
//!
//!     // Use cached queries
//!     let experiment_id = Uuid::new_v4();
//!     let key = CacheKey::Experiment(experiment_id).to_string();
//!
//!     let result = cached(&cache, key, || async {
//!         // This would be your actual database query
//!         // let experiment = sqlx::query!("SELECT * FROM experiments WHERE id = $1", experiment_id)
//!         //     .fetch_one(&pool)
//!         //     .await?;
//!         Ok::<String, llm_research_api::performance::cache::CacheError>(
//!             "experiment_data".to_string()
//!         )
//!     }).await?;
//!
//!     if result.was_cached {
//!         println!("Loaded from cache!");
//!     } else {
//!         println!("Fetched from database");
//!     }
//!
//!     Ok(())
//! }
//! ```

pub mod cache;
pub mod pool;

// Re-export commonly used types for convenience
pub use cache::{
    CacheConfig, CacheConfigBuilder, CacheError, CacheKey, CacheService, CacheStatistics,
    CachedResult, EvictionPolicy, InMemoryCache, cached,
};

pub use pool::{
    PoolConfig, PoolConfigBuilder, PoolError, PoolHealth, PoolMonitor, PoolStatistics,
    create_clickhouse_pool, create_postgres_pool, get_postgres_pool_stats,
};
