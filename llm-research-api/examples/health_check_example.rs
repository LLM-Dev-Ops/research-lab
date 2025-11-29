//! Example demonstrating health check endpoints
//!
//! This example shows how to:
//! 1. Set up health checks for PostgreSQL, ClickHouse, and S3
//! 2. Create a health check registry
//! 3. Configure health check handlers for Kubernetes probes
//! 4. Test the health endpoints
//!
//! Run with:
//! ```bash
//! cargo run --example health_check_example
//! ```

use axum::{
    routing::get,
    Router,
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
use tokio::net::TcpListener;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    println!("Health Check Example - Setting up health checks...\n");

    // 1. Set up PostgreSQL connection pool
    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://postgres:postgres@localhost:5432/llm_research".to_string());

    let db_pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await?;

    println!("✓ PostgreSQL connection pool created");

    // 2. Set up ClickHouse client (optional, non-critical)
    let clickhouse_url = std::env::var("CLICKHOUSE_URL")
        .unwrap_or_else(|_| "http://localhost:8123".to_string());

    let clickhouse_client = clickhouse::Client::default()
        .with_url(clickhouse_url);

    println!("✓ ClickHouse client created");

    // 3. Set up S3 client
    let aws_config = aws_config::defaults(BehaviorVersion::latest())
        .load()
        .await;

    let s3_client = aws_sdk_s3::Client::new(&aws_config);
    let s3_bucket = std::env::var("S3_BUCKET")
        .unwrap_or_else(|_| "llm-research-dev".to_string());

    println!("✓ S3 client created");

    // 4. Create health checks with custom configurations
    let postgres_check = Arc::new(PostgresHealthCheck::with_config(
        db_pool.clone(),
        HealthCheckConfig::critical()
            .with_timeout(Duration::from_secs(3))
            .with_cache_ttl(Duration::from_secs(30))
    ));

    let clickhouse_check = Arc::new(ClickHouseHealthCheck::with_config(
        clickhouse_client,
        HealthCheckConfig::non_critical()  // Non-critical, won't fail readiness
            .with_timeout(Duration::from_secs(5))
            .with_cache_ttl(Duration::from_secs(60))
    ));

    let s3_check = Arc::new(S3HealthCheck::with_config(
        s3_client,
        s3_bucket.clone(),
        HealthCheckConfig::critical()
            .with_timeout(Duration::from_secs(5))
            .with_cache_ttl(Duration::from_secs(45))
    ));

    println!("✓ Health checks configured\n");

    // 5. Create health check registry
    let version = env!("CARGO_PKG_VERSION");
    let registry = Arc::new(
        HealthCheckRegistry::new(version)
            .register(postgres_check)
            .register(clickhouse_check)
            .register(s3_check)
    );

    let health_state = HealthCheckState::new(registry.clone());

    println!("✓ Health check registry created with {} checks\n", 3);

    // 6. Set up HTTP server with health endpoints
    let app = Router::new()
        // Kubernetes liveness probe - simple check if app is running
        .route("/health/live", get(liveness_handler))
        // Kubernetes readiness probe - checks critical dependencies
        .route("/health/ready", get(readiness_handler))
        // Detailed health check - all components with diagnostics
        .route("/health", get(health_handler))
        .with_state(health_state);

    let listener = TcpListener::bind("127.0.0.1:3001").await?;
    let addr = listener.local_addr()?;

    println!("🚀 Health check server running on http://{}\n", addr);
    println!("Available endpoints:");
    println!("  • http://{}/health/live  - Liveness probe (K8s)", addr);
    println!("  • http://{}/health/ready - Readiness probe (K8s)", addr);
    println!("  • http://{}/health       - Detailed health check\n", addr);

    // 7. Spawn a task to test the endpoints
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_secs(2)).await;

        println!("\n=== Testing Health Endpoints ===\n");

        // Test liveness endpoint
        println!("1. Testing liveness endpoint...");
        let client = reqwest::Client::new();
        match client.get(format!("http://{}/health/live", addr)).send().await {
            Ok(resp) => {
                println!("   Status: {}", resp.status());
                if let Ok(body) = resp.text().await {
                    println!("   Response: {}\n", body);
                }
            }
            Err(e) => println!("   Error: {}\n", e),
        }

        // Test readiness endpoint
        println!("2. Testing readiness endpoint...");
        match client.get(format!("http://{}/health/ready", addr)).send().await {
            Ok(resp) => {
                println!("   Status: {}", resp.status());
                if let Ok(body) = resp.text().await {
                    println!("   Response: {}\n", body);
                }
            }
            Err(e) => println!("   Error: {}\n", e),
        }

        // Test detailed health endpoint
        println!("3. Testing detailed health endpoint...");
        match client.get(format!("http://{}/health", addr)).send().await {
            Ok(resp) => {
                println!("   Status: {}", resp.status());
                if let Ok(body) = resp.text().await {
                    if let Ok(json) = serde_json::from_str::<serde_json::Value>(&body) {
                        println!("   Response (formatted):\n{}\n",
                            serde_json::to_string_pretty(&json).unwrap_or(body));
                    } else {
                        println!("   Response: {}\n", body);
                    }
                }
            }
            Err(e) => println!("   Error: {}\n", e),
        }

        println!("=== Testing Complete ===\n");
        println!("Press Ctrl+C to stop the server");
    });

    // Run the server
    axum::serve(listener, app).await?;

    Ok(())
}
