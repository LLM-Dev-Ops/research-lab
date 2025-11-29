//! Example usage of the performance module
//!
//! This example demonstrates how to use the connection pool and caching
//! features of the performance module.
//!
//! Run with:
//! ```bash
//! cargo run --example performance_usage --features example
//! ```

use llm_research_api::performance::{
    // Pool management
    PoolConfig, PoolMonitor, create_postgres_pool, get_postgres_pool_stats,
    create_clickhouse_pool,
    // Cache management
    CacheConfig, CacheKey, CacheService, InMemoryCache, cached, EvictionPolicy,
};
use std::time::Duration;
use uuid::Uuid;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Performance Module Example ===\n");

    // ===================================================================
    // Part 1: Connection Pool Configuration
    // ===================================================================
    println!("1. Setting up PostgreSQL connection pool...");

    // Create a high-throughput pool configuration
    let pool_config = PoolConfig::builder()
        .min_connections(10)
        .max_connections(50)
        .acquire_timeout(Duration::from_secs(10))
        .idle_timeout(Some(Duration::from_secs(300)))
        .max_lifetime(Some(Duration::from_secs(1800)))
        .test_on_acquire(true)
        .build()?;

    println!("   Pool config: {:?}", pool_config);

    // Alternative: Use predefined configurations
    let _high_throughput = PoolConfig::high_throughput();
    let _low_latency = PoolConfig::low_latency();
    let _development = PoolConfig::development();

    // In a real application, you would create the pool:
    // let pool = create_postgres_pool("postgresql://localhost/llm_research", pool_config).await?;

    // ===================================================================
    // Part 2: Pool Monitoring
    // ===================================================================
    println!("\n2. Setting up pool monitoring...");

    let pool_monitor = PoolMonitor::new(100); // Keep 100 stats entries

    // Simulate pool statistics (in real app, these come from the actual pool)
    // let stats = get_postgres_pool_stats(&pool);
    // pool_monitor.record_statistics(stats).await;

    // Check pool health
    let health = pool_monitor.get_health_status().await;
    println!("   Pool health: {:?}", health);

    // ===================================================================
    // Part 3: Cache Configuration
    // ===================================================================
    println!("\n3. Setting up cache...");

    let cache_config = CacheConfig::builder()
        .default_ttl(Duration::from_secs(300)) // 5 minutes
        .max_entries(10000)
        .eviction_policy(EvictionPolicy::LRU)
        .auto_cleanup(true)
        .cleanup_interval(Duration::from_secs(60))
        .enable_statistics(true)
        .build()?;

    println!("   Cache config: max_entries={}, ttl={:?}",
             cache_config.max_entries, cache_config.default_ttl);

    // Create cache instances for different data types
    type ExperimentCache = InMemoryCache<String, String>;
    let experiment_cache = ExperimentCache::new(cache_config.clone());

    // ===================================================================
    // Part 4: Basic Cache Operations
    // ===================================================================
    println!("\n4. Performing cache operations...");

    // Insert into cache
    let experiment_id = Uuid::new_v4();
    let key = CacheKey::Experiment(experiment_id).to_string();
    experiment_cache.insert(key.clone(), "experiment_data".to_string()).await;
    println!("   Inserted experiment {}", experiment_id);

    // Retrieve from cache
    if let Some(data) = experiment_cache.get(&key).await {
        println!("   Cache HIT: {}", data);
    } else {
        println!("   Cache MISS");
    }

    // Insert with custom TTL
    let temp_key = CacheKey::Custom("temp".to_string()).to_string();
    experiment_cache.insert_with_ttl(
        temp_key.clone(),
        "temporary_data".to_string(),
        Duration::from_secs(10),
    ).await;
    println!("   Inserted temporary data with 10s TTL");

    // ===================================================================
    // Part 5: Cache Namespaces
    // ===================================================================
    println!("\n5. Using cache namespaces...");

    // Different cache key types
    let model_key = CacheKey::Model(Uuid::new_v4());
    let dataset_key = CacheKey::Dataset(Uuid::new_v4());
    let metrics_key = CacheKey::Metrics {
        experiment_id: Uuid::new_v4(),
        metric_name: "accuracy".to_string(),
    };
    let list_key = CacheKey::ExperimentList {
        page: 1,
        limit: 20,
        filter: Some("status:completed".to_string()),
    };

    println!("   Model key: {}", model_key);
    println!("   Dataset key: {}", dataset_key);
    println!("   Metrics key: {}", metrics_key);
    println!("   List key: {}", list_key);

    // ===================================================================
    // Part 6: Using the `cached` Helper Function
    // ===================================================================
    println!("\n6. Using cached query wrapper...");

    let query_key = CacheKey::Experiment(Uuid::new_v4()).to_string();

    // First call - will fetch from "database"
    let result1 = cached(&experiment_cache, query_key.clone(), || async {
        println!("   Fetching from database...");
        tokio::time::sleep(Duration::from_millis(100)).await;
        Ok::<String, llm_research_api::performance::CacheError>(
            "fresh_experiment_data".to_string()
        )
    }).await?;

    println!("   First call - was_cached: {}", result1.was_cached);

    // Second call - will hit cache
    let result2 = cached(&experiment_cache, query_key.clone(), || async {
        println!("   This should not be called!");
        Ok::<String, llm_research_api::performance::CacheError>(
            "fresh_experiment_data".to_string()
        )
    }).await?;

    println!("   Second call - was_cached: {}", result2.was_cached);

    // ===================================================================
    // Part 7: Cache Invalidation
    // ===================================================================
    println!("\n7. Cache invalidation...");

    // Insert multiple entries
    for i in 0..5 {
        let key = format!("user:{}", i);
        experiment_cache.insert(key, format!("user_data_{}", i)).await;
    }

    experiment_cache.insert("product:1".to_string(), "product_data".to_string()).await;

    println!("   Inserted 6 entries");
    println!("   Cache size: {}", experiment_cache.len().await);

    // Invalidate all user entries
    experiment_cache.invalidate_matching(|key| key.starts_with("user:")).await;

    println!("   After invalidating user entries: {}", experiment_cache.len().await);

    // ===================================================================
    // Part 8: Cache Statistics
    // ===================================================================
    println!("\n8. Cache statistics...");

    let stats = experiment_cache.statistics().await;
    println!("   Hits: {}", stats.hits);
    println!("   Misses: {}", stats.misses);
    println!("   Evictions: {}", stats.evictions);
    println!("   Current entries: {}", stats.current_entries);
    println!("   Hit rate: {:.2}%", stats.hit_rate * 100.0);

    // ===================================================================
    // Part 9: ClickHouse Pool
    // ===================================================================
    println!("\n9. ClickHouse pool configuration...");

    let ch_config = PoolConfig::builder()
        .min_connections(5)
        .max_connections(20)
        .build()?;

    // In a real application:
    // let ch_client = create_clickhouse_pool("http://localhost:8123", ch_config)?;
    println!("   ClickHouse config ready: {:?}", ch_config);

    // ===================================================================
    // Part 10: Advanced Cache Patterns
    // ===================================================================
    println!("\n10. Advanced cache patterns...");

    // Write-through cache: update both cache and database
    async fn update_experiment_write_through(
        cache: &ExperimentCache,
        id: Uuid,
        data: String,
    ) {
        // In real app: update database first
        // db.update_experiment(id, &data).await?;

        // Then update cache
        let key = CacheKey::Experiment(id).to_string();
        cache.insert(key, data).await;
    }

    let exp_id = Uuid::new_v4();
    update_experiment_write_through(&experiment_cache, exp_id, "updated_data".to_string()).await;
    println!("   Write-through cache updated for {}", exp_id);

    // Cache-aside pattern: check cache first, then database
    async fn get_experiment_cache_aside(
        cache: &ExperimentCache,
        id: Uuid,
    ) -> Option<String> {
        let key = CacheKey::Experiment(id).to_string();

        // Try cache first
        if let Some(data) = cache.get(&key).await {
            return Some(data);
        }

        // Cache miss - fetch from database
        // let data = db.get_experiment(id).await?;

        // Store in cache for next time
        let data = format!("experiment_data_{}", id);
        cache.insert(key, data.clone()).await;

        Some(data)
    }

    let _ = get_experiment_cache_aside(&experiment_cache, Uuid::new_v4()).await;
    println!("   Cache-aside pattern demonstrated");

    println!("\n=== Example complete! ===");

    Ok(())
}
