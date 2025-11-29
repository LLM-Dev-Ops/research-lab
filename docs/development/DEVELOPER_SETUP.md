# LLM Research Lab - Developer Setup Guide

## Overview

This guide provides step-by-step instructions for setting up a local development environment for the LLM Research Lab platform.

## Table of Contents
- [Prerequisites](#prerequisites)
- [Environment Setup](#environment-setup)
- [Local Development](#local-development)
- [Testing](#testing)
- [Code Quality](#code-quality)
- [IDE Configuration](#ide-configuration)
- [Troubleshooting](#troubleshooting)

---

## Prerequisites

### Required Software

| Software | Version | Purpose | Installation |
|----------|---------|---------|--------------|
| Rust | >= 1.75 | Primary language | [rustup.rs](https://rustup.rs) |
| Docker | >= 24.0 | Container runtime | [docker.com](https://docker.com) |
| Docker Compose | >= 2.20 | Local services | Included with Docker Desktop |
| PostgreSQL Client | >= 15 | Database tools | `brew install postgresql` / `apt install postgresql-client` |
| Git | >= 2.40 | Version control | System package manager |

### Recommended Software

| Software | Purpose | Installation |
|----------|---------|--------------|
| Just | Command runner | `cargo install just` |
| cargo-watch | Auto-rebuild | `cargo install cargo-watch` |
| cargo-nextest | Better test runner | `cargo install cargo-nextest` |
| sqlx-cli | Database migrations | `cargo install sqlx-cli` |
| pre-commit | Git hooks | `pip install pre-commit` |

### Hardware Requirements

| Requirement | Minimum | Recommended |
|-------------|---------|-------------|
| RAM | 8 GB | 16 GB |
| CPU | 4 cores | 8 cores |
| Disk | 20 GB free | 50 GB free (SSD) |

---

## Environment Setup

### Step 1: Clone Repository

```bash
# Clone the repository
git clone https://github.com/llm-research-lab/llm-research-lab.git
cd llm-research-lab

# Verify you're on the main branch
git branch
```

### Step 2: Install Rust Toolchain

```bash
# Install Rust via rustup
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Restart shell or source the environment
source $HOME/.cargo/env

# Verify installation
rustc --version
cargo --version

# Install required components
rustup component add rustfmt clippy

# Install nightly for some tools (optional)
rustup install nightly
```

### Step 3: Start Local Services

```bash
# Start PostgreSQL and other services
docker compose up -d

# Verify services are running
docker compose ps

# Expected output:
# NAME                 STATUS
# postgres             running (healthy)
# clickhouse           running (healthy)
# minio                running (healthy)
```

**docker-compose.yml for local development:**

```yaml
version: '3.8'

services:
  postgres:
    image: postgres:15-alpine
    container_name: llm-research-postgres
    environment:
      POSTGRES_USER: llm_research
      POSTGRES_PASSWORD: development
      POSTGRES_DB: llm_research
    ports:
      - "5432:5432"
    volumes:
      - postgres_data:/var/lib/postgresql/data
    healthcheck:
      test: ["CMD-SHELL", "pg_isready -U llm_research"]
      interval: 5s
      timeout: 5s
      retries: 5

  clickhouse:
    image: clickhouse/clickhouse-server:23.8
    container_name: llm-research-clickhouse
    environment:
      CLICKHOUSE_USER: llm_research
      CLICKHOUSE_PASSWORD: development
      CLICKHOUSE_DB: llm_metrics
    ports:
      - "8123:8123"
      - "9000:9000"
    volumes:
      - clickhouse_data:/var/lib/clickhouse
    healthcheck:
      test: ["CMD", "clickhouse-client", "--query", "SELECT 1"]
      interval: 5s
      timeout: 5s
      retries: 5

  minio:
    image: minio/minio:latest
    container_name: llm-research-minio
    environment:
      MINIO_ROOT_USER: minioadmin
      MINIO_ROOT_PASSWORD: minioadmin
    command: server /data --console-address ":9001"
    ports:
      - "9000:9000"
      - "9001:9001"
    volumes:
      - minio_data:/data

volumes:
  postgres_data:
  clickhouse_data:
  minio_data:
```

### Step 4: Configure Environment Variables

```bash
# Copy example environment file
cp .env.example .env

# Edit with your local settings
vim .env
```

**.env file contents:**

```bash
# Database
DATABASE_URL=postgresql://llm_research:development@localhost:5432/llm_research

# ClickHouse
CLICKHOUSE_URL=http://localhost:8123
CLICKHOUSE_USER=llm_research
CLICKHOUSE_PASSWORD=development
CLICKHOUSE_DATABASE=llm_metrics

# S3 (MinIO)
AWS_ACCESS_KEY_ID=minioadmin
AWS_SECRET_ACCESS_KEY=minioadmin
AWS_REGION=us-east-1
S3_BUCKET=llm-research-datasets
S3_ENDPOINT=http://localhost:9000

# Security
JWT_SECRET=development-jwt-secret-key-do-not-use-in-production
API_KEY_SECRET=development-api-key-secret

# Server
SERVER_HOST=127.0.0.1
SERVER_PORT=8080
METRICS_PORT=9090

# Logging
RUST_LOG=debug,llm_research_api=debug
LOG_FORMAT=pretty

# Development
ENVIRONMENT=development
```

### Step 5: Run Database Migrations

```bash
# Install sqlx-cli if not already installed
cargo install sqlx-cli --features postgres

# Run migrations
sqlx migrate run --database-url $DATABASE_URL

# Verify migrations
sqlx migrate info --database-url $DATABASE_URL
```

### Step 6: Create S3 Bucket (MinIO)

```bash
# Install MinIO client
brew install minio/stable/mc  # macOS
# or
wget https://dl.min.io/client/mc/release/linux-amd64/mc  # Linux

# Configure MinIO client
mc alias set local http://localhost:9000 minioadmin minioadmin

# Create bucket
mc mb local/llm-research-datasets

# Verify bucket
mc ls local/
```

### Step 7: Build and Run

```bash
# Build all workspace members
cargo build --workspace

# Run the API server
cargo run --package llm-research-lab

# Or with auto-reload
cargo watch -x 'run --package llm-research-lab'
```

---

## Local Development

### Project Structure

```
llm-research-lab/
├── llm-research-core/      # Domain models and traits
├── llm-research-storage/   # Database implementations
├── llm-research-metrics/   # Metric calculations
├── llm-research-workflow/  # Experiment orchestration
├── llm-research-api/       # HTTP API layer
├── llm-research-lab/       # Binary entrypoint
├── migrations/             # Database migrations
├── tests/                  # Integration tests
├── docs/                   # Documentation
└── scripts/                # Development scripts
```

### Development Workflow

```bash
# 1. Create a feature branch
git checkout -b feature/my-feature

# 2. Make changes and run tests
cargo test --workspace

# 3. Check formatting and linting
cargo fmt --check
cargo clippy --workspace -- -D warnings

# 4. Run the full CI suite locally
./scripts/ci-local.sh

# 5. Commit changes
git add .
git commit -m "feat: add my feature"

# 6. Push and create PR
git push -u origin feature/my-feature
```

### Useful Commands

```bash
# Build
cargo build --workspace                    # Debug build
cargo build --workspace --release          # Release build

# Run
cargo run --package llm-research-lab       # Run server
cargo run --package llm-research-lab -- --help  # Show help

# Test
cargo test --workspace                     # Run all tests
cargo test --package llm-research-api      # Run specific package tests
cargo nextest run --workspace              # Run with nextest (faster)

# Format & Lint
cargo fmt --all                            # Format code
cargo clippy --workspace                   # Run linter
cargo clippy --fix --workspace             # Auto-fix lint issues

# Documentation
cargo doc --workspace --open               # Generate and open docs

# Database
sqlx migrate run                           # Run migrations
sqlx migrate revert                        # Revert last migration
sqlx migrate add <name>                    # Create new migration

# Dependencies
cargo update                               # Update dependencies
cargo outdated                             # Check for outdated deps
cargo audit                                # Security audit
```

### API Testing

```bash
# Health check
curl http://localhost:8080/health

# List providers (no auth required)
curl http://localhost:8080/models/providers

# Create experiment (requires auth)
curl -X POST http://localhost:8080/experiments \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $JWT_TOKEN" \
  -d '{
    "name": "Test Experiment",
    "owner_id": "550e8400-e29b-41d4-a716-446655440000",
    "config": {
      "model_ids": [],
      "dataset_ids": [],
      "prompt_template_ids": [],
      "parameters": {},
      "evaluation_metrics": []
    }
  }'
```

### Debugging

```bash
# Enable full backtrace
RUST_BACKTRACE=full cargo run --package llm-research-lab

# Enable debug logging
RUST_LOG=debug cargo run --package llm-research-lab

# Enable trace logging (very verbose)
RUST_LOG=trace cargo run --package llm-research-lab

# Profile with flamegraph
cargo install flamegraph
CARGO_PROFILE_RELEASE_DEBUG=true cargo flamegraph --package llm-research-lab
```

---

## Testing

### Unit Tests

```bash
# Run all unit tests
cargo test --workspace

# Run tests with output
cargo test --workspace -- --nocapture

# Run specific test
cargo test test_create_experiment

# Run tests in specific module
cargo test --package llm-research-api security::

# Run tests with coverage
cargo install cargo-tarpaulin
cargo tarpaulin --workspace --out Html
```

### Integration Tests

```bash
# Ensure services are running
docker compose up -d

# Run integration tests
cargo test --workspace --test '*'

# Run with test database
DATABASE_URL=postgresql://llm_research:development@localhost:5432/llm_research_test \
  cargo test --workspace --test '*'
```

### Test Categories

```bash
# Run only fast tests
cargo test --workspace -- --skip slow

# Run benchmark tests
cargo test --workspace -- --ignored
```

---

## Code Quality

### Pre-commit Hooks

```bash
# Install pre-commit
pip install pre-commit

# Install hooks
pre-commit install

# Run manually
pre-commit run --all-files
```

**.pre-commit-config.yaml:**

```yaml
repos:
  - repo: local
    hooks:
      - id: cargo-fmt
        name: cargo fmt
        entry: cargo fmt --all --
        language: system
        types: [rust]
        pass_filenames: false

      - id: cargo-clippy
        name: cargo clippy
        entry: cargo clippy --workspace -- -D warnings
        language: system
        types: [rust]
        pass_filenames: false

      - id: cargo-test
        name: cargo test
        entry: cargo test --workspace
        language: system
        types: [rust]
        pass_filenames: false
```

### Code Standards

```rust
// Example: Good code style

/// Creates a new experiment with the given configuration.
///
/// # Arguments
///
/// * `req` - The experiment creation request
///
/// # Returns
///
/// Returns the created experiment or an error
///
/// # Errors
///
/// Returns `ApiError::Validation` if the request is invalid
/// Returns `ApiError::Database` if the database operation fails
pub async fn create_experiment(
    State(state): State<AppState>,
    ValidatedJson(req): ValidatedJson<CreateExperimentRequest>,
) -> Result<Json<ExperimentResponse>, ApiError> {
    let experiment = state
        .experiment_service
        .create(req.into())
        .await
        .map_err(ApiError::from)?;

    Ok(Json(experiment.into()))
}
```

---

## IDE Configuration

### VS Code

**Recommended Extensions:**

- rust-analyzer
- Even Better TOML
- crates
- Error Lens
- GitLens

**.vscode/settings.json:**

```json
{
  "rust-analyzer.cargo.features": "all",
  "rust-analyzer.checkOnSave.command": "clippy",
  "rust-analyzer.checkOnSave.extraArgs": ["--", "-D", "warnings"],
  "[rust]": {
    "editor.formatOnSave": true,
    "editor.defaultFormatter": "rust-lang.rust-analyzer"
  },
  "rust-analyzer.inlayHints.chainingHints.enable": true,
  "rust-analyzer.inlayHints.typeHints.enable": true
}
```

### IntelliJ IDEA / CLion

1. Install Rust plugin
2. Open project as Cargo project
3. Configure:
   - Settings → Languages & Frameworks → Rust
   - Enable "Run rustfmt on save"
   - Enable "Run external linter (Clippy)"

### Neovim

**With rust-tools.nvim:**

```lua
require('rust-tools').setup({
  tools = {
    autoSetHints = true,
    inlay_hints = {
      show_parameter_hints = true,
      parameter_hints_prefix = "<- ",
      other_hints_prefix = "=> ",
    },
  },
  server = {
    settings = {
      ["rust-analyzer"] = {
        checkOnSave = {
          command = "clippy",
        },
      },
    },
  },
})
```

---

## Troubleshooting

### Common Issues

#### Database Connection Failed

```
Error: failed to connect to database
```

**Solution:**
```bash
# Check PostgreSQL is running
docker compose ps

# Restart PostgreSQL
docker compose restart postgres

# Check connection
psql $DATABASE_URL -c "SELECT 1"
```

#### Migration Failed

```
Error: migration failed
```

**Solution:**
```bash
# Check current migration state
sqlx migrate info --database-url $DATABASE_URL

# Reset database (development only)
docker compose down -v
docker compose up -d
sqlx migrate run --database-url $DATABASE_URL
```

#### Compilation Errors

```
error[E0433]: failed to resolve
```

**Solution:**
```bash
# Clean and rebuild
cargo clean
cargo build --workspace

# Update dependencies
cargo update
```

#### Port Already in Use

```
Error: Address already in use (os error 98)
```

**Solution:**
```bash
# Find process using port
lsof -i :8080

# Kill process
kill -9 <PID>

# Or use different port
SERVER_PORT=8081 cargo run --package llm-research-lab
```

### Getting Help

1. Check existing documentation in `/docs`
2. Search GitHub issues
3. Ask in #dev-help Slack channel
4. Create a new GitHub issue with:
   - Rust version (`rustc --version`)
   - OS and version
   - Steps to reproduce
   - Full error output

---

## Next Steps

After setting up your development environment:

1. Read the [Architecture Documentation](../architecture/SYSTEM_ARCHITECTURE.md)
2. Review the [API Documentation](../api/openapi.yaml)
3. Check the [Contributing Guidelines](../../CONTRIBUTING.md)
4. Pick an issue labeled `good-first-issue`
