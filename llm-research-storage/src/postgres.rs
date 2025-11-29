use anyhow::Result;
use sqlx::{postgres::PgPoolOptions, PgPool};
use std::time::Duration;

/// Configuration for PostgreSQL connection pool
#[derive(Debug, Clone)]
pub struct PostgresConfig {
    pub database_url: String,
    pub max_connections: u32,
    pub min_connections: u32,
    pub acquire_timeout_seconds: u64,
    pub idle_timeout_seconds: u64,
    pub max_lifetime_seconds: u64,
}

impl Default for PostgresConfig {
    fn default() -> Self {
        Self {
            database_url: String::new(),
            max_connections: 20,
            min_connections: 5,
            acquire_timeout_seconds: 5,
            idle_timeout_seconds: 600,  // 10 minutes
            max_lifetime_seconds: 1800, // 30 minutes
        }
    }
}

impl PostgresConfig {
    pub fn new(database_url: String) -> Self {
        Self {
            database_url,
            ..Default::default()
        }
    }

    pub fn with_max_connections(mut self, max: u32) -> Self {
        self.max_connections = max;
        self
    }

    pub fn with_min_connections(mut self, min: u32) -> Self {
        self.min_connections = min;
        self
    }
}

/// Create a PostgreSQL connection pool with default settings
pub async fn create_pool(database_url: &str) -> Result<PgPool> {
    let config = PostgresConfig::new(database_url.to_string());
    create_pool_with_config(&config).await
}

/// Create a PostgreSQL connection pool with custom configuration
pub async fn create_pool_with_config(config: &PostgresConfig) -> Result<PgPool> {
    let pool = PgPoolOptions::new()
        .max_connections(config.max_connections)
        .min_connections(config.min_connections)
        .acquire_timeout(Duration::from_secs(config.acquire_timeout_seconds))
        .idle_timeout(Some(Duration::from_secs(config.idle_timeout_seconds)))
        .max_lifetime(Some(Duration::from_secs(config.max_lifetime_seconds)))
        .connect(&config.database_url)
        .await?;

    tracing::info!(
        "PostgreSQL connection pool created (max: {}, min: {})",
        config.max_connections,
        config.min_connections
    );

    Ok(pool)
}

/// Run database migrations
pub async fn migrate(pool: &PgPool) -> Result<()> {
    sqlx::migrate!("./migrations").run(pool).await?;
    tracing::info!("Database migrations completed");
    Ok(())
}

/// Health check for database connection
pub async fn health_check(pool: &PgPool) -> Result<()> {
    sqlx::query("SELECT 1")
        .execute(pool)
        .await?;

    tracing::debug!("Database health check passed");
    Ok(())
}

/// Get database pool statistics
pub async fn pool_status(pool: &PgPool) -> PoolStatus {
    let size = pool.size();
    let idle = pool.num_idle();
    PoolStatus {
        size,
        idle,
        active: (size as usize).saturating_sub(idle),
    }
}

#[derive(Debug, Clone)]
pub struct PoolStatus {
    pub size: u32,
    pub idle: usize,
    pub active: usize,
}

impl std::fmt::Display for PoolStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Pool(size: {}, active: {}, idle: {})",
            self.size, self.active, self.idle
        )
    }
}
