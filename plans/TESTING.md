# Testing Guide for LLM Research Lab

## Quick Start

```bash
# Run all tests (unit tests only, integration tests are ignored by default)
cargo test

# Compile tests without running
cargo test --no-run

# Run tests with output
cargo test -- --nocapture

# Run tests in a specific file
cargo test --test domain_tests

# Run a specific test
cargo test test_experiment_creation
```

## Test Organization

### Directory Structure
```
llm-research-lab/
├── llm-research-core/tests/
│   ├── domain_tests.rs          # Domain model tests
│   └── config_tests.rs          # Configuration tests
├── llm-research-metrics/tests/
│   ├── calculator_tests.rs      # Metric calculator tests
│   └── statistical_tests.rs     # Statistical analysis tests
├── llm-research-workflow/tests/
│   └── pipeline_tests.rs        # Workflow pipeline tests
├── llm-research-api/tests/
│   └── integration_tests.rs     # API integration tests
└── tests/integration/
    ├── mod.rs
    └── api_tests.rs             # End-to-end integration tests
```

## Test Types

### 1. Unit Tests
Fast, isolated tests that don't require external services.

**Location**: `crate/tests/*.rs`

**Run**: `cargo test`

**Example**:
```rust
#[test]
fn test_experiment_creation() {
    let owner_id = UserId::new();
    let config = ExperimentConfig::default();

    let experiment = Experiment::new(
        "Test".to_string(),
        None,
        None,
        owner_id,
        config,
    );

    assert_eq!(experiment.status, ExperimentStatus::Draft);
}
```

### 2. Async Tests
Tests for async functions using tokio runtime.

**Decorator**: `#[tokio::test]`

**Example**:
```rust
#[tokio::test]
async fn test_accuracy_calculator() {
    let calculator = AccuracyCalculator::default();
    let input = MetricInput {
        predicted: "hello".to_string(),
        reference: Some("hello".to_string()),
    };

    let result = calculator.calculate(input).await.unwrap();
    assert_eq!(result.score, Decimal::ONE);
}
```

### 3. Integration Tests (Ignored)
Tests requiring external services (database, S3, etc.).

**Marker**: `#[ignore]`

**Run**: `cargo test -- --ignored`

**Example**:
```rust
#[tokio::test]
#[ignore]
async fn test_create_experiment_api() {
    // Requires running PostgreSQL and S3
    let response = client.post("/experiments")
        .json(&experiment_data)
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::CREATED);
}
```

## Running Integration Tests

Integration tests require external services to be running:

### Prerequisites

1. **PostgreSQL Database**
   ```bash
   docker run -d \
     --name llm-research-postgres \
     -e POSTGRES_PASSWORD=password \
     -e POSTGRES_DB=llm_research \
     -p 5432:5432 \
     postgres:15
   ```

2. **S3-Compatible Storage (MinIO)**
   ```bash
   docker run -d \
     --name llm-research-minio \
     -p 9000:9000 \
     -p 9001:9001 \
     -e MINIO_ROOT_USER=minioadmin \
     -e MINIO_ROOT_PASSWORD=minioadmin \
     minio/minio server /data --console-address ":9001"
   ```

3. **Environment Variables**
   ```bash
   export DATABASE_URL="postgresql://postgres:password@localhost/llm_research"
   export S3_ENDPOINT="http://localhost:9000"
   export S3_ACCESS_KEY="minioadmin"
   export S3_SECRET_KEY="minioadmin"
   export S3_BUCKET="llm-research"
   ```

4. **Run Migrations**
   ```bash
   sqlx migrate run
   ```

5. **Run Integration Tests**
   ```bash
   cargo test -- --ignored
   ```

## Test Coverage by Component

### Domain Models (llm-research-core)
- ✅ Experiment lifecycle
- ✅ Run lifecycle
- ✅ Status transitions
- ✅ ID types
- ✅ Semantic versioning
- ✅ Content hashing

### Configuration (llm-research-core)
- ✅ Model configuration
- ✅ Dataset references
- ✅ Metric configuration
- ✅ Parameter values
- ✅ Resource requirements
- ✅ Reproducibility settings

### Metrics (llm-research-metrics)
- ✅ Accuracy calculator
- ✅ BLEU score
- ✅ ROUGE score
- ✅ Perplexity
- ✅ Latency metrics
- ✅ Statistical analysis (t-test, Mann-Whitney U, Cohen's d)
- ✅ Confidence intervals
- ✅ Bootstrap comparison

### Workflow (llm-research-workflow)
- ✅ Pipeline construction
- ✅ DAG validation
- ✅ Topological sort
- ✅ Cycle detection
- ✅ Parallel execution
- ✅ Task dependencies

### API (llm-research-api)
- ✅ Request/response formats
- ✅ Error handling
- ✅ Pagination
- ✅ JWT validation (mocked)
- ✅ CRUD operations (mocked)
- 🔄 Full integration (requires services)

## Writing New Tests

### Test Naming Convention
```rust
// Good names - describe what is being tested
#[test]
fn test_experiment_status_transitions()

#[tokio::test]
async fn test_bleu_calculator_perfect_match()

// Avoid generic names
#[test]
fn test1()  // ❌

#[test]
fn experiment_test()  // ❌
```

### Test Structure (AAA Pattern)
```rust
#[test]
fn test_example() {
    // Arrange - Set up test data
    let owner_id = UserId::new();
    let config = ExperimentConfig::default();

    // Act - Perform the action
    let experiment = Experiment::new(
        "Test".to_string(),
        None,
        None,
        owner_id,
        config,
    );

    // Assert - Verify the result
    assert_eq!(experiment.name, "Test");
    assert_eq!(experiment.status, ExperimentStatus::Draft);
}
```

### Testing Edge Cases
```rust
#[test]
fn test_empty_input() {
    let result = process("");
    assert!(result.is_err());
}

#[test]
fn test_very_large_input() {
    let large_input = "x".repeat(1_000_000);
    let result = process(&large_input);
    assert!(result.is_ok());
}

#[test]
fn test_null_values() {
    let result = process(None);
    assert_eq!(result, default_value);
}
```

### Testing Async Functions
```rust
#[tokio::test]
async fn test_async_operation() {
    let result = async_function().await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_concurrent_operations() {
    let handles: Vec<_> = (0..10)
        .map(|i| tokio::spawn(async move {
            perform_operation(i).await
        }))
        .collect();

    for handle in handles {
        assert!(handle.await.is_ok());
    }
}
```

### Testing Error Cases
```rust
#[test]
fn test_invalid_state_transition() {
    let mut experiment = create_draft_experiment();

    // Cannot complete from Draft
    let result = experiment.complete();
    assert!(result.is_err());
}

#[tokio::test]
async fn test_validation_error() {
    let invalid_data = ExperimentConfig {
        model_configs: vec![],  // Empty should fail
        // ... other fields
    };

    let result = validate(invalid_data);
    assert!(result.is_err());
}
```

## Common Test Patterns

### Builder Pattern for Test Data
```rust
fn create_test_experiment() -> Experiment {
    Experiment::new(
        "Test Experiment".to_string(),
        Some("Description".to_string()),
        Some("Hypothesis".to_string()),
        UserId::new(),
        ExperimentConfig::default(),
    )
    .with_tags(vec!["test".to_string()])
}

#[test]
fn test_with_builder() {
    let experiment = create_test_experiment();
    assert_eq!(experiment.tags.len(), 1);
}
```

### Parameterized Tests
```rust
#[test]
fn test_version_parsing() {
    let test_cases = vec![
        ("1.2.3", Ok((1, 2, 3))),
        ("0.1.0", Ok((0, 1, 0))),
        ("invalid", Err(())),
    ];

    for (input, expected) in test_cases {
        let result = parse_version(input);
        assert_eq!(result.is_ok(), expected.is_ok());
    }
}
```

## Test Helpers

### Assertion Macros
```rust
// Basic assertions
assert!(condition);
assert_eq!(actual, expected);
assert_ne!(actual, not_expected);

// Custom error messages
assert!(condition, "Custom error message");
assert_eq!(actual, expected, "Values don't match: {} != {}", actual, expected);

// Floating point comparison
assert!((value - expected).abs() < 1e-10);
```

### Debugging Tests
```rust
#[test]
fn test_with_debug_output() {
    let value = compute_something();

    // Print debug info (only shown with --nocapture)
    println!("Debug: value = {:?}", value);

    assert_eq!(value, expected);
}
```

## Continuous Integration

Tests are automatically run on CI/CD:

```yaml
# .github/workflows/test.yml
- name: Run tests
  run: cargo test --all-features

- name: Run integration tests
  run: cargo test -- --ignored
  env:
    DATABASE_URL: ${{ secrets.DATABASE_URL }}
```

## Test Performance

### Fast Tests
- Keep unit tests fast (<1ms each)
- Avoid I/O operations
- Use mocks for external dependencies

### Slow Tests
- Mark slow tests with `#[ignore]`
- Run separately in CI
- Consider using `#[should_panic]` sparingly

## Best Practices

1. ✅ **Test one thing per test**
2. ✅ **Use descriptive test names**
3. ✅ **Follow AAA pattern** (Arrange, Act, Assert)
4. ✅ **Test edge cases and error conditions**
5. ✅ **Keep tests independent** (no shared state)
6. ✅ **Use setup helpers** for common test data
7. ✅ **Don't test implementation details**
8. ✅ **Test behavior, not structure**
9. ✅ **Mark integration tests** with `#[ignore]`
10. ✅ **Keep tests readable** and maintainable

## Troubleshooting

### Tests Won't Compile
```bash
# Check for missing dependencies
cargo check --tests

# Update dependencies
cargo update
```

### Tests Hang
```bash
# Run with timeout
cargo test -- --test-threads=1 --nocapture

# Check for deadlocks in async code
```

### Tests Fail Randomly
- Check for race conditions
- Ensure tests are independent
- Avoid shared mutable state

## Resources

- [Rust Testing Documentation](https://doc.rust-lang.org/book/ch11-00-testing.html)
- [Tokio Testing Guide](https://tokio.rs/tokio/topics/testing)
- [Test Organization Best Practices](https://rust-lang.github.io/api-guidelines/documentation.html)
