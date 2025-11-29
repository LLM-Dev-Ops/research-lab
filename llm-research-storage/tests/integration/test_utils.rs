//! Test utilities for integration tests
//!
//! Provides helpers for setting up test containers and test data.

use llm_research_core::domain::{
    config::ExperimentConfig,
    experiment::Experiment,
    ids::UserId,
};
use uuid::Uuid;

/// Create a test experiment with default configuration
pub fn create_test_experiment(name: &str) -> Experiment {
    let owner_id = UserId::from(Uuid::new_v4());
    let config = ExperimentConfig::default();

    Experiment::new(
        name.to_string(),
        Some(format!("Test experiment: {}", name)),
        Some("Testing hypothesis".to_string()),
        owner_id,
        config,
    )
}

/// Generate a unique test name to avoid conflicts
pub fn unique_test_name(prefix: &str) -> String {
    format!("{}_{}", prefix, Uuid::new_v4().to_string().split('-').next().unwrap())
}

#[cfg(feature = "integration-tests")]
pub mod containers {
    use testcontainers::{clients::Cli, Container, Image};
    use testcontainers_modules::postgres::Postgres;

    /// Start a PostgreSQL container for testing
    pub async fn start_postgres(docker: &Cli) -> Container<'_, Postgres> {
        docker.run(Postgres::default())
    }

    /// Get the connection string for a PostgreSQL container
    pub fn postgres_connection_string(container: &Container<'_, Postgres>) -> String {
        let port = container.get_host_port_ipv4(5432);
        format!(
            "postgres://postgres:postgres@127.0.0.1:{}/postgres",
            port
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_test_experiment() {
        let experiment = create_test_experiment("Test");
        assert_eq!(experiment.name, "Test");
        assert!(experiment.description.is_some());
    }

    #[test]
    fn test_unique_test_name() {
        let name1 = unique_test_name("test");
        let name2 = unique_test_name("test");
        assert_ne!(name1, name2);
        assert!(name1.starts_with("test_"));
    }
}
