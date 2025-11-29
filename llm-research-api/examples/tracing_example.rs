//! Example demonstrating OpenTelemetry distributed tracing
//!
//! This example shows how to:
//! 1. Initialize OpenTelemetry tracing
//! 2. Create custom spans
//! 3. Add tracing middleware to Axum
//! 4. Trace database queries
//! 5. Propagate trace context

use axum::{routing::get, Router};
use llm_research_api::observability::tracing::{
    create_span, init_tracing, record_error, record_event, tracing_middleware, DbSpan,
    TracingConfig,
};
use opentelemetry::trace::SpanKind;
use std::error::Error;
use tracing::{info, instrument, Instrument};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // Configure tracing
    let config = TracingConfig::builder()
        .service_name("llm-research-api-example")
        .service_version("1.0.0")
        .environment("development")
        .otlp_endpoint("http://localhost:4317")
        .sampling_rate(1.0) // Sample all traces in development
        .enable_console_logging(true)
        .resource_attribute("deployment.region", "us-west-2")
        .resource_attribute("k8s.cluster", "dev-cluster")
        .build();

    // Initialize tracing
    init_tracing(config).await?;

    info!("OpenTelemetry tracing initialized");

    // Example 1: Manual span creation
    example_manual_spans().await;

    // Example 2: Database tracing
    example_database_tracing().await;

    // Example 3: HTTP middleware (commented out as it requires a server)
    // example_http_server().await?;

    Ok(())
}

/// Example of manual span creation with custom attributes
async fn example_manual_spans() {
    let span = create_span("process_experiment")
        .kind(SpanKind::Internal)
        .user_id("user-123")
        .experiment_id("exp-456")
        .model_id("model-789")
        .attribute("batch_size", "32")
        .create();

    async {
        info!("Processing experiment with custom attributes");

        // Record an event
        record_event("experiment_started", vec![("reason", "scheduled")]);

        // Simulate work
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        info!("Experiment processing complete");
    }
    .instrument(span)
    .await;
}

/// Example of database query tracing
async fn example_database_tracing() {
    let db_span = DbSpan::new(
        "SELECT",
        "SELECT * FROM experiments WHERE user_id = 123 AND status = 'active'",
    )
    .sanitize(true);

    // Simulate a database query
    let result = db_span
        .execute(async {
            tokio::time::sleep(tokio::time::Duration::from_millis(5)).await;
            Ok::<Vec<String>, sqlx::Error>(vec!["exp-1".to_string(), "exp-2".to_string()])
        })
        .await;

    match result {
        Ok(experiments) => {
            info!(count = experiments.len(), "Found experiments");
        }
        Err(e) => {
            record_error(&e);
        }
    }
}

/// Example of HTTP server with tracing middleware
#[allow(dead_code)]
async fn example_http_server() -> Result<(), Box<dyn Error>> {
    let app = Router::new()
        .route("/health", get(health_handler))
        .route("/experiments", get(list_experiments))
        .layer(axum::middleware::from_fn(tracing_middleware));

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000").await?;
    info!("Server listening on 127.0.0.1:3000");

    axum::serve(listener, app).await?;

    Ok(())
}

async fn health_handler() -> &'static str {
    "OK"
}

#[instrument(name = "list_experiments", skip_all)]
async fn list_experiments() -> String {
    info!("Listing experiments");

    // Create a child span for database query
    let span = create_span("query_experiments")
        .kind(SpanKind::Client)
        .attribute("db.system", "postgresql")
        .create();

    async {
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
        "[]".to_string()
    }
    .instrument(span)
    .await
}
