//! Integration tests module for llm-research-storage
//!
//! These tests require external services (PostgreSQL, ClickHouse, S3)
//! and are skipped by default. Run with:
//!
//! ```sh
//! cargo test --test integration_tests --features integration-tests
//! ```

#[cfg(feature = "integration-tests")]
pub mod postgres_tests;

#[cfg(feature = "integration-tests")]
pub mod s3_tests;

// Re-export test utilities
pub mod test_utils;
