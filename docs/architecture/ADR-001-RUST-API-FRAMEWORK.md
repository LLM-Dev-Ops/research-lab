# ADR-001: Rust with Axum for API Framework

## Status
Accepted

## Date
2025-01-15

## Context

The LLM Research Lab platform requires a high-performance, reliable API layer to handle:
- Experiment management and execution
- Dataset storage and retrieval
- Real-time metrics collection
- High-throughput model inference coordination

Key requirements:
- Low latency for API responses (< 100ms P95)
- High concurrency (thousands of concurrent connections)
- Memory safety and reliability
- Strong type system for complex domain models
- Excellent async/await support for I/O-bound operations

## Decision

We chose **Rust** as the primary programming language with **Axum** as the web framework.

### Rationale

**Language: Rust**
1. **Memory Safety**: Zero-cost abstractions with compile-time memory safety guarantees
2. **Performance**: Near-C performance with predictable latency (no garbage collection pauses)
3. **Concurrency**: First-class async/await support with Tokio runtime
4. **Type System**: Strong static typing catches errors at compile time
5. **Reliability**: Pattern matching and Result types enforce error handling
6. **Ecosystem**: Mature crates for databases (sqlx), HTTP (reqwest), serialization (serde)

**Framework: Axum**
1. **Tower Integration**: Built on Tower middleware ecosystem for composable services
2. **Type Safety**: Compile-time route extraction and validation
3. **Performance**: One of the fastest Rust web frameworks
4. **Ergonomics**: Clean API with extractors for request parsing
5. **Active Development**: Maintained by Tokio team, regular updates
6. **Ecosystem Compatibility**: Works with tower-http, tracing, and other standard crates

### Alternatives Considered

| Framework | Pros | Cons | Decision |
|-----------|------|------|----------|
| **Actix-web** | Mature, fast, feature-rich | Complex actor model, steeper learning curve | Not chosen |
| **Rocket** | Developer ergonomics, macros | Slower, requires nightly Rust | Not chosen |
| **Warp** | Composable filters | Less intuitive, verbose | Not chosen |
| **Node.js/Express** | Large ecosystem, fast dev | GC pauses, type safety concerns | Not chosen |
| **Go/Gin** | Simple, good concurrency | Less expressive type system | Not chosen |

## Consequences

### Positive
- **Performance**: Sub-millisecond overhead per request
- **Reliability**: Memory bugs caught at compile time
- **Maintainability**: Strong types document the code
- **Scalability**: Efficient resource usage allows fewer instances
- **Security**: Memory safety prevents entire classes of vulnerabilities

### Negative
- **Learning Curve**: Rust has a steeper learning curve than Go or Python
- **Compile Times**: Longer builds compared to interpreted languages
- **Smaller Talent Pool**: Fewer Rust developers in the market
- **Development Speed**: Initial development slower due to compiler strictness

### Mitigations
- Comprehensive documentation and onboarding guides
- Use of `cargo watch` for faster iteration cycles
- Investment in team training and pair programming
- Strategic use of dynamic typing where appropriate (serde_json::Value)

## Implementation Notes

### Project Structure
```
llm-research-api/
├── src/
│   ├── lib.rs           # Public API exports
│   ├── handlers/        # Request handlers
│   ├── dto/             # Data transfer objects
│   ├── middleware/      # Custom middleware
│   ├── error.rs         # Error types
│   └── security/        # Auth, rate limiting
├── tests/
│   ├── integration_tests.rs
│   └── dto_tests.rs
└── Cargo.toml
```

### Key Dependencies
```toml
[dependencies]
axum = "0.7"
tokio = { version = "1", features = ["full"] }
sqlx = { version = "0.8", features = ["postgres", "runtime-tokio-native-tls"] }
serde = { version = "1", features = ["derive"] }
tower = "0.4"
tower-http = "0.5"
tracing = "0.1"
```

### Example Handler
```rust
pub async fn create_experiment(
    State(state): State<AppState>,
    ValidatedJson(req): ValidatedJson<CreateExperimentRequest>,
) -> Result<Json<ExperimentResponse>, ApiError> {
    let experiment = state.experiment_service
        .create(req.into())
        .await?;

    Ok(Json(experiment.into()))
}
```

## References
- [Axum Documentation](https://docs.rs/axum)
- [Tokio Runtime](https://tokio.rs)
- [Rust Performance Book](https://nnethercote.github.io/perf-book/)
- [Tower Middleware](https://docs.rs/tower)
