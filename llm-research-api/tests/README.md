# LLM Research API Test Suite

This directory contains comprehensive unit and integration tests for the `llm-research-api` crate.

## Test Files

### 1. handler_tests.rs
Tests for all API route handlers including request validation, response formatting, and HTTP status codes.

**Coverage:**
- **Experiment Handlers**: create, list, get, update, delete, start
- **Run Handlers**: create, list, complete, fail
- **Model Handlers**: create, list, get, update, delete, list_providers
- **Dataset Handlers**: create, list, get, update, delete, create_version, list_versions, upload, download
- **Prompt Template Handlers**: create, list, get, update, delete
- **Evaluation Handlers**: create, list, get, get_metrics
- **Health Check**: basic health endpoint
- **HTTP Methods**: method validation
- **Content Types**: content type handling

**Test Count**: 40+ tests

**Key Test Patterns:**
```rust
// Testing success paths
#[tokio::test]
async fn test_create_experiment_success() { ... }

// Testing validation errors
#[tokio::test]
async fn test_create_experiment_validation_error() { ... }

// Testing not found errors
#[tokio::test]
async fn test_get_experiment_not_found() { ... }
```

### 2. middleware_tests.rs
Tests for authentication, authorization, and request processing middleware.

**Coverage:**
- **Authentication Middleware**: valid token, missing header, invalid token, expired token
- **Optional Authentication**: with/without token, invalid token handling
- **JWT Claims**: serialization, deserialization, validation
- **Role-Based Authorization**: has_role, has_any_role helpers
- **Token Validation**: edge cases, format validation
- **Error Responses**: proper error formatting

**Test Count**: 35+ tests

**Key Test Patterns:**
```rust
// Testing auth middleware
#[tokio::test]
async fn test_auth_middleware_with_valid_token() { ... }

// Testing role checks
#[test]
fn test_has_role_positive() { ... }

// Testing JWT validation
#[tokio::test]
async fn test_auth_middleware_expired_token() { ... }
```

### 3. dto_tests.rs
Tests for Data Transfer Objects including validation, serialization, and conversions.

**Coverage:**
- **Experiment DTOs**: CreateExperimentRequest, UpdateExperimentRequest, ExperimentResponse
- **Run DTOs**: CreateRunRequest, RunResponse, FailRunRequest
- **Model DTOs**: CreateModelRequest, UpdateModelRequest, ModelResponse, ProviderResponse
- **Dataset DTOs**: CreateDatasetRequest, UpdateDatasetRequest, DatasetResponse, versions
- **Prompt DTOs**: CreatePromptTemplateRequest, UpdatePromptTemplateRequest, PromptTemplateResponse
- **Evaluation DTOs**: CreateEvaluationRequest, EvaluationResponse, MetricsResponse
- **Pagination DTOs**: PaginationQuery, PaginatedResponse
- **Error DTOs**: ErrorResponse
- **Domain Conversions**: DTO to/from domain model conversions

**Test Count**: 55+ tests

**Key Test Patterns:**
```rust
// Testing validation
#[test]
fn test_create_experiment_request_validation_success() { ... }

// Testing serialization
#[test]
fn test_experiment_response_serialization() { ... }

// Testing conversions
#[test]
fn test_experiment_response_from_domain() { ... }
```

### 4. integration_tests.rs
Existing integration tests for end-to-end API testing (placeholder tests currently).

## Running Tests

### Run All Tests
```bash
cargo test --package llm-research-api
```

### Run Specific Test File
```bash
cargo test --package llm-research-api --test handler_tests
cargo test --package llm-research-api --test middleware_tests
cargo test --package llm-research-api --test dto_tests
```

### Run Specific Test
```bash
cargo test --package llm-research-api test_create_experiment_success
```

### Run with Output
```bash
cargo test --package llm-research-api -- --nocapture
```

### Run Tests in Parallel
```bash
cargo test --package llm-research-api -- --test-threads=4
```

## Test Dependencies

The following dev dependencies are required:

- `tokio` (with macros feature) - async test runtime
- `tower` (with util feature) - test utilities for Axum
- `axum-test` - Axum testing helpers
- `http-body-util` - HTTP body utilities
- `hyper` - HTTP testing
- `base64` - JWT token validation
- `serde_json` - JSON serialization
- `validator` - request validation
- `uuid` - ID generation
- `chrono` - timestamps
- `rust_decimal` - decimal precision

## Test Structure

### Handler Tests
Handler tests use the `oneshot` pattern to test Axum routes without starting a server:

```rust
let state = create_mock_app_state();
let app = llm_research_api::routes(state);

let request = Request::builder()
    .uri("/experiments")
    .method("POST")
    .header(header::CONTENT_TYPE, "application/json")
    .body(Body::from(serde_json::to_string(&request_body).unwrap()))
    .unwrap();

let response = app.oneshot(request).await.unwrap();
assert_eq!(response.status(), StatusCode::CREATED);
```

### Middleware Tests
Middleware tests create test routers with middleware layers:

```rust
fn create_test_app_with_auth(state: AppState) -> Router {
    Router::new()
        .route("/protected", axum::routing::get(handler))
        .layer(middleware::from_fn_with_state(state.clone(), auth_middleware))
        .with_state(state)
}
```

### DTO Tests
DTO tests focus on validation and serialization:

```rust
let request = CreateExperimentRequest { ... };

// Test validation
assert!(request.validate().is_ok());

// Test serialization
let json = serde_json::to_string(&request).unwrap();

// Test deserialization
let parsed: CreateExperimentRequest = serde_json::from_str(&json).unwrap();
```

## Test Coverage Areas

### Success Paths ✓
- Valid requests with proper data
- Correct HTTP status codes (200, 201, 204)
- Proper response formatting
- Authentication with valid tokens

### Error Paths ✓
- Validation errors (400)
- Not found errors (404)
- Unauthorized errors (401)
- Invalid tokens
- Missing required fields
- Field length constraints

### Edge Cases ✓
- Empty strings
- Maximum length values
- Null/None optional fields
- Multiple roles
- Expired tokens
- Malformed tokens
- Case sensitivity

### Request Validation ✓
- Name length constraints (1-255 chars)
- Required fields
- Optional fields
- Nested object validation
- Array validation

### Response Formatting ✓
- JSON serialization
- UUID handling
- Timestamp formatting
- Decimal precision
- Pagination metadata

## Mock Dependencies

Tests use mock implementations for external dependencies:

- **Database**: Lazy PgPool connection (not actually used in unit tests)
- **S3 Client**: Mock S3 client with dummy credentials
- **JWT Secrets**: Environment variable or default "your-secret-key"

## Future Enhancements

1. **Database Integration Tests**: Add tests with test database
2. **S3 Integration Tests**: Add tests with MinIO or LocalStack
3. **Request Context Tests**: Test request extensions and user context
4. **Performance Tests**: Add benchmarks for critical paths
5. **Property-Based Tests**: Add quickcheck/proptest for edge cases
6. **Snapshot Tests**: Add insta for response snapshot testing

## Best Practices

1. **Use descriptive test names**: `test_create_experiment_validation_error`
2. **Test one thing per test**: Keep tests focused and simple
3. **Use helper functions**: Create test data generators
4. **Clean up**: Ensure no side effects between tests
5. **Test both success and failure**: Cover happy and sad paths
6. **Document complex tests**: Add comments for non-obvious test logic
7. **Keep tests fast**: Mock external dependencies
8. **Use async tests**: Mark async tests with `#[tokio::test]`

## Troubleshooting

### Tests not compiling?
- Ensure all dependencies are in Cargo.toml
- Check that workspace dependencies are properly configured
- Verify feature flags are enabled

### Tests failing?
- Check mock data matches domain model requirements
- Verify validation rules in DTOs
- Ensure test database is available (for integration tests)

### Async tests hanging?
- Ensure `#[tokio::test]` is used for async tests
- Check that all futures are properly awaited
- Verify no deadlocks in middleware chains

## Contributing

When adding new API endpoints:
1. Add handler tests in `handler_tests.rs`
2. Add middleware tests if new middleware is introduced
3. Add DTO tests for new request/response types
4. Update this README with new test coverage

When fixing bugs:
1. Add a failing test that reproduces the bug
2. Fix the bug
3. Verify the test now passes
4. Add regression test to prevent future issues
