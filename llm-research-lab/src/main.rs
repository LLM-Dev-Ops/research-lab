use anyhow::Result;
use axum::{
    routing::get,
    Router,
};
use std::net::SocketAddr;
use tower_http::trace::TraceLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod config;
mod server;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "llm_research_lab=debug,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    tracing::info!("Starting LLM Research Lab server");

    // Load configuration
    let config = config::Config::load()?;
    tracing::info!("Configuration loaded");

    // Initialize database pool
    let db_pool = llm_research_storage::postgres::create_pool(&config.database_url).await?;
    tracing::info!("Database pool initialized");

    // Initialize S3 client
    let s3_client = llm_research_storage::s3::create_client().await?;
    tracing::info!("S3 client initialized");

    // Build application state
    let api_state = llm_research_api::AppState {
        db_pool: db_pool.clone(),
        s3_client: s3_client.clone(),
        s3_bucket: config.s3_bucket.clone(),
    };

    let app = Router::new()
        .route("/health", get(health_check))
        .nest("/api/v1", llm_research_api::routes(api_state))
        .layer(TraceLayer::new_for_http());

    // Start server
    let addr = SocketAddr::from(([0, 0, 0, 0], config.port));
    tracing::info!("Listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

async fn health_check() -> &'static str {
    "OK"
}
