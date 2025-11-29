//! Query result caching with TTL support and cache invalidation.
//!
//! This module provides a flexible caching system for database query results with
//! automatic expiration, eviction policies, and comprehensive statistics tracking.
//!
//! # Examples
//!
//! ```no_run
//! use llm_research_api::performance::cache::{CacheConfig, InMemoryCache, CacheKey};
//! use std::time::Duration;
//!
//! #[tokio::main]
//! async fn main() {
//!     let config = CacheConfig::builder()
//!         .default_ttl(Duration::from_secs(300))
//!         .max_entries(1000)
//!         .build()
//!         .unwrap();
//!
//!     let cache = InMemoryCache::<String, String>::new(config);
//!     cache.insert("key".to_string(), "value".to_string()).await;
//! }
//! ```

use async_trait::async_trait;
use chrono::{DateTime, Duration as ChronoDuration, Utc};
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::hash::Hash;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;
use thiserror::Error;
use uuid::Uuid;

/// Errors that can occur during cache operations.
#[derive(Error, Debug)]
pub enum CacheError {
    #[error("Cache entry not found: {0}")]
    NotFound(String),

    #[error("Cache is full (max entries: {0})")]
    CacheFull(usize),

    #[error("Invalid cache configuration: {0}")]
    ConfigurationError(String),

    #[error("Cache entry expired")]
    Expired,

    #[error("Serialization error: {0}")]
    SerializationError(String),

    #[error("Deserialization error: {0}")]
    DeserializationError(String),
}

/// Cache eviction policy.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum EvictionPolicy {
    /// Least Recently Used - evict entries that haven't been accessed recently.
    LRU,

    /// Least Frequently Used - evict entries with the lowest access count.
    LFU,

    /// First In First Out - evict the oldest entries.
    FIFO,

    /// Time To Live only - only evict expired entries.
    TTLOnly,
}

/// Configuration for the cache.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheConfig {
    /// Default time-to-live for cache entries.
    pub default_ttl: Duration,

    /// Maximum number of entries in the cache.
    pub max_entries: usize,

    /// Eviction policy to use when cache is full.
    pub eviction_policy: EvictionPolicy,

    /// Enable automatic cleanup of expired entries.
    pub auto_cleanup: bool,

    /// Interval for automatic cleanup (if enabled).
    pub cleanup_interval: Duration,

    /// Enable statistics tracking.
    pub enable_statistics: bool,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            default_ttl: Duration::from_secs(300), // 5 minutes
            max_entries: 10000,
            eviction_policy: EvictionPolicy::LRU,
            auto_cleanup: true,
            cleanup_interval: Duration::from_secs(60),
            enable_statistics: true,
        }
    }
}

/// Builder for creating CacheConfig instances.
#[derive(Debug, Default)]
pub struct CacheConfigBuilder {
    default_ttl: Option<Duration>,
    max_entries: Option<usize>,
    eviction_policy: Option<EvictionPolicy>,
    auto_cleanup: Option<bool>,
    cleanup_interval: Option<Duration>,
    enable_statistics: Option<bool>,
}

impl CacheConfigBuilder {
    /// Creates a new CacheConfigBuilder.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the default TTL.
    pub fn default_ttl(mut self, ttl: Duration) -> Self {
        self.default_ttl = Some(ttl);
        self
    }

    /// Sets the maximum number of entries.
    pub fn max_entries(mut self, max: usize) -> Self {
        self.max_entries = Some(max);
        self
    }

    /// Sets the eviction policy.
    pub fn eviction_policy(mut self, policy: EvictionPolicy) -> Self {
        self.eviction_policy = Some(policy);
        self
    }

    /// Enables or disables automatic cleanup.
    pub fn auto_cleanup(mut self, enabled: bool) -> Self {
        self.auto_cleanup = Some(enabled);
        self
    }

    /// Sets the cleanup interval.
    pub fn cleanup_interval(mut self, interval: Duration) -> Self {
        self.cleanup_interval = Some(interval);
        self
    }

    /// Enables or disables statistics tracking.
    pub fn enable_statistics(mut self, enabled: bool) -> Self {
        self.enable_statistics = Some(enabled);
        self
    }

    /// Builds the CacheConfig.
    pub fn build(self) -> Result<CacheConfig, CacheError> {
        let default = CacheConfig::default();

        let max_entries = self.max_entries.unwrap_or(default.max_entries);
        if max_entries == 0 {
            return Err(CacheError::ConfigurationError(
                "max_entries must be greater than 0".to_string(),
            ));
        }

        Ok(CacheConfig {
            default_ttl: self.default_ttl.unwrap_or(default.default_ttl),
            max_entries,
            eviction_policy: self.eviction_policy.unwrap_or(default.eviction_policy),
            auto_cleanup: self.auto_cleanup.unwrap_or(default.auto_cleanup),
            cleanup_interval: self.cleanup_interval.unwrap_or(default.cleanup_interval),
            enable_statistics: self.enable_statistics.unwrap_or(default.enable_statistics),
        })
    }
}

impl CacheConfig {
    /// Creates a new CacheConfigBuilder.
    pub fn builder() -> CacheConfigBuilder {
        CacheConfigBuilder::new()
    }
}

/// Cache key namespace for different types of cached data.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CacheKey {
    /// Cache key for experiment data.
    Experiment(Uuid),

    /// Cache key for experiment list with optional filters.
    ExperimentList { page: usize, limit: usize, filter: Option<String> },

    /// Cache key for model data.
    Model(Uuid),

    /// Cache key for model list with optional filters.
    ModelList { page: usize, limit: usize, filter: Option<String> },

    /// Cache key for dataset data.
    Dataset(Uuid),

    /// Cache key for dataset list with optional filters.
    DatasetList { page: usize, limit: usize, filter: Option<String> },

    /// Cache key for metrics data.
    Metrics { experiment_id: Uuid, metric_name: String },

    /// Cache key for aggregated statistics.
    Statistics { resource_type: String, aggregation: String },

    /// Custom cache key for any other data.
    Custom(String),
}

impl fmt::Display for CacheKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CacheKey::Experiment(id) => write!(f, "experiment:{}", id),
            CacheKey::ExperimentList { page, limit, filter } => {
                write!(f, "experiment_list:{}:{}:{}", page, limit, filter.as_deref().unwrap_or("none"))
            }
            CacheKey::Model(id) => write!(f, "model:{}", id),
            CacheKey::ModelList { page, limit, filter } => {
                write!(f, "model_list:{}:{}:{}", page, limit, filter.as_deref().unwrap_or("none"))
            }
            CacheKey::Dataset(id) => write!(f, "dataset:{}", id),
            CacheKey::DatasetList { page, limit, filter } => {
                write!(f, "dataset_list:{}:{}:{}", page, limit, filter.as_deref().unwrap_or("none"))
            }
            CacheKey::Metrics { experiment_id, metric_name } => {
                write!(f, "metrics:{}:{}", experiment_id, metric_name)
            }
            CacheKey::Statistics { resource_type, aggregation } => {
                write!(f, "stats:{}:{}", resource_type, aggregation)
            }
            CacheKey::Custom(key) => write!(f, "custom:{}", key),
        }
    }
}

/// A cached value with metadata.
#[derive(Debug, Clone)]
struct CacheEntry<V> {
    value: V,
    created_at: DateTime<Utc>,
    expires_at: DateTime<Utc>,
    last_accessed: DateTime<Utc>,
    access_count: u64,
}

impl<V> CacheEntry<V> {
    fn new(value: V, ttl: Duration) -> Self {
        let now = Utc::now();
        let ttl_chrono = ChronoDuration::from_std(ttl).unwrap_or(ChronoDuration::seconds(300));

        Self {
            value,
            created_at: now,
            expires_at: now + ttl_chrono,
            last_accessed: now,
            access_count: 0,
        }
    }

    fn is_expired(&self) -> bool {
        Utc::now() > self.expires_at
    }

    fn access(&mut self) {
        self.last_accessed = Utc::now();
        self.access_count += 1;
    }
}

/// Wrapper for cached results with hit/miss tracking.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedResult<T> {
    /// The cached value.
    pub value: T,

    /// Whether this was a cache hit.
    pub was_cached: bool,

    /// Timestamp when the value was retrieved.
    pub retrieved_at: DateTime<Utc>,

    /// Original creation time of the cached entry (if cached).
    pub cached_at: Option<DateTime<Utc>>,
}

impl<T> CachedResult<T> {
    /// Creates a new CachedResult from a cache hit.
    pub fn hit(value: T, cached_at: DateTime<Utc>) -> Self {
        Self {
            value,
            was_cached: true,
            retrieved_at: Utc::now(),
            cached_at: Some(cached_at),
        }
    }

    /// Creates a new CachedResult from a cache miss.
    pub fn miss(value: T) -> Self {
        Self {
            value,
            was_cached: false,
            retrieved_at: Utc::now(),
            cached_at: None,
        }
    }
}

/// Cache statistics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheStatistics {
    /// Total number of cache hits.
    pub hits: u64,

    /// Total number of cache misses.
    pub misses: u64,

    /// Total number of evictions.
    pub evictions: u64,

    /// Total number of entries expired and removed.
    pub expirations: u64,

    /// Current number of entries in cache.
    pub current_entries: usize,

    /// Maximum allowed entries.
    pub max_entries: usize,

    /// Cache hit rate (0.0 - 1.0).
    pub hit_rate: f64,

    /// Timestamp when statistics were collected.
    pub collected_at: DateTime<Utc>,
}

impl CacheStatistics {
    fn new(max_entries: usize) -> Self {
        Self {
            hits: 0,
            misses: 0,
            evictions: 0,
            expirations: 0,
            current_entries: 0,
            max_entries,
            hit_rate: 0.0,
            collected_at: Utc::now(),
        }
    }

    fn calculate_hit_rate(&mut self) {
        let total = self.hits + self.misses;
        self.hit_rate = if total > 0 {
            self.hits as f64 / total as f64
        } else {
            0.0
        };
        self.collected_at = Utc::now();
    }
}

/// Trait for cache service implementations.
#[async_trait]
pub trait CacheService<K, V>: Send + Sync {
    /// Gets a value from the cache.
    async fn get(&self, key: &K) -> Option<V>;

    /// Inserts a value into the cache.
    async fn insert(&self, key: K, value: V);

    /// Inserts a value with a custom TTL.
    async fn insert_with_ttl(&self, key: K, value: V, ttl: Duration);

    /// Removes a value from the cache.
    async fn remove(&self, key: &K) -> Option<V>;

    /// Clears all entries from the cache.
    async fn clear(&self);

    /// Gets the current number of entries in the cache.
    async fn len(&self) -> usize;

    /// Checks if the cache is empty.
    async fn is_empty(&self) -> bool {
        self.len().await == 0
    }

    /// Gets cache statistics.
    async fn statistics(&self) -> CacheStatistics;
}

/// In-memory cache implementation using DashMap.
pub struct InMemoryCache<K, V>
where
    K: Hash + Eq + Clone + Send + Sync,
    V: Clone + Send + Sync,
{
    config: CacheConfig,
    entries: Arc<DashMap<K, CacheEntry<V>>>,
    stats_hits: Arc<AtomicU64>,
    stats_misses: Arc<AtomicU64>,
    stats_evictions: Arc<AtomicU64>,
    stats_expirations: Arc<AtomicU64>,
}

impl<K, V> InMemoryCache<K, V>
where
    K: Hash + Eq + Clone + Send + Sync + 'static,
    V: Clone + Send + Sync + 'static,
{
    /// Creates a new in-memory cache with the given configuration.
    pub fn new(config: CacheConfig) -> Self {
        let cache = Self {
            config,
            entries: Arc::new(DashMap::new()),
            stats_hits: Arc::new(AtomicU64::new(0)),
            stats_misses: Arc::new(AtomicU64::new(0)),
            stats_evictions: Arc::new(AtomicU64::new(0)),
            stats_expirations: Arc::new(AtomicU64::new(0)),
        };

        // Start background cleanup task if enabled
        if cache.config.auto_cleanup {
            let entries = cache.entries.clone();
            let expirations = cache.stats_expirations.clone();
            let interval = cache.config.cleanup_interval;

            tokio::spawn(async move {
                let mut ticker = tokio::time::interval(interval);
                loop {
                    ticker.tick().await;
                    Self::cleanup_expired_entries(&entries, &expirations);
                }
            });
        }

        cache
    }

    fn cleanup_expired_entries(
        entries: &DashMap<K, CacheEntry<V>>,
        expirations: &AtomicU64,
    ) {
        let expired_keys: Vec<K> = entries
            .iter()
            .filter(|entry| entry.value().is_expired())
            .map(|entry| entry.key().clone())
            .collect();

        for key in expired_keys {
            entries.remove(&key);
            expirations.fetch_add(1, Ordering::Relaxed);
        }
    }

    fn evict_entry(&self) {
        if self.entries.is_empty() {
            return;
        }

        let key_to_evict = match self.config.eviction_policy {
            EvictionPolicy::LRU => self.find_lru_key(),
            EvictionPolicy::LFU => self.find_lfu_key(),
            EvictionPolicy::FIFO => self.find_fifo_key(),
            EvictionPolicy::TTLOnly => return, // Don't evict if TTL only
        };

        if let Some(key) = key_to_evict {
            self.entries.remove(&key);
            self.stats_evictions.fetch_add(1, Ordering::Relaxed);
        }
    }

    fn find_lru_key(&self) -> Option<K> {
        self.entries
            .iter()
            .min_by_key(|entry| entry.value().last_accessed)
            .map(|entry| entry.key().clone())
    }

    fn find_lfu_key(&self) -> Option<K> {
        self.entries
            .iter()
            .min_by_key(|entry| entry.value().access_count)
            .map(|entry| entry.key().clone())
    }

    fn find_fifo_key(&self) -> Option<K> {
        self.entries
            .iter()
            .min_by_key(|entry| entry.value().created_at)
            .map(|entry| entry.key().clone())
    }

    /// Invalidates all cache entries matching a predicate.
    pub async fn invalidate_matching<F>(&self, predicate: F)
    where
        F: Fn(&K) -> bool,
    {
        let keys_to_remove: Vec<K> = self
            .entries
            .iter()
            .filter(|entry| predicate(entry.key()))
            .map(|entry| entry.key().clone())
            .collect();

        for key in keys_to_remove {
            self.entries.remove(&key);
        }
    }
}

#[async_trait]
impl<K, V> CacheService<K, V> for InMemoryCache<K, V>
where
    K: Hash + Eq + Clone + Send + Sync + 'static,
    V: Clone + Send + Sync + 'static,
{
    async fn get(&self, key: &K) -> Option<V> {
        if let Some(mut entry) = self.entries.get_mut(key) {
            if entry.is_expired() {
                drop(entry);
                self.entries.remove(key);
                self.stats_expirations.fetch_add(1, Ordering::Relaxed);
                self.stats_misses.fetch_add(1, Ordering::Relaxed);
                return None;
            }

            entry.access();
            self.stats_hits.fetch_add(1, Ordering::Relaxed);
            Some(entry.value.clone())
        } else {
            self.stats_misses.fetch_add(1, Ordering::Relaxed);
            None
        }
    }

    async fn insert(&self, key: K, value: V) {
        self.insert_with_ttl(key, value, self.config.default_ttl).await;
    }

    async fn insert_with_ttl(&self, key: K, value: V, ttl: Duration) {
        // Check if we need to evict
        if self.entries.len() >= self.config.max_entries && !self.entries.contains_key(&key) {
            self.evict_entry();
        }

        let entry = CacheEntry::new(value, ttl);
        self.entries.insert(key, entry);
    }

    async fn remove(&self, key: &K) -> Option<V> {
        self.entries.remove(key).map(|(_, entry)| entry.value)
    }

    async fn clear(&self) {
        self.entries.clear();
    }

    async fn len(&self) -> usize {
        self.entries.len()
    }

    async fn statistics(&self) -> CacheStatistics {
        let mut stats = CacheStatistics::new(self.config.max_entries);
        stats.hits = self.stats_hits.load(Ordering::Relaxed);
        stats.misses = self.stats_misses.load(Ordering::Relaxed);
        stats.evictions = self.stats_evictions.load(Ordering::Relaxed);
        stats.expirations = self.stats_expirations.load(Ordering::Relaxed);
        stats.current_entries = self.entries.len();
        stats.calculate_hit_rate();
        stats
    }
}

/// Function wrapper for caching query results.
///
/// This is a helper function to wrap database queries with caching logic.
pub async fn cached<K, V, F, Fut>(
    cache: &InMemoryCache<K, V>,
    key: K,
    fetch_fn: F,
) -> Result<CachedResult<V>, CacheError>
where
    K: Hash + Eq + Clone + Send + Sync + 'static,
    V: Clone + Send + Sync + 'static,
    F: FnOnce() -> Fut,
    Fut: std::future::Future<Output = Result<V, CacheError>>,
{
    // Try to get from cache first
    if let Some(cached_value) = cache.get(&key).await {
        return Ok(CachedResult::hit(cached_value, Utc::now()));
    }

    // Cache miss - fetch the value
    let value = fetch_fn().await?;

    // Store in cache
    cache.insert(key, value.clone()).await;

    Ok(CachedResult::miss(value))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    use tokio::time::sleep;

    #[test]
    fn test_cache_config_default() {
        let config = CacheConfig::default();
        assert_eq!(config.default_ttl, Duration::from_secs(300));
        assert_eq!(config.max_entries, 10000);
        assert_eq!(config.eviction_policy, EvictionPolicy::LRU);
        assert!(config.auto_cleanup);
        assert!(config.enable_statistics);
    }

    #[test]
    fn test_cache_config_builder() {
        let config = CacheConfig::builder()
            .default_ttl(Duration::from_secs(600))
            .max_entries(5000)
            .eviction_policy(EvictionPolicy::LFU)
            .auto_cleanup(false)
            .enable_statistics(true)
            .build()
            .unwrap();

        assert_eq!(config.default_ttl, Duration::from_secs(600));
        assert_eq!(config.max_entries, 5000);
        assert_eq!(config.eviction_policy, EvictionPolicy::LFU);
        assert!(!config.auto_cleanup);
    }

    #[test]
    fn test_cache_config_builder_validation() {
        let result = CacheConfig::builder()
            .max_entries(0)
            .build();

        assert!(result.is_err());
    }

    #[test]
    fn test_cache_key_display() {
        let key = CacheKey::Experiment(Uuid::new_v4());
        assert!(key.to_string().starts_with("experiment:"));

        let key = CacheKey::Model(Uuid::new_v4());
        assert!(key.to_string().starts_with("model:"));

        let key = CacheKey::Custom("test".to_string());
        assert_eq!(key.to_string(), "custom:test");
    }

    #[tokio::test]
    async fn test_in_memory_cache_basic() {
        let config = CacheConfig::builder()
            .default_ttl(Duration::from_secs(60))
            .max_entries(100)
            .auto_cleanup(false)
            .build()
            .unwrap();

        let cache = InMemoryCache::new(config);

        cache.insert("key1".to_string(), "value1".to_string()).await;
        assert_eq!(cache.get(&"key1".to_string()).await, Some("value1".to_string()));
        assert_eq!(cache.len().await, 1);
    }

    #[tokio::test]
    async fn test_in_memory_cache_miss() {
        let config = CacheConfig::default();
        let cache: InMemoryCache<String, String> = InMemoryCache::new(config);

        assert_eq!(cache.get(&"nonexistent".to_string()).await, None);
    }

    #[tokio::test]
    async fn test_in_memory_cache_expiration() {
        let config = CacheConfig::builder()
            .default_ttl(Duration::from_millis(100))
            .auto_cleanup(false)
            .build()
            .unwrap();

        let cache = InMemoryCache::new(config);

        cache.insert("key1".to_string(), "value1".to_string()).await;
        assert_eq!(cache.get(&"key1".to_string()).await, Some("value1".to_string()));

        sleep(Duration::from_millis(150)).await;

        assert_eq!(cache.get(&"key1".to_string()).await, None);
    }

    #[tokio::test]
    async fn test_in_memory_cache_custom_ttl() {
        let config = CacheConfig::default();
        let cache = InMemoryCache::new(config);

        cache.insert_with_ttl(
            "key1".to_string(),
            "value1".to_string(),
            Duration::from_millis(50),
        ).await;

        assert_eq!(cache.get(&"key1".to_string()).await, Some("value1".to_string()));

        sleep(Duration::from_millis(100)).await;
        assert_eq!(cache.get(&"key1".to_string()).await, None);
    }

    #[tokio::test]
    async fn test_in_memory_cache_remove() {
        let config = CacheConfig::default();
        let cache = InMemoryCache::new(config);

        cache.insert("key1".to_string(), "value1".to_string()).await;
        assert_eq!(cache.len().await, 1);

        let removed = cache.remove(&"key1".to_string()).await;
        assert_eq!(removed, Some("value1".to_string()));
        assert_eq!(cache.len().await, 0);
    }

    #[tokio::test]
    async fn test_in_memory_cache_clear() {
        let config = CacheConfig::default();
        let cache = InMemoryCache::new(config);

        cache.insert("key1".to_string(), "value1".to_string()).await;
        cache.insert("key2".to_string(), "value2".to_string()).await;
        assert_eq!(cache.len().await, 2);

        cache.clear().await;
        assert_eq!(cache.len().await, 0);
    }

    #[tokio::test]
    async fn test_in_memory_cache_eviction_lru() {
        let config = CacheConfig::builder()
            .max_entries(2)
            .eviction_policy(EvictionPolicy::LRU)
            .auto_cleanup(false)
            .build()
            .unwrap();

        let cache = InMemoryCache::new(config);

        cache.insert("key1".to_string(), "value1".to_string()).await;
        sleep(Duration::from_millis(10)).await;
        cache.insert("key2".to_string(), "value2".to_string()).await;

        // Access key1 to make it more recently used
        cache.get(&"key1".to_string()).await;

        // This should evict key2 (least recently used)
        cache.insert("key3".to_string(), "value3".to_string()).await;

        assert!(cache.get(&"key1".to_string()).await.is_some());
        assert!(cache.get(&"key2".to_string()).await.is_none());
        assert!(cache.get(&"key3".to_string()).await.is_some());
    }

    #[tokio::test]
    async fn test_in_memory_cache_statistics() {
        let config = CacheConfig::default();
        let cache = InMemoryCache::new(config);

        cache.insert("key1".to_string(), "value1".to_string()).await;

        // Generate some hits
        cache.get(&"key1".to_string()).await;
        cache.get(&"key1".to_string()).await;

        // Generate some misses
        cache.get(&"key2".to_string()).await;

        let stats = cache.statistics().await;
        assert_eq!(stats.hits, 2);
        assert_eq!(stats.misses, 1);
        assert_eq!(stats.current_entries, 1);
        assert!(stats.hit_rate > 0.0);
    }

    #[tokio::test]
    async fn test_cached_result_hit() {
        let result = CachedResult::hit("value".to_string(), Utc::now());
        assert!(result.was_cached);
        assert!(result.cached_at.is_some());
    }

    #[tokio::test]
    async fn test_cached_result_miss() {
        let result = CachedResult::miss("value".to_string());
        assert!(!result.was_cached);
        assert!(result.cached_at.is_none());
    }

    #[tokio::test]
    async fn test_cached_function_wrapper() {
        let config = CacheConfig::default();
        let cache = InMemoryCache::new(config);

        let fetch_count = Arc::new(AtomicU64::new(0));
        let fetch_count_clone = fetch_count.clone();

        // First call should fetch
        let result1 = cached(&cache, "key1".to_string(), || {
            let count = fetch_count_clone.clone();
            async move {
                count.fetch_add(1, Ordering::Relaxed);
                Ok::<String, CacheError>("value1".to_string())
            }
        }).await.unwrap();

        assert!(!result1.was_cached);
        assert_eq!(fetch_count.load(Ordering::Relaxed), 1);

        // Second call should hit cache
        let result2 = cached(&cache, "key1".to_string(), || {
            let count = fetch_count_clone.clone();
            async move {
                count.fetch_add(1, Ordering::Relaxed);
                Ok::<String, CacheError>("value1".to_string())
            }
        }).await.unwrap();

        assert!(result2.was_cached);
        assert_eq!(fetch_count.load(Ordering::Relaxed), 1); // Fetch count unchanged
    }

    #[tokio::test]
    async fn test_invalidate_matching() {
        let config = CacheConfig::default();
        let cache = InMemoryCache::new(config);

        cache.insert("user:1".to_string(), "data1".to_string()).await;
        cache.insert("user:2".to_string(), "data2".to_string()).await;
        cache.insert("product:1".to_string(), "data3".to_string()).await;

        cache.invalidate_matching(|key| key.starts_with("user:")).await;

        assert!(cache.get(&"user:1".to_string()).await.is_none());
        assert!(cache.get(&"user:2".to_string()).await.is_none());
        assert!(cache.get(&"product:1".to_string()).await.is_some());
    }
}
