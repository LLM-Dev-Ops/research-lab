# LLM Research Lab - Rust Implementation

An enterprise-grade experimental evaluation platform for Large Language Models (LLMs) built with Rust.

## Architecture

This project uses a Cargo workspace with 6 crates organized in a clean, modular architecture:

### Crates

1. **llm-research-lab** - Main binary crate
   - Axum-based HTTP server
   - Application configuration
   - Server initialization and orchestration

2. **llm-research-core** - Core domain models and business logic
   - Domain entities: Experiment, Model, Dataset, PromptTemplate, Evaluation
   - Business logic and validation
   - Core traits and interfaces
   - Error types

3. **llm-research-api** - REST API layer
   - HTTP handlers for all resources
   - Request/response DTOs
   - Input validation
   - Error handling middleware

4. **llm-research-storage** - Storage implementations
   - PostgreSQL repository implementations
   - ClickHouse time-series storage
   - S3 object storage
   - Database migrations

5. **llm-research-metrics** - Metric computation and statistical analysis
   - Accuracy, BLEU, ROUGE, perplexity calculators
   - Statistical aggregations (mean, median, percentiles)
   - Statistical tests (t-test, Cohen's d)
   - Confidence intervals

6. **llm-research-workflow** - Workflow engine for experiment pipelines
   - Workflow orchestration
   - Pipeline execution (sequential and parallel)
   - Task management
   - Experiment lifecycle management

## Technology Stack

- **Web Framework**: Axum 0.7 with Tower middleware
- **Async Runtime**: Tokio 1.41
- **Databases**:
  - PostgreSQL via SQLx 0.8
  - ClickHouse 0.12 for time-series metrics
- **Object Storage**: AWS S3 SDK 1.64
- **Serialization**: Serde 1.0 with JSON support
- **Configuration**: config 0.14 with environment variables
- **Observability**: tracing + tracing-subscriber
- **Error Handling**: thiserror + anyhow
- **Authentication**: jsonwebtoken 9.3
- **Statistics**: statrs 0.18

## Project Structure

```
llm-research-lab/
├── Cargo.toml                      # Workspace configuration
├── .cargo/
│   └── config.toml                 # Cargo build settings
├── config/
│   └── default.toml                # Default configuration
├── llm-research-lab/               # Main binary
│   ├── Cargo.toml
│   └── src/
│       ├── main.rs                 # Server entry point
│       ├── config.rs               # Configuration loading
│       └── server.rs               # Application state
├── llm-research-core/              # Core domain
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs
│       ├── error.rs                # Error types
│       ├── traits.rs               # Core traits
│       └── domain/
│           ├── experiment.rs
│           ├── model.rs
│           ├── evaluation.rs
│           ├── prompt.rs
│           └── dataset.rs
├── llm-research-api/               # API layer
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs
│       ├── error.rs                # API error handling
│       ├── dto/                    # Request/response DTOs
│       ├── handlers/               # HTTP handlers
│       └── middleware/             # Auth, logging, etc.
├── llm-research-storage/           # Storage layer
│   ├── Cargo.toml
│   ├── migrations/                 # SQL migrations
│   └── src/
│       ├── lib.rs
│       ├── postgres.rs             # PostgreSQL client
│       ├── clickhouse.rs           # ClickHouse client
│       ├── s3.rs                   # S3 client
│       └── repositories/           # Data access objects
├── llm-research-metrics/           # Metrics
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs
│       ├── calculators/            # Metric calculators
│       ├── aggregators.rs          # Statistical aggregations
│       └── statistical.rs          # Statistical tests
└── llm-research-workflow/          # Workflow engine
    ├── Cargo.toml
    └── src/
        ├── lib.rs
        ├── engine.rs               # Workflow engine
        ├── pipeline.rs             # Pipeline executor
        ├── executor.rs             # Task executor
        └── tasks/                  # Task implementations
```

## Getting Started

### Prerequisites

- Rust 1.91+ (install from https://rustup.rs/)
- PostgreSQL 14+
- ClickHouse 23+
- AWS S3 or compatible object storage

### Building

```bash
# Check all crates compile
cargo check --workspace

# Build in debug mode
cargo build --workspace

# Build optimized release
cargo build --workspace --release
```

### Running

```bash
# Run the server (development)
cargo run

# Run with custom config
LLM_RESEARCH_PORT=8080 cargo run
```

### Configuration

Configuration can be set via:
1. `config/default.toml` - Default settings
2. `config/local.toml` - Local overrides (gitignored)
3. Environment variables with `LLM_RESEARCH_` prefix

Example environment variables:
```bash
export LLM_RESEARCH_PORT=3000
export LLM_RESEARCH_DATABASE_URL="postgres://user:pass@localhost/llm_research"
export LLM_RESEARCH_CLICKHOUSE_URL="http://localhost:8123"
export LLM_RESEARCH_S3_BUCKET="llm-research-artifacts"
export LLM_RESEARCH_LOG_LEVEL="info"
```

## API Endpoints

### Health Check
- `GET /health` - Server health status

### Experiments
- `POST /api/v1/experiments` - Create experiment
- `GET /api/v1/experiments` - List experiments
- `GET /api/v1/experiments/:id` - Get experiment details
- `PUT /api/v1/experiments/:id` - Update experiment
- `DELETE /api/v1/experiments/:id` - Delete experiment
- `POST /api/v1/experiments/:id/start` - Start experiment
- `POST /api/v1/experiments/:id/stop` - Stop experiment

### Models
- `POST /api/v1/models` - Register model
- `GET /api/v1/models` - List models
- `GET /api/v1/models/:id` - Get model details

### Datasets
- `POST /api/v1/datasets` - Create dataset
- `GET /api/v1/datasets` - List datasets
- `GET /api/v1/datasets/:id` - Get dataset details

### Prompt Templates
- `POST /api/v1/prompts` - Create prompt template
- `GET /api/v1/prompts` - List prompts
- `GET /api/v1/prompts/:id` - Get prompt details

### Evaluations
- `GET /api/v1/evaluations` - List evaluations
- `GET /api/v1/evaluations/:id` - Get evaluation details

## Development

### Running Tests

```bash
# Run all tests
cargo test --workspace

# Run tests for specific crate
cargo test -p llm-research-core

# Run with output
cargo test --workspace -- --nocapture
```

### Code Quality

```bash
# Format code
cargo fmt --all

# Lint with clippy
cargo clippy --workspace -- -D warnings

# Check for common mistakes
cargo clippy --workspace --all-targets
```

### Database Migrations

```bash
# Install sqlx-cli
cargo install sqlx-cli --no-default-features --features postgres

# Run migrations
sqlx migrate run --database-url $DATABASE_URL

# Create new migration
sqlx migrate add create_experiments_table
```

## Performance

The project is configured for optimal performance:

- **Release builds**: LTO enabled, single codegen unit
- **Async runtime**: Tokio with full feature set
- **Database**: Connection pooling with SQLx
- **Caching**: Planned for frequent queries
- **Observability**: Structured logging with tracing

### Benchmarking

```bash
# Build with optimizations
cargo build --release --workspace

# Run with profiling (requires perf)
cargo build --profile=bench
```

## License

See LICENSE.md for details.

## Contributing

This is an experimental research platform. Contributions should focus on:
- Robust error handling
- Comprehensive testing
- Performance optimization
- Clean, maintainable code

## Roadmap

### Phase 1 - Foundation (Current)
- [x] Project structure and workspace setup
- [x] Core domain models
- [x] Basic API endpoints
- [x] Storage layer interfaces
- [x] Metric calculators
- [x] Workflow engine

### Phase 2 - Implementation
- [ ] Complete repository implementations
- [ ] Database schema and migrations
- [ ] API handler implementations
- [ ] Authentication and authorization
- [ ] Comprehensive error handling

### Phase 3 - Features
- [ ] Real LLM integrations (OpenAI, Anthropic, etc.)
- [ ] Advanced metrics (BERT Score, semantic similarity)
- [ ] Experiment comparison and analysis
- [ ] Result visualization
- [ ] Export and reporting

### Phase 4 - Scale
- [ ] Distributed experiment execution
- [ ] Caching and optimization
- [ ] Monitoring and alerting
- [ ] Multi-tenancy support
- [ ] API rate limiting

## Support

For issues, questions, or contributions, please open an issue on the repository.
