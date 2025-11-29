use anyhow::Result;
use clickhouse::Client;
use serde::{Deserialize, Serialize};

/// ClickHouse configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClickHouseConfig {
    pub url: String,
    pub database: String,
    pub username: Option<String>,
    pub password: Option<String>,
}

impl Default for ClickHouseConfig {
    fn default() -> Self {
        Self {
            url: "http://localhost:8123".to_string(),
            database: "llm_research".to_string(),
            username: None,
            password: None,
        }
    }
}

/// Create a ClickHouse client with the given configuration
pub async fn create_client(config: &ClickHouseConfig) -> Result<Client> {
    let mut client = Client::default()
        .with_url(&config.url)
        .with_database(&config.database);

    if let Some(username) = &config.username {
        client = client.with_user(username);
    }

    if let Some(password) = &config.password {
        client = client.with_password(password);
    }

    tracing::info!(
        "ClickHouse client created for database: {}",
        config.database
    );
    Ok(client)
}

/// Check if ClickHouse is healthy and accessible
pub async fn health_check(client: &Client) -> Result<bool> {
    match client.query("SELECT 1").fetch_one::<u8>().await {
        Ok(_) => {
            tracing::info!("ClickHouse health check passed");
            Ok(true)
        }
        Err(e) => {
            tracing::error!("ClickHouse health check failed: {}", e);
            Err(e.into())
        }
    }
}

pub async fn create_tables(client: &Client) -> Result<()> {
    // Create evaluations time-series table
    client
        .query(
            r#"
            CREATE TABLE IF NOT EXISTS evaluations (
                id UUID,
                experiment_id UUID,
                sample_id UUID,
                timestamp DateTime64(3),
                latency_ms Int64,
                token_count Int32,
                cost Decimal64(8),
                metrics String,
                INDEX idx_experiment experiment_id TYPE minmax GRANULARITY 3,
                INDEX idx_timestamp timestamp TYPE minmax GRANULARITY 3
            )
            ENGINE = MergeTree()
            PARTITION BY toYYYYMM(timestamp)
            ORDER BY (experiment_id, timestamp)
            "#,
        )
        .execute()
        .await?;

    tracing::info!("ClickHouse tables created");
    Ok(())
}
