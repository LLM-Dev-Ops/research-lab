# LLM Research Lab

A comprehensive, enterprise-grade platform for conducting systematic research on Large Language Models (LLMs). Built with Rust for performance, reliability, and safety.

[![Rust](https://img.shields.io/badge/rust-1.75%2B-orange.svg)](https://www.rust-lang.org/)
[![License](https://img.shields.io/badge/license-Source%20Available-blue.svg)](LICENSE)
[![Tests](https://img.shields.io/badge/tests-passing-brightgreen.svg)]()

## Overview

LLM Research Lab provides a complete infrastructure for:

- **Experiment Management** - Design, execute, and track LLM experiments with full reproducibility
- **Model Registry** - Manage multiple LLM providers and model configurations
- **Dataset Management** - Version-controlled datasets with schema validation
- **Prompt Engineering** - Template management with variable substitution and versioning
- **Evaluation Framework** - Comprehensive metrics including LLM-as-judge evaluations
- **Analytics & Observability** - Real-time metrics, alerting, and performance monitoring

## Architecture

```
┌─────────────────────────────────────────────────────────────────────┐
│                         LLM Research Lab                            │
├─────────────────────────────────────────────────────────────────────┤
│  CLI (llm-research)              │  SDK (llm-research-sdk)          │
│  ├── auth                        │  ├── LlmResearchClient           │
│  ├── experiments                 │  ├── ExperimentsClient           │
│  ├── models                      │  ├── ModelsClient                │
│  ├── datasets                    │  ├── DatasetsClient              │
│  ├── prompts                     │  ├── PromptsClient               │
│  └── evaluations                 │  └── EvaluationsClient           │
├─────────────────────────────────────────────────────────────────────┤
│                           API Layer (Axum)                          │
│  ├── REST Endpoints              │  ├── Authentication (JWT/API Key)│
│  ├── Rate Limiting               │  ├── RBAC Authorization          │
│  └── Request Validation          │  └── Audit Logging               │
├─────────────────────────────────────────────────────────────────────┤
│                         Core Services                               │
│  ├── Experiment Engine           │  ├── Workflow Orchestration      │
│  ├── Metrics Calculator          │  ├── Evaluation Runner           │
│  └── Model Adapter               │  └── Dataset Processor           │
├─────────────────────────────────────────────────────────────────────┤
│                         Storage Layer                               │
│  ├── PostgreSQL (metadata)       │  ├── ClickHouse (time-series)    │
│  └── S3 (artifacts/datasets)     │  └── Redis (caching)             │
└─────────────────────────────────────────────────────────────────────┘
```

## Features

### Experiment Management

- **Reproducible Experiments** - Full configuration tracking with random seeds
- **Collaborative Research** - Multi-user support with role-based access
- **Experiment Runs** - Execute multiple runs with configuration overrides
- **Metrics Aggregation** - Automatic statistical analysis across runs

### Model Registry

- **Multi-Provider Support** - OpenAI, Anthropic, Google, Cohere, local models
- **Version Tracking** - Track model versions and configurations
- **Custom Configurations** - Temperature, top-p, max tokens, and more

### Dataset Management

- **Multiple Formats** - JSON, JSONL, CSV, Parquet, plain text
- **Schema Validation** - Enforce data structure consistency
- **Version Control** - Full versioning with changelogs
- **Secure Storage** - Pre-signed URLs for uploads/downloads

### Prompt Engineering

- **Template System** - Mustache-style variable substitution `{{variable}}`
- **Variable Types** - String, number, boolean, array, object support
- **Version History** - Track template evolution over time
- **Validation** - Syntax checking and variable detection

### Evaluation Framework

- **Built-in Metrics**
  - BLEU, ROUGE (1/2/L), METEOR
  - Perplexity, F1 Score, Exact Match
  - Latency, Throughput, Token Usage
- **LLM-as-Judge** - Use LLMs to evaluate response quality
- **Custom Metrics** - Extensible metric calculator interface
- **Comparative Analysis** - Compare evaluations across experiments

### Enterprise Features

- **Authentication** - JWT tokens, API keys, OAuth support
- **Authorization** - Fine-grained RBAC with resource-level permissions
- **Audit Logging** - Complete audit trail for compliance
- **Rate Limiting** - Configurable rate limits per user/endpoint
- **Circuit Breakers** - Fault tolerance for external services
- **Health Checks** - Kubernetes-ready liveness/readiness probes

## Quick Start

### Prerequisites

- Rust 1.75 or later
- PostgreSQL 14+
- ClickHouse 23+
- Redis 7+
- S3-compatible storage (AWS S3, MinIO)

### Installation

```bash
# Clone the repository
git clone https://github.com/your-org/llm-research-lab.git
cd llm-research-lab

# Build all crates
cargo build --release

# Run tests
cargo test --workspace
```

### Configuration

Create a `.env` file or set environment variables:

```bash
# Database
DATABASE_URL=postgres://user:password@localhost:5432/llm_research

# ClickHouse
CLICKHOUSE_URL=http://localhost:8123

# S3 Storage
AWS_ACCESS_KEY_ID=your-access-key
AWS_SECRET_ACCESS_KEY=your-secret-key
S3_BUCKET=llm-research-data
S3_REGION=us-east-1

# Redis
REDIS_URL=redis://localhost:6379

# JWT Secret
JWT_SECRET=your-super-secret-key

# Server
API_HOST=0.0.0.0
API_PORT=8080
```

### Running the Server

```bash
# Start the API server
./target/release/llm-research-lab

# Or with environment file
source .env && ./target/release/llm-research-lab
```

---

## CLI Reference

The `llm-research` CLI provides a comprehensive interface for interacting with the platform.

### Global Options

```bash
llm-research [OPTIONS] <COMMAND>

Options:
  -o, --output <FORMAT>    Output format: table, json, yaml, compact [default: table]
      --api-url <URL>      API base URL [env: LLM_RESEARCH_API_URL]
      --api-key <KEY>      API key [env: LLM_RESEARCH_API_KEY]
  -p, --profile <NAME>     Configuration profile [env: LLM_RESEARCH_PROFILE]
  -v, --verbose            Enable verbose output
      --no-color           Disable colored output
  -h, --help               Print help
  -V, --version            Print version
```

### Authentication Commands

```bash
# Interactive login
llm-research auth login

# Login with API key
llm-research auth login --api-key YOUR_API_KEY

# Check authentication status
llm-research auth status

# Display current token
llm-research auth token

# Logout and clear credentials
llm-research auth logout
```

### Configuration Commands

```bash
# Show current configuration
llm-research config show

# Set a configuration value
llm-research config set settings.timeout_secs 60

# Get a configuration value
llm-research config get settings.output_format

# List all profiles
llm-research config profiles

# Use a specific profile
llm-research config use-profile production

# Create a new profile
llm-research config create-profile staging --api-url https://staging-api.example.com

# Delete a profile
llm-research config delete-profile old-profile --force

# Show configuration file paths
llm-research config path

# Reset to defaults
llm-research config reset --force
```

### Experiment Commands

```bash
# List experiments
llm-research experiments list
llm-research experiments list --status running --limit 50

# Get experiment details
llm-research experiments get <EXPERIMENT_ID>

# Create a new experiment
llm-research experiments create \
  --name "GPT-4 vs Claude Comparison" \
  --description "Comparing response quality" \
  --hypothesis "GPT-4 will perform better on coding tasks" \
  --tags "comparison,coding"

# Update an experiment
llm-research experiments update <ID> --name "Updated Name" --tags "new,tags"

# Delete an experiment
llm-research experiments delete <ID> --force

# Start an experiment
llm-research experiments start <ID>

# List experiment runs
llm-research experiments runs <ID>

# Create a new run
llm-research experiments run <ID> --overrides '{"temperature": 0.7}'

# Get experiment metrics
llm-research experiments metrics <ID>
```

### Model Commands

```bash
# List models
llm-research models list
llm-research models list --provider openai

# Get model details
llm-research models get <MODEL_ID>

# Register a new model
llm-research models create \
  --name "GPT-4 Turbo" \
  --provider openai \
  --identifier gpt-4-turbo-preview \
  --version "0125" \
  --config '{"temperature": 0.7, "max_tokens": 4096}'

# Update a model
llm-research models update <ID> --version "0409"

# Delete a model
llm-research models delete <ID> --force

# List available providers
llm-research models providers
```

### Dataset Commands

```bash
# List datasets
llm-research datasets list
llm-research datasets list --format jsonl --tags "training"

# Get dataset details
llm-research datasets get <DATASET_ID>

# Create a new dataset
llm-research datasets create \
  --name "Code Generation Benchmark" \
  --format jsonl \
  --description "10K coding problems" \
  --schema '{"type": "object", "properties": {"prompt": {"type": "string"}}}' \
  --tags "coding,benchmark"

# Update a dataset
llm-research datasets update <ID> --description "Updated description"

# Delete a dataset
llm-research datasets delete <ID> --force

# List dataset versions
llm-research datasets versions <ID>

# Create a new version
llm-research datasets create-version <ID> \
  --version "2.0.0" \
  --description "Added 5K more examples" \
  --changelog "Expanded dataset with code review tasks"

# Get upload URL
llm-research datasets upload <ID> --filename data.jsonl --content-type application/jsonl

# Get download URL
llm-research datasets download <ID>
```

### Prompt Commands

```bash
# List prompts
llm-research prompts list
llm-research prompts list --tags "summarization" --search "article"

# Get prompt details
llm-research prompts get <PROMPT_ID>

# Create a prompt template
llm-research prompts create \
  --name "Article Summarizer" \
  --template "Summarize the following article in {{word_count}} words:\n\n{{article}}" \
  --description "Summarizes articles to specified length" \
  --system "You are a professional summarizer. Be concise and accurate." \
  --tags "summarization,articles"

# Update a prompt
llm-research prompts update <ID> --name "New Name" --tags "updated,tags"

# Delete a prompt
llm-research prompts delete <ID> --force

# List prompt versions
llm-research prompts versions <ID>

# Create a new version
llm-research prompts create-version <ID> \
  --template "New template content with {{variables}}" \
  --changelog "Improved prompt structure"

# Render a prompt with variables
llm-research prompts render <ID> \
  --vars '{"word_count": 100, "article": "Your article text here..."}'

# Validate a template
llm-research prompts validate --template "Hello {{name}}, your order {{order_id}} is ready."
```

### Evaluation Commands

```bash
# List evaluations
llm-research evaluations list
llm-research evaluations list --experiment <EXPERIMENT_ID> --status completed

# Get evaluation details
llm-research evaluations get <EVALUATION_ID>

# Create an evaluation
llm-research evaluations create \
  --name "Quality Assessment" \
  --experiment-id <EXPERIMENT_ID> \
  --dataset-id <DATASET_ID> \
  --metrics "bleu,rouge_l,exact_match" \
  --config '{"sample_size": 1000}'

# Update an evaluation
llm-research evaluations update <ID> --name "Updated Evaluation"

# Delete an evaluation
llm-research evaluations delete <ID> --force

# Run an evaluation
llm-research evaluations run <ID>

# Get evaluation results
llm-research evaluations results <ID>

# Compare evaluations
llm-research evaluations compare <ID1> <ID2> --metrics "bleu,rouge_l"
```

### Output Formats

```bash
# Table format (default) - human-readable tables
llm-research experiments list

# JSON format - for programmatic processing
llm-research experiments list -o json

# YAML format - human-readable structured data
llm-research experiments list -o yaml

# Compact format - one line per item for scripting
llm-research experiments list -o compact
```

---

## SDK Reference

The Rust SDK provides a type-safe interface for integrating with the platform.

### Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
llm-research-sdk = "0.1"
tokio = { version = "1", features = ["full"] }
```

### Quick Start

```rust
use llm_research_sdk::{LlmResearchClient, CreateExperimentRequest};
use uuid::Uuid;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create client with API key
    let client = LlmResearchClient::builder("https://api.example.com")
        .with_api_key("your-api-key")
        .build()?;

    // List experiments
    let experiments = client.experiments().list(None).await?;
    println!("Found {} experiments", experiments.data.len());

    // Create a new experiment
    let request = CreateExperimentRequest::new(
        "My Experiment",
        Uuid::new_v4(), // owner_id
    )
    .with_description("Testing GPT-4 performance")
    .with_tags(vec!["gpt4".to_string(), "benchmark".to_string()]);

    let experiment = client.experiments().create(request).await?;
    println!("Created experiment: {}", experiment.id);

    Ok(())
}
```

### Client Configuration

```rust
use llm_research_sdk::{LlmResearchClient, AuthConfig};
use std::time::Duration;

// Basic configuration
let client = LlmResearchClient::builder("https://api.example.com")
    .with_api_key("your-api-key")
    .build()?;

// Full configuration
let client = LlmResearchClient::builder("https://api.example.com")
    .with_auth(AuthConfig::BearerToken("jwt-token".to_string()))
    .with_timeout(Duration::from_secs(60))
    .with_connect_timeout(Duration::from_secs(10))
    .with_max_retries(3)
    .with_logging(true)
    .with_header("X-Custom-Header", "value")
    .build()?;

// Using environment variables
let client = LlmResearchClient::from_env()?;
```

### Experiments API

```rust
use llm_research_sdk::{
    CreateExperimentRequest, UpdateExperimentRequest,
    ListExperimentsParams, ExperimentConfig, StartExperimentRequest
};

// List experiments with filters
let params = ListExperimentsParams::new()
    .with_limit(20)
    .with_offset(0)
    .with_status("running")
    .with_tags(vec!["production".to_string()]);

let experiments = client.experiments().list(Some(params)).await?;

// Get experiment by ID
let experiment = client.experiments().get(experiment_id).await?;

// Create experiment
let config = ExperimentConfig::new()
    .with_model(model_id)
    .with_dataset(dataset_id)
    .with_prompt_template(prompt_id)
    .with_parameter("temperature", serde_json::json!(0.7))
    .with_metric("bleu");

let request = CreateExperimentRequest::new("My Experiment", owner_id)
    .with_description("Description")
    .with_hypothesis("Hypothesis")
    .with_config(config);

let experiment = client.experiments().create(request).await?;

// Update experiment
let update = UpdateExperimentRequest::new()
    .with_name("Updated Name")
    .with_tags(vec!["updated".to_string()]);

let experiment = client.experiments().update(experiment_id, update).await?;

// Start experiment
let request = StartExperimentRequest::new();
let run = client.experiments().start(experiment_id, request).await?;

// Get runs
let runs = client.experiments().list_runs(experiment_id, None).await?;

// Get metrics
let metrics = client.experiments().get_metrics(experiment_id).await?;
println!("BLEU mean: {}", metrics.aggregated_metrics["bleu"].mean);

// Delete experiment
client.experiments().delete(experiment_id).await?;
```

### Models API

```rust
use llm_research_sdk::{CreateModelRequest, UpdateModelRequest, ListModelsParams};

// List models
let params = ListModelsParams::new()
    .with_provider("openai")
    .with_limit(50);

let models = client.models().list(Some(params)).await?;

// Get model
let model = client.models().get(model_id).await?;

// Create model
let request = CreateModelRequest::new("GPT-4", "openai", "gpt-4-turbo")
    .with_version("0125")
    .with_config(serde_json::json!({
        "temperature": 0.7,
        "max_tokens": 4096
    }));

let model = client.models().create(request).await?;

// Update model
let update = UpdateModelRequest::new()
    .with_version("0409");

let model = client.models().update(model_id, update).await?;

// List providers
let providers = client.models().list_providers().await?;

// Delete model
client.models().delete(model_id).await?;
```

### Datasets API

```rust
use llm_research_sdk::{
    CreateDatasetRequest, DatasetFormat, CreateDatasetVersionRequest, UploadRequest
};

// List datasets
let datasets = client.datasets().list(None).await?;

// Create dataset
let request = CreateDatasetRequest::new("My Dataset", DatasetFormat::Jsonl)
    .with_description("Training data")
    .with_schema(serde_json::json!({
        "type": "object",
        "properties": {
            "prompt": { "type": "string" },
            "response": { "type": "string" }
        }
    }))
    .with_tags(vec!["training".to_string()]);

let dataset = client.datasets().create(request).await?;

// Create version
let version_request = CreateDatasetVersionRequest::new("1.0.0")
    .with_description("Initial release")
    .with_changelog("First version with 10K examples");

let version = client.datasets().create_version(dataset_id, version_request).await?;

// Get upload URL
let upload = UploadRequest::new("data.jsonl", "application/jsonl");
let upload_info = client.datasets().get_upload_url(dataset_id, upload).await?;
println!("Upload to: {}", upload_info.upload_url);

// Get download URL
let download_info = client.datasets().get_download_url(dataset_id).await?;
println!("Download from: {}", download_info.download_url);
```

### Prompts API

```rust
use llm_research_sdk::{
    CreatePromptRequest, CreatePromptVersionRequest,
    RenderPromptRequest, ValidatePromptRequest
};

// List prompts
let prompts = client.prompts().list(None).await?;

// Create prompt
let request = CreatePromptRequest::new(
    "Summarizer",
    "Summarize in {{word_count}} words:\n\n{{content}}"
)
.with_description("Article summarization template")
.with_system_prompt("You are a professional summarizer.")
.with_tags(vec!["summarization".to_string()]);

let prompt = client.prompts().create(request).await?;

// Create new version
let version_request = CreatePromptVersionRequest::new(
    "Summarize the following in {{word_count}} words, focusing on key points:\n\n{{content}}"
)
.with_system_prompt("You are an expert summarizer. Be concise and accurate.")
.with_changelog("Improved prompt clarity");

let version = client.prompts().create_version(prompt_id, version_request).await?;

// Render prompt
let mut variables = std::collections::HashMap::new();
variables.insert("word_count".to_string(), serde_json::json!(100));
variables.insert("content".to_string(), serde_json::json!("Your article here..."));

let render_request = RenderPromptRequest::new(variables);
let rendered = client.prompts().render(prompt_id, render_request).await?;
println!("Rendered: {}", rendered.rendered_template);

// Validate template
let validate_request = ValidatePromptRequest::new("Hello {{name}}!");
let validation = client.prompts().validate(validate_request).await?;
println!("Valid: {}, Variables: {:?}", validation.valid, validation.detected_variables);
```

### Evaluations API

```rust
use llm_research_sdk::{
    CreateEvaluationRequest, MetricConfig, JudgeConfig, CompareEvaluationsRequest
};

// List evaluations
let evaluations = client.evaluations().list(None).await?;

// Create evaluation with metrics
let metrics = vec![
    MetricConfig::new("bleu"),
    MetricConfig::new("rouge_l"),
    MetricConfig::with_config("exact_match", serde_json::json!({"case_sensitive": false})),
];

let request = CreateEvaluationRequest::new("Quality Eval", experiment_id, dataset_id)
    .with_metrics(metrics)
    .with_sample_size(1000);

let evaluation = client.evaluations().create(request).await?;

// Create evaluation with LLM judge
let judge = JudgeConfig::new(judge_model_id)
    .with_criteria(vec!["relevance".to_string(), "accuracy".to_string()])
    .with_scale(JudgeScale::FivePoint);

let request = CreateEvaluationRequest::new("LLM Judge Eval", experiment_id, dataset_id)
    .with_judge(judge);

let evaluation = client.evaluations().create(request).await?;

// Run evaluation
let run = client.evaluations().run(evaluation_id).await?;

// Get results
let results = client.evaluations().get_results(evaluation_id).await?;
println!("BLEU: {}", results.metrics["bleu"]);

// Compare evaluations
let compare_request = CompareEvaluationsRequest::new(vec![eval1_id, eval2_id])
    .with_metrics(vec!["bleu".to_string(), "rouge_l".to_string()]);

let comparison = client.evaluations().compare(compare_request).await?;
```

### Error Handling

```rust
use llm_research_sdk::{SdkError, SdkResult};

async fn handle_errors(client: &LlmResearchClient) -> SdkResult<()> {
    match client.experiments().get(experiment_id).await {
        Ok(experiment) => {
            println!("Found: {}", experiment.name);
        }
        Err(SdkError::NotFound { resource_type, resource_id }) => {
            println!("Experiment {} not found", resource_id);
        }
        Err(SdkError::AuthenticationError(msg)) => {
            println!("Auth failed: {}", msg);
        }
        Err(SdkError::RateLimited { retry_after, .. }) => {
            println!("Rate limited, retry after {} seconds", retry_after);
        }
        Err(SdkError::ValidationError(errors)) => {
            for error in errors.errors {
                println!("Field {}: {}", error.field, error.message);
            }
        }
        Err(e) => {
            println!("Error: {}", e);
        }
    }
    Ok(())
}
```

---

## Crate Structure

| Crate | Description |
|-------|-------------|
| `llm-research-lab` | Main application binary |
| `llm-research-core` | Core domain types and traits |
| `llm-research-api` | REST API with Axum |
| `llm-research-storage` | Database and storage implementations |
| `llm-research-metrics` | Metric calculators (BLEU, ROUGE, etc.) |
| `llm-research-workflow` | Workflow orchestration and pipelines |
| `llm-research-sdk` | Rust SDK for API clients |
| `llm-research-cli` | Command-line interface |

## Metrics Reference

### Text Similarity

| Metric | Description | Range |
|--------|-------------|-------|
| BLEU | Bilingual Evaluation Understudy | 0-1 |
| ROUGE-1 | Unigram overlap | 0-1 |
| ROUGE-2 | Bigram overlap | 0-1 |
| ROUGE-L | Longest common subsequence | 0-1 |
| METEOR | Metric for Evaluation of Translation | 0-1 |

### Classification

| Metric | Description | Range |
|--------|-------------|-------|
| Exact Match | Perfect string match | 0-1 |
| F1 Score | Harmonic mean of precision/recall | 0-1 |

### Language Modeling

| Metric | Description | Range |
|--------|-------------|-------|
| Perplexity | Model uncertainty | 1-∞ (lower is better) |

### Performance

| Metric | Description | Unit |
|--------|-------------|------|
| Latency | Response time | milliseconds |
| Throughput | Requests per second | req/s |
| Token Usage | Tokens consumed | count |

## Development

### Building

```bash
# Debug build
cargo build

# Release build
cargo build --release

# Build specific crate
cargo build -p llm-research-sdk
```

### Testing

```bash
# Run all tests
cargo test --workspace

# Run tests for specific crate
cargo test -p llm-research-sdk

# Run with output
cargo test -- --nocapture
```

### Code Quality

```bash
# Format code
cargo fmt

# Run linter
cargo clippy --workspace

# Check documentation
cargo doc --workspace --no-deps
```

## Contributing

1. Fork the repository
2. Create a feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m 'Add amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request

## License

This project is licensed under the LLM Dev Ops Permanent Source-Available License. See [LICENSE](LICENSE) for details.

## Support

- Documentation: [docs.example.com](https://docs.example.com)
- Issues: [GitHub Issues](https://github.com/your-org/llm-research-lab/issues)
- Discussions: [GitHub Discussions](https://github.com/your-org/llm-research-lab/discussions)

---

Built with Rust for performance, reliability, and safety.
