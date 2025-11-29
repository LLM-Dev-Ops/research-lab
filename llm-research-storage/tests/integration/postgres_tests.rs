//! PostgreSQL integration tests
//!
//! These tests require a PostgreSQL container and are run with:
//! ```sh
//! cargo test --test integration_tests --features integration-tests
//! ```

#![cfg(feature = "integration-tests")]

use llm_research_core::domain::{
    config::ExperimentConfig,
    experiment::{Experiment, ExperimentStatus},
    ids::UserId,
};
use llm_research_storage::postgres::create_pool;
use serial_test::serial;
use sqlx::PgPool;
use testcontainers::{clients::Cli, Container};
use testcontainers_modules::postgres::Postgres;
use uuid::Uuid;

use super::test_utils::{create_test_experiment, unique_test_name};

/// Start PostgreSQL container and return pool
async fn setup_postgres() -> (Cli, Container<'_, Postgres>, PgPool) {
    let docker = Cli::default();
    let container = docker.run(Postgres::default());
    let port = container.get_host_port_ipv4(5432);
    let connection_string = format!(
        "postgres://postgres:postgres@127.0.0.1:{}/postgres",
        port
    );

    let pool = create_pool(&connection_string).await.expect("Failed to create pool");

    // Run migrations
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("Failed to run migrations");

    (docker, container, pool)
}

#[tokio::test]
#[serial]
async fn test_postgres_connection() {
    let (_docker, _container, pool) = setup_postgres().await;

    // Test connection
    let result: (i64,) = sqlx::query_as("SELECT 1")
        .fetch_one(&pool)
        .await
        .expect("Failed to query");

    assert_eq!(result.0, 1);
}

#[tokio::test]
#[serial]
async fn test_experiment_crud() {
    let (_docker, _container, pool) = setup_postgres().await;

    let experiment = create_test_experiment(&unique_test_name("crud_test"));

    // Insert experiment
    let id = experiment.id;
    sqlx::query(
        r#"
        INSERT INTO experiments (id, name, description, hypothesis, owner_id, status, config, created_at, updated_at)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
        "#,
    )
    .bind(id.as_uuid())
    .bind(&experiment.name)
    .bind(&experiment.description)
    .bind(&experiment.hypothesis)
    .bind(experiment.owner_id.as_uuid())
    .bind("draft")
    .bind(serde_json::to_value(&experiment.config).unwrap())
    .bind(experiment.created_at)
    .bind(experiment.updated_at)
    .execute(&pool)
    .await
    .expect("Failed to insert experiment");

    // Read experiment
    let result: (String, String) = sqlx::query_as(
        r#"SELECT name, status FROM experiments WHERE id = $1"#,
    )
    .bind(id.as_uuid())
    .fetch_one(&pool)
    .await
    .expect("Failed to read experiment");

    assert_eq!(result.0, experiment.name);
    assert_eq!(result.1, "draft");

    // Update experiment
    sqlx::query("UPDATE experiments SET status = $1 WHERE id = $2")
        .bind("active")
        .bind(id.as_uuid())
        .execute(&pool)
        .await
        .expect("Failed to update experiment");

    // Verify update
    let result: (String,) = sqlx::query_as(
        r#"SELECT status FROM experiments WHERE id = $1"#,
    )
    .bind(id.as_uuid())
    .fetch_one(&pool)
    .await
    .expect("Failed to read updated experiment");

    assert_eq!(result.0, "active");

    // Delete experiment
    sqlx::query("DELETE FROM experiments WHERE id = $1")
        .bind(id.as_uuid())
        .execute(&pool)
        .await
        .expect("Failed to delete experiment");

    // Verify deletion
    let result: Option<(Uuid,)> = sqlx::query_as(
        r#"SELECT id FROM experiments WHERE id = $1"#,
    )
    .bind(id.as_uuid())
    .fetch_optional(&pool)
    .await
    .expect("Failed to check deletion");

    assert!(result.is_none());
}

#[tokio::test]
#[serial]
async fn test_experiment_listing() {
    let (_docker, _container, pool) = setup_postgres().await;

    // Create multiple experiments
    for i in 0..5 {
        let experiment = create_test_experiment(&unique_test_name(&format!("list_test_{}", i)));

        sqlx::query(
            r#"
            INSERT INTO experiments (id, name, description, hypothesis, owner_id, status, config, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
            "#,
        )
        .bind(experiment.id.as_uuid())
        .bind(&experiment.name)
        .bind(&experiment.description)
        .bind(&experiment.hypothesis)
        .bind(experiment.owner_id.as_uuid())
        .bind("draft")
        .bind(serde_json::to_value(&experiment.config).unwrap())
        .bind(experiment.created_at)
        .bind(experiment.updated_at)
        .execute(&pool)
        .await
        .expect("Failed to insert experiment");
    }

    // List experiments with pagination
    let results: Vec<(Uuid, String)> = sqlx::query_as(
        r#"SELECT id, name FROM experiments ORDER BY created_at DESC LIMIT 3"#,
    )
    .fetch_all(&pool)
    .await
    .expect("Failed to list experiments");

    assert_eq!(results.len(), 3);
}

#[tokio::test]
#[serial]
async fn test_experiment_search() {
    let (_docker, _container, pool) = setup_postgres().await;

    // Create experiments with specific names
    let names = vec!["Alpha Model Test", "Beta Testing", "Gamma Analysis"];

    for name in names {
        let experiment = create_test_experiment(name);

        sqlx::query(
            r#"
            INSERT INTO experiments (id, name, description, hypothesis, owner_id, status, config, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
            "#,
        )
        .bind(experiment.id.as_uuid())
        .bind(&experiment.name)
        .bind(&experiment.description)
        .bind(&experiment.hypothesis)
        .bind(experiment.owner_id.as_uuid())
        .bind("draft")
        .bind(serde_json::to_value(&experiment.config).unwrap())
        .bind(experiment.created_at)
        .bind(experiment.updated_at)
        .execute(&pool)
        .await
        .expect("Failed to insert experiment");
    }

    // Search by name pattern
    let results: Vec<(String,)> = sqlx::query_as(
        r#"SELECT name FROM experiments WHERE name ILIKE $1"#,
    )
    .bind("%test%")
    .fetch_all(&pool)
    .await
    .expect("Failed to search experiments");

    assert_eq!(results.len(), 2); // "Alpha Model Test" and "Beta Testing"
}

#[tokio::test]
#[serial]
async fn test_transaction_rollback() {
    let (_docker, _container, pool) = setup_postgres().await;

    let experiment = create_test_experiment(&unique_test_name("rollback_test"));

    // Start transaction
    let mut tx = pool.begin().await.expect("Failed to start transaction");

    // Insert in transaction
    sqlx::query(
        r#"
        INSERT INTO experiments (id, name, description, hypothesis, owner_id, status, config, created_at, updated_at)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
        "#,
    )
    .bind(experiment.id.as_uuid())
    .bind(&experiment.name)
    .bind(&experiment.description)
    .bind(&experiment.hypothesis)
    .bind(experiment.owner_id.as_uuid())
    .bind("draft")
    .bind(serde_json::to_value(&experiment.config).unwrap())
    .bind(experiment.created_at)
    .bind(experiment.updated_at)
    .execute(&mut *tx)
    .await
    .expect("Failed to insert in transaction");

    // Rollback
    tx.rollback().await.expect("Failed to rollback");

    // Verify not committed
    let result: Option<(Uuid,)> = sqlx::query_as(
        r#"SELECT id FROM experiments WHERE id = $1"#,
    )
    .bind(experiment.id.as_uuid())
    .fetch_optional(&pool)
    .await
    .expect("Failed to check rollback");

    assert!(result.is_none());
}
