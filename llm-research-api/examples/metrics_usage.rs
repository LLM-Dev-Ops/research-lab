//! Example demonstrating the comprehensive Prometheus metrics system
//!
//! This example shows how to:
//! - Initialize the metrics system
//! - Set up the metrics endpoint
//! - Use automatic HTTP metrics middleware
//! - Record custom business metrics
//! - Use helper functions for common patterns
//!
//! Run with:
//! ```bash
//! cargo run --example metrics_usage
//! ```

use axum::{
    routing::get,
    Router,
};
use llm_research_api::{
    init_metrics, metrics_handler, MetricsLayer, BusinessMetrics, DatabaseMetrics,
    SystemMetrics, observe_duration, increment_counter,
};
use std::time::Duration;
use tower::ServiceBuilder;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Prometheus Metrics System Example ===\n");

    // 1. Initialize the metrics system
    println!("1. Initializing metrics system...");
    init_metrics()?;
    println!("   ✓ Metrics system initialized\n");

    // 2. Record process start time
    println!("2. Recording system metrics...");
    SystemMetrics::record_start_time();
    SystemMetrics::update_all();
    println!("   ✓ System metrics recorded\n");

    // 3. Record some business metrics
    println!("3. Recording business metrics...");
    BusinessMetrics::experiment_created();
    BusinessMetrics::model_registered();
    BusinessMetrics::dataset_uploaded(1_000_000);
    BusinessMetrics::experiment_duration(Duration::from_secs(60));
    println!("   ✓ Business metrics recorded\n");

    // 4. Record database metrics
    println!("4. Recording database metrics...");
    DatabaseMetrics::record_query("select", "experiments", Duration::from_millis(15));
    DatabaseMetrics::set_active_connections(5);
    println!("   ✓ Database metrics recorded\n");

    // 5. Use helper functions
    println!("5. Using helper functions...");
    increment_counter("custom_counter", &[("type", "example")]);
    observe_duration(
        "custom_duration",
        Duration::from_millis(100),
        &[("operation", "example")],
    );
    println!("   ✓ Custom metrics recorded\n");

    // 6. Create router with metrics endpoint and middleware
    println!("6. Setting up HTTP server with metrics...");
    let app = Router::new()
        .route("/", get(|| async { "Hello, World!" }))
        .route("/api/data", get(|| async { "Some data" }))
        .route("/metrics", get(metrics_handler))
        .layer(ServiceBuilder::new().layer(MetricsLayer::default()));

    println!("   ✓ Router configured with metrics middleware\n");

    // 7. Start the server
    println!("=== Server Starting ===");
    println!("Metrics endpoint: http://localhost:3000/metrics");
    println!("API endpoint:     http://localhost:3000/api/data");
    println!("Health endpoint:  http://localhost:3000/");
    println!("\nPress Ctrl+C to stop\n");

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000").await?;
    axum::serve(listener, app).await?;

    Ok(())
}
