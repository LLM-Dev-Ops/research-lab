//! Example of Kubernetes-ready health check integration
//!
//! This example demonstrates a production-ready setup with:
//! - Separate liveness and readiness probes
//! - Critical vs non-critical component configuration
//! - Health check caching to reduce load on dependencies
//! - Proper timeout handling
//! - Integration with application state
//!
//! Kubernetes configuration example:
//! ```yaml
//! livenessProbe:
//!   httpGet:
//!     path: /health/live
//!     port: 8080
//!   initialDelaySeconds: 30
//!   periodSeconds: 10
//!   timeoutSeconds: 5
//!   failureThreshold: 3
//!
//! readinessProbe:
//!   httpGet:
//!     path: /health/ready
//!     port: 8080
//!   initialDelaySeconds: 10
//!   periodSeconds: 5
//!   timeoutSeconds: 3
//!   failureThreshold: 2
//! ```

use axum::{
    routing::get,
    Router,
    extract::State,
};
use llm_research_api::{
    HealthCheckRegistry, HealthCheckState, HealthCheckConfig,
    PostgresHealthCheck, ClickHouseHealthCheck, S3HealthCheck,
    liveness_handler, readiness_handler, health_handler,
};
use sqlx::postgres::PgPoolOptions;
use aws_config::BehaviorVersion;
use std::sync::Arc;
use std::time::Duration;

/// Application state with both business logic and health checks
#[derive(Clone)]
struct AppState {
    // Business logic state
    db_pool: sqlx::PgPool,
    s3_client: aws_sdk_s3::Client,
    s3_bucket: String,

    // Health check state
    health: HealthCheckState,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_target(false)
        .with_level(true)
        .init();

    println!("\n🏥 Kubernetes Health Check Integration Example\n");
    println!("{}", "=".repeat(60));

    // 1. Initialize dependencies
    println!("\n1️⃣  Initializing dependencies...");

    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://postgres:postgres@localhost:5432/llm_research".to_string());

    let db_pool = PgPoolOptions::new()
        .max_connections(10)
        .acquire_timeout(Duration::from_secs(3))
        .connect(&database_url)
        .await?;

    println!("   ✓ PostgreSQL connected");

    let clickhouse_url = std::env::var("CLICKHOUSE_URL")
        .unwrap_or_else(|_| "http://localhost:8123".to_string());

    let clickhouse_client = clickhouse::Client::default()
        .with_url(clickhouse_url);

    println!("   ✓ ClickHouse configured");

    let aws_config = aws_config::defaults(BehaviorVersion::latest())
        .load()
        .await;

    let s3_client = aws_sdk_s3::Client::new(&aws_config);
    let s3_bucket = std::env::var("S3_BUCKET")
        .unwrap_or_else(|_| "llm-research-dev".to_string());

    println!("   ✓ S3 configured");

    // 2. Configure health checks
    println!("\n2️⃣  Configuring health checks...");

    // PostgreSQL is CRITICAL - must be healthy for readiness
    let postgres_check = Arc::new(PostgresHealthCheck::with_config(
        db_pool.clone(),
        HealthCheckConfig::critical()
            .with_timeout(Duration::from_secs(2))      // Fast timeout
            .with_cache_ttl(Duration::from_secs(30))   // Cache for 30s
    ));
    println!("   ✓ PostgreSQL health check (critical)");

    // ClickHouse is NON-CRITICAL - degraded if unavailable
    let clickhouse_check = Arc::new(ClickHouseHealthCheck::with_config(
        clickhouse_client,
        HealthCheckConfig::non_critical()
            .with_timeout(Duration::from_secs(5))      // Longer timeout ok
            .with_cache_ttl(Duration::from_secs(60))   // Cache longer
    ));
    println!("   ✓ ClickHouse health check (non-critical)");

    // S3 is CRITICAL - must be healthy for readiness
    let s3_check = Arc::new(S3HealthCheck::with_config(
        s3_client.clone(),
        s3_bucket.clone(),
        HealthCheckConfig::critical()
            .with_timeout(Duration::from_secs(5))
            .with_cache_ttl(Duration::from_secs(45))
    ));
    println!("   ✓ S3 health check (critical)");

    // 3. Create health check registry
    println!("\n3️⃣  Creating health check registry...");

    let version = env!("CARGO_PKG_VERSION");
    let registry = Arc::new(
        HealthCheckRegistry::new(version)
            .register(postgres_check)
            .register(clickhouse_check)
            .register(s3_check)
    );

    let health_state = HealthCheckState::new(registry);

    println!("   ✓ Registry created with 3 health checks");
    println!("   ✓ Critical checks: PostgreSQL, S3");
    println!("   ✓ Non-critical checks: ClickHouse");

    // 4. Create application state
    let app_state = AppState {
        db_pool: db_pool.clone(),
        s3_client: s3_client.clone(),
        s3_bucket: s3_bucket.clone(),
        health: health_state,
    };

    // 5. Build router with health endpoints
    println!("\n4️⃣  Building HTTP router...");

    // Health check routes with their own state
    let health_routes = Router::new()
        .route("/live", get(liveness_handler))
        .route("/ready", get(readiness_handler))
        .route("/", get(health_handler))
        .with_state(app_state.health.clone());

    // Business routes with full app state
    let api_routes = Router::new()
        .route("/status", get(api_status))
        .with_state(app_state);

    let app = Router::new()
        .nest("/health", health_routes)
        .nest("/api/v1", api_routes);

    println!("   ✓ Routes configured");

    // 6. Start server
    println!("\n5️⃣  Starting HTTP server...");

    let listener = tokio::net::TcpListener::bind("0.0.0.0:8080").await?;
    let addr = listener.local_addr()?;

    println!("\n{}", "=".repeat(60));
    println!("🚀 Server running on http://{}\n", addr);
    println!("Health Check Endpoints:");
    println!("  ├─ http://{}/health/live  (Kubernetes liveness probe)", addr);
    println!("  ├─ http://{}/health/ready (Kubernetes readiness probe)", addr);
    println!("  └─ http://{}/health       (Detailed diagnostics)\n", addr);
    println!("API Endpoints:");
    println!("  └─ http://{}/api/v1/status\n", addr);
    println!("{}", "=".repeat(60));
    println!("\n💡 Health Check Behavior:");
    println!("  • Liveness:  Always returns 200 (app is alive)");
    println!("  • Readiness: Returns 503 if critical deps unhealthy");
    println!("  • Detailed:  Shows all component statuses + metrics\n");
    println!("⏱️  Health Check Performance:");
    println!("  • Results cached for 30-60s to reduce dependency load");
    println!("  • Timeouts: 2-5s depending on component");
    println!("  • Concurrent execution for fast response\n");
    println!("Press Ctrl+C to stop\n");

    axum::serve(listener, app).await?;

    Ok(())
}

/// Example API endpoint that uses application state
async fn api_status(State(state): State<AppState>) -> &'static str {
    // Could use state.db_pool, state.s3_client, etc.
    let _ = state;
    "API is operational"
}
