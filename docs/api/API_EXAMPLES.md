# LLM Research Lab API Usage Examples

This document provides comprehensive examples for interacting with the LLM Research Lab API.

## Table of Contents
- [Authentication](#authentication)
- [Experiments](#experiments)
- [Models](#models)
- [Datasets](#datasets)
- [Prompt Templates](#prompt-templates)
- [Evaluations](#evaluations)
- [Error Handling](#error-handling)
- [Pagination](#pagination)
- [Rate Limiting](#rate-limiting)

---

## Authentication

### JWT Authentication

#### Obtain Access Token

```bash
curl -X POST https://api.llm-research-lab.io/auth/token \
  -H "Content-Type: application/json" \
  -d '{
    "username": "researcher@example.com",
    "password": "your-secure-password"
  }'
```

**Response:**
```json
{
  "access_token": "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...",
  "refresh_token": "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...",
  "token_type": "Bearer",
  "expires_in": 3600
}
```

#### Using the Access Token

```bash
curl -X GET https://api.llm-research-lab.io/experiments \
  -H "Authorization: Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9..."
```

#### Refresh Token

```bash
curl -X POST https://api.llm-research-lab.io/auth/refresh \
  -H "Content-Type: application/json" \
  -d '{
    "refresh_token": "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9..."
  }'
```

### API Key Authentication

#### Using API Key Header

```bash
curl -X GET https://api.llm-research-lab.io/experiments \
  -H "X-API-Key: llm-research_your-api-key-here"
```

---

## Experiments

### Create an Experiment

```bash
curl -X POST https://api.llm-research-lab.io/experiments \
  -H "Authorization: Bearer $ACCESS_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "GPT-4 vs Claude Comparison",
    "description": "Comparative analysis of GPT-4 and Claude on reasoning tasks",
    "hypothesis": "Claude will show stronger performance on multi-step reasoning",
    "owner_id": "550e8400-e29b-41d4-a716-446655440000",
    "collaborators": ["550e8400-e29b-41d4-a716-446655440001"],
    "tags": ["comparison", "reasoning", "benchmarking"],
    "config": {
      "model_ids": [
        "550e8400-e29b-41d4-a716-446655440100",
        "550e8400-e29b-41d4-a716-446655440101"
      ],
      "dataset_ids": ["550e8400-e29b-41d4-a716-446655440200"],
      "prompt_template_ids": ["550e8400-e29b-41d4-a716-446655440300"],
      "parameters": {
        "temperature": 0.7,
        "max_tokens": 2048,
        "top_p": 0.95
      },
      "evaluation_metrics": ["accuracy", "bleu", "latency"],
      "num_samples": 1000,
      "random_seed": 42
    }
  }'
```

**Response (201 Created):**
```json
{
  "id": "550e8400-e29b-41d4-a716-446655440010",
  "name": "GPT-4 vs Claude Comparison",
  "description": "Comparative analysis of GPT-4 and Claude on reasoning tasks",
  "hypothesis": "Claude will show stronger performance on multi-step reasoning",
  "owner_id": "550e8400-e29b-41d4-a716-446655440000",
  "collaborators": ["550e8400-e29b-41d4-a716-446655440001"],
  "tags": ["comparison", "reasoning", "benchmarking"],
  "status": "draft",
  "config": {
    "model_ids": ["550e8400-e29b-41d4-a716-446655440100", "550e8400-e29b-41d4-a716-446655440101"],
    "dataset_ids": ["550e8400-e29b-41d4-a716-446655440200"],
    "prompt_template_ids": ["550e8400-e29b-41d4-a716-446655440300"],
    "parameters": {
      "temperature": 0.7,
      "max_tokens": 2048,
      "top_p": 0.95
    },
    "evaluation_metrics": ["accuracy", "bleu", "latency"],
    "num_samples": 1000,
    "random_seed": 42
  },
  "created_at": "2025-01-15T10:30:00Z",
  "updated_at": "2025-01-15T10:30:00Z",
  "archived_at": null,
  "metadata": {}
}
```

### List Experiments with Filtering and Pagination

```bash
# List with pagination
curl -X GET "https://api.llm-research-lab.io/experiments?limit=20&offset=0" \
  -H "Authorization: Bearer $ACCESS_TOKEN"

# Filter by status
curl -X GET "https://api.llm-research-lab.io/experiments?status=running" \
  -H "Authorization: Bearer $ACCESS_TOKEN"

# Filter by owner
curl -X GET "https://api.llm-research-lab.io/experiments?owner_id=550e8400-e29b-41d4-a716-446655440000" \
  -H "Authorization: Bearer $ACCESS_TOKEN"

# Filter by tags
curl -X GET "https://api.llm-research-lab.io/experiments?tags=benchmarking,reasoning" \
  -H "Authorization: Bearer $ACCESS_TOKEN"
```

**Response:**
```json
{
  "data": [
    {
      "id": "550e8400-e29b-41d4-a716-446655440010",
      "name": "GPT-4 vs Claude Comparison",
      "status": "draft",
      "created_at": "2025-01-15T10:30:00Z"
    }
  ],
  "pagination": {
    "total": 45,
    "limit": 20,
    "offset": 0,
    "has_more": true
  },
  "links": {
    "self": "/experiments?limit=20&offset=0",
    "next": "/experiments?limit=20&offset=20",
    "first": "/experiments?limit=20&offset=0",
    "last": "/experiments?limit=20&offset=40"
  }
}
```

### Get a Specific Experiment

```bash
curl -X GET https://api.llm-research-lab.io/experiments/550e8400-e29b-41d4-a716-446655440010 \
  -H "Authorization: Bearer $ACCESS_TOKEN"
```

### Update an Experiment

```bash
curl -X PUT https://api.llm-research-lab.io/experiments/550e8400-e29b-41d4-a716-446655440010 \
  -H "Authorization: Bearer $ACCESS_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "description": "Updated description with more details",
    "tags": ["comparison", "reasoning", "benchmarking", "production"]
  }'
```

### Start an Experiment

```bash
curl -X POST https://api.llm-research-lab.io/experiments/550e8400-e29b-41d4-a716-446655440010/start \
  -H "Authorization: Bearer $ACCESS_TOKEN"
```

**Response:**
```json
{
  "id": "550e8400-e29b-41d4-a716-446655440010",
  "status": "running",
  "started_at": "2025-01-15T11:00:00Z"
}
```

### Create an Experiment Run

```bash
curl -X POST https://api.llm-research-lab.io/experiments/550e8400-e29b-41d4-a716-446655440010/runs \
  -H "Authorization: Bearer $ACCESS_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "config_overrides": {
      "parameters": {
        "temperature": 0.5
      }
    }
  }'
```

**Response:**
```json
{
  "id": "550e8400-e29b-41d4-a716-446655440020",
  "experiment_id": "550e8400-e29b-41d4-a716-446655440010",
  "status": "pending",
  "config": {
    "parameters": {
      "temperature": 0.5,
      "max_tokens": 2048,
      "top_p": 0.95
    }
  },
  "started_at": "2025-01-15T11:05:00Z",
  "completed_at": null,
  "error": null
}
```

### Complete a Run

```bash
curl -X POST https://api.llm-research-lab.io/experiments/550e8400-e29b-41d4-a716-446655440010/runs/550e8400-e29b-41d4-a716-446655440020/complete \
  -H "Authorization: Bearer $ACCESS_TOKEN"
```

### Fail a Run

```bash
curl -X POST https://api.llm-research-lab.io/experiments/550e8400-e29b-41d4-a716-446655440010/runs/550e8400-e29b-41d4-a716-446655440020/fail \
  -H "Authorization: Bearer $ACCESS_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "error": "Out of memory error during model inference"
  }'
```

### Delete an Experiment

```bash
curl -X DELETE https://api.llm-research-lab.io/experiments/550e8400-e29b-41d4-a716-446655440010 \
  -H "Authorization: Bearer $ACCESS_TOKEN"
```

---

## Models

### Create a Model

```bash
curl -X POST https://api.llm-research-lab.io/models \
  -H "Authorization: Bearer $ACCESS_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "Claude 3.5 Sonnet",
    "provider": "anthropic",
    "model_identifier": "claude-3-5-sonnet-20241022",
    "version": "20241022",
    "config": {
      "max_context_window": 200000,
      "supports_vision": true,
      "supports_function_calling": true,
      "pricing": {
        "input_per_million": 3.00,
        "output_per_million": 15.00
      }
    }
  }'
```

**Response:**
```json
{
  "id": "550e8400-e29b-41d4-a716-446655440100",
  "name": "Claude 3.5 Sonnet",
  "provider": "anthropic",
  "model_identifier": "claude-3-5-sonnet-20241022",
  "version": "20241022",
  "config": {
    "max_context_window": 200000,
    "supports_vision": true,
    "supports_function_calling": true,
    "pricing": {
      "input_per_million": 3.00,
      "output_per_million": 15.00
    }
  },
  "created_at": "2025-01-15T10:00:00Z",
  "updated_at": "2025-01-15T10:00:00Z"
}
```

### List Models

```bash
# List all models
curl -X GET https://api.llm-research-lab.io/models \
  -H "Authorization: Bearer $ACCESS_TOKEN"

# Filter by provider
curl -X GET "https://api.llm-research-lab.io/models?provider=openai" \
  -H "Authorization: Bearer $ACCESS_TOKEN"
```

### List Available Providers

```bash
curl -X GET https://api.llm-research-lab.io/models/providers \
  -H "Authorization: Bearer $ACCESS_TOKEN"
```

**Response:**
```json
[
  {
    "name": "openai",
    "display_name": "OpenAI",
    "description": "OpenAI GPT models including GPT-4 and GPT-3.5",
    "supported_models": ["gpt-4", "gpt-4-turbo", "gpt-3.5-turbo"]
  },
  {
    "name": "anthropic",
    "display_name": "Anthropic",
    "description": "Anthropic Claude models",
    "supported_models": ["claude-3-opus", "claude-3-sonnet", "claude-3-haiku"]
  },
  {
    "name": "google",
    "display_name": "Google AI",
    "description": "Google Gemini models",
    "supported_models": ["gemini-pro", "gemini-pro-vision"]
  }
]
```

### Update a Model

```bash
curl -X PUT https://api.llm-research-lab.io/models/550e8400-e29b-41d4-a716-446655440100 \
  -H "Authorization: Bearer $ACCESS_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "version": "20250101",
    "config": {
      "max_context_window": 250000
    }
  }'
```

---

## Datasets

### Create a Dataset

```bash
curl -X POST https://api.llm-research-lab.io/datasets \
  -H "Authorization: Bearer $ACCESS_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "Multi-Step Reasoning Benchmark",
    "description": "A dataset of 5000 multi-step reasoning problems",
    "format": "jsonl",
    "schema": {
      "type": "object",
      "properties": {
        "id": {"type": "string"},
        "question": {"type": "string"},
        "steps": {"type": "array", "items": {"type": "string"}},
        "answer": {"type": "string"},
        "difficulty": {"type": "string", "enum": ["easy", "medium", "hard"]}
      },
      "required": ["id", "question", "answer"]
    },
    "tags": ["reasoning", "benchmark", "multi-step"],
    "metadata": {
      "source": "internal",
      "version": "1.0.0",
      "license": "proprietary"
    }
  }'
```

**Response:**
```json
{
  "id": "550e8400-e29b-41d4-a716-446655440200",
  "name": "Multi-Step Reasoning Benchmark",
  "description": "A dataset of 5000 multi-step reasoning problems",
  "format": "jsonl",
  "schema": {
    "type": "object",
    "properties": {
      "id": {"type": "string"},
      "question": {"type": "string"},
      "steps": {"type": "array", "items": {"type": "string"}},
      "answer": {"type": "string"},
      "difficulty": {"type": "string", "enum": ["easy", "medium", "hard"]}
    },
    "required": ["id", "question", "answer"]
  },
  "tags": ["reasoning", "benchmark", "multi-step"],
  "metadata": {
    "source": "internal",
    "version": "1.0.0",
    "license": "proprietary"
  },
  "size_bytes": null,
  "row_count": null,
  "created_at": "2025-01-15T10:00:00Z",
  "updated_at": "2025-01-15T10:00:00Z"
}
```

### Upload Dataset File

```bash
# Get presigned upload URL
curl -X POST https://api.llm-research-lab.io/datasets/550e8400-e29b-41d4-a716-446655440200/upload \
  -H "Authorization: Bearer $ACCESS_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "filename": "reasoning_benchmark_v1.jsonl",
    "content_type": "application/jsonl"
  }'
```

**Response:**
```json
{
  "upload_url": "https://s3.amazonaws.com/llm-research-datasets/...",
  "expires_at": "2025-01-15T11:00:00Z"
}
```

```bash
# Upload the file
curl -X PUT "https://s3.amazonaws.com/llm-research-datasets/..." \
  -H "Content-Type: application/jsonl" \
  --data-binary @reasoning_benchmark_v1.jsonl
```

### Download Dataset

```bash
curl -X GET https://api.llm-research-lab.io/datasets/550e8400-e29b-41d4-a716-446655440200/download \
  -H "Authorization: Bearer $ACCESS_TOKEN"
```

**Response:**
```json
{
  "download_url": "https://s3.amazonaws.com/llm-research-datasets/...",
  "expires_at": "2025-01-15T11:00:00Z"
}
```

### Create Dataset Version

```bash
curl -X POST https://api.llm-research-lab.io/datasets/550e8400-e29b-41d4-a716-446655440200/versions \
  -H "Authorization: Bearer $ACCESS_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "version": "1.1.0",
    "description": "Added 500 new hard difficulty problems",
    "changelog": "- Added 500 new problems\n- Fixed 12 incorrect answers\n- Improved step explanations"
  }'
```

### List Dataset Versions

```bash
curl -X GET https://api.llm-research-lab.io/datasets/550e8400-e29b-41d4-a716-446655440200/versions \
  -H "Authorization: Bearer $ACCESS_TOKEN"
```

---

## Prompt Templates

### Create a Prompt Template

```bash
curl -X POST https://api.llm-research-lab.io/prompts \
  -H "Authorization: Bearer $ACCESS_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "Chain-of-Thought Reasoning",
    "description": "Prompts the model to think step-by-step before answering",
    "template": "You are a helpful assistant that solves problems by thinking step-by-step.\n\nProblem: {{question}}\n\nPlease solve this problem by:\n1. Breaking it down into smaller steps\n2. Explaining your reasoning for each step\n3. Arriving at the final answer\n\nThink carefully and show your work.",
    "variables": ["question"],
    "default_values": {},
    "tags": ["reasoning", "chain-of-thought", "problem-solving"],
    "metadata": {
      "author": "research-team",
      "technique": "chain-of-thought",
      "paper_reference": "https://arxiv.org/abs/2201.11903"
    }
  }'
```

**Response:**
```json
{
  "id": "550e8400-e29b-41d4-a716-446655440300",
  "name": "Chain-of-Thought Reasoning",
  "description": "Prompts the model to think step-by-step before answering",
  "template": "You are a helpful assistant that solves problems by thinking step-by-step.\n\nProblem: {{question}}\n\nPlease solve this problem by:\n1. Breaking it down into smaller steps\n2. Explaining your reasoning for each step\n3. Arriving at the final answer\n\nThink carefully and show your work.",
  "variables": ["question"],
  "default_values": {},
  "tags": ["reasoning", "chain-of-thought", "problem-solving"],
  "metadata": {
    "author": "research-team",
    "technique": "chain-of-thought",
    "paper_reference": "https://arxiv.org/abs/2201.11903"
  },
  "created_at": "2025-01-15T10:00:00Z",
  "updated_at": "2025-01-15T10:00:00Z"
}
```

### Render a Prompt Template

```bash
curl -X POST https://api.llm-research-lab.io/prompts/550e8400-e29b-41d4-a716-446655440300/render \
  -H "Authorization: Bearer $ACCESS_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "variables": {
      "question": "If a train leaves Chicago at 9 AM traveling at 60 mph, and another train leaves New York at 10 AM traveling at 80 mph toward Chicago (800 miles away), when and where will they meet?"
    }
  }'
```

**Response:**
```json
{
  "rendered": "You are a helpful assistant that solves problems by thinking step-by-step.\n\nProblem: If a train leaves Chicago at 9 AM traveling at 60 mph, and another train leaves New York at 10 AM traveling at 80 mph toward Chicago (800 miles away), when and where will they meet?\n\nPlease solve this problem by:\n1. Breaking it down into smaller steps\n2. Explaining your reasoning for each step\n3. Arriving at the final answer\n\nThink carefully and show your work."
}
```

---

## Evaluations

### Create an Evaluation

```bash
curl -X POST https://api.llm-research-lab.io/evaluations \
  -H "Authorization: Bearer $ACCESS_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "experiment_id": "550e8400-e29b-41d4-a716-446655440010",
    "run_id": "550e8400-e29b-41d4-a716-446655440020",
    "metrics": {
      "accuracy": 0.847,
      "bleu": 0.723,
      "latency_p50_ms": 1250,
      "latency_p95_ms": 2340,
      "latency_p99_ms": 3100,
      "tokens_per_second": 45.2,
      "total_tokens": 125000,
      "cost_usd": 12.50
    },
    "metadata": {
      "evaluator_version": "1.2.0",
      "hardware": "NVIDIA A100",
      "batch_size": 32
    }
  }'
```

**Response:**
```json
{
  "id": "550e8400-e29b-41d4-a716-446655440400",
  "experiment_id": "550e8400-e29b-41d4-a716-446655440010",
  "run_id": "550e8400-e29b-41d4-a716-446655440020",
  "metrics": {
    "accuracy": 0.847,
    "bleu": 0.723,
    "latency_p50_ms": 1250,
    "latency_p95_ms": 2340,
    "latency_p99_ms": 3100,
    "tokens_per_second": 45.2,
    "total_tokens": 125000,
    "cost_usd": 12.50
  },
  "metadata": {
    "evaluator_version": "1.2.0",
    "hardware": "NVIDIA A100",
    "batch_size": 32
  },
  "created_at": "2025-01-15T12:00:00Z"
}
```

### Get Experiment Metrics

```bash
curl -X GET https://api.llm-research-lab.io/experiments/550e8400-e29b-41d4-a716-446655440010/metrics \
  -H "Authorization: Bearer $ACCESS_TOKEN"
```

**Response:**
```json
{
  "experiment_id": "550e8400-e29b-41d4-a716-446655440010",
  "aggregated_metrics": {
    "accuracy": {
      "mean": 0.842,
      "std": 0.015,
      "min": 0.812,
      "max": 0.871,
      "count": 10
    },
    "bleu": {
      "mean": 0.718,
      "std": 0.023,
      "min": 0.685,
      "max": 0.756,
      "count": 10
    },
    "latency_p50_ms": {
      "mean": 1280,
      "std": 120,
      "min": 1100,
      "max": 1520,
      "count": 10
    }
  },
  "runs": [
    {
      "run_id": "550e8400-e29b-41d4-a716-446655440020",
      "metrics": {
        "accuracy": 0.847,
        "bleu": 0.723,
        "latency_p50_ms": 1250
      }
    }
  ]
}
```

---

## Error Handling

### Common Error Responses

#### 400 Bad Request
```json
{
  "error": "validation_error",
  "message": "Invalid request body",
  "details": {
    "fields": {
      "name": "Name must be between 1 and 255 characters",
      "config.temperature": "Temperature must be between 0 and 2"
    }
  },
  "request_id": "req_abc123"
}
```

#### 401 Unauthorized
```json
{
  "error": "unauthorized",
  "message": "Invalid or expired authentication token",
  "request_id": "req_abc123"
}
```

#### 403 Forbidden
```json
{
  "error": "forbidden",
  "message": "You do not have permission to access this resource",
  "required_permission": "experiments:write",
  "request_id": "req_abc123"
}
```

#### 404 Not Found
```json
{
  "error": "not_found",
  "message": "Experiment with ID 550e8400-e29b-41d4-a716-446655440010 not found",
  "request_id": "req_abc123"
}
```

#### 409 Conflict
```json
{
  "error": "conflict",
  "message": "An experiment with this name already exists",
  "existing_id": "550e8400-e29b-41d4-a716-446655440010",
  "request_id": "req_abc123"
}
```

#### 422 Unprocessable Entity
```json
{
  "error": "unprocessable_entity",
  "message": "Cannot start experiment: no datasets configured",
  "request_id": "req_abc123"
}
```

#### 429 Too Many Requests
```json
{
  "error": "rate_limit_exceeded",
  "message": "Rate limit exceeded. Please retry after 60 seconds.",
  "retry_after": 60,
  "limit": 100,
  "remaining": 0,
  "reset_at": "2025-01-15T10:01:00Z",
  "request_id": "req_abc123"
}
```

#### 500 Internal Server Error
```json
{
  "error": "internal_error",
  "message": "An unexpected error occurred. Please try again later.",
  "request_id": "req_abc123"
}
```

---

## Pagination

### Offset-Based Pagination

```bash
# First page
curl -X GET "https://api.llm-research-lab.io/experiments?limit=20&offset=0" \
  -H "Authorization: Bearer $ACCESS_TOKEN"

# Second page
curl -X GET "https://api.llm-research-lab.io/experiments?limit=20&offset=20" \
  -H "Authorization: Bearer $ACCESS_TOKEN"
```

### Cursor-Based Pagination

```bash
# First request
curl -X GET "https://api.llm-research-lab.io/experiments?limit=20" \
  -H "Authorization: Bearer $ACCESS_TOKEN"
```

**Response:**
```json
{
  "data": [...],
  "page_info": {
    "has_next_page": true,
    "has_previous_page": false,
    "start_cursor": "eyJpZCI6IjU1MGU4NDAwLWUyOWItNDFkNC1hNzE2LTQ0NjY1NTQ0MDAxMCJ9",
    "end_cursor": "eyJpZCI6IjU1MGU4NDAwLWUyOWItNDFkNC1hNzE2LTQ0NjY1NTQ0MDAyMCJ9"
  }
}
```

```bash
# Next page using cursor
curl -X GET "https://api.llm-research-lab.io/experiments?limit=20&after=eyJpZCI6IjU1MGU4NDAwLWUyOWItNDFkNC1hNzE2LTQ0NjY1NTQ0MDAyMCJ9" \
  -H "Authorization: Bearer $ACCESS_TOKEN"
```

---

## Rate Limiting

Rate limits are applied per API key or user. The current limits are indicated in response headers:

```
X-RateLimit-Limit: 1000
X-RateLimit-Remaining: 999
X-RateLimit-Reset: 1705320000
```

### Rate Limit Tiers

| Tier | Requests/Hour | Burst Limit |
|------|---------------|-------------|
| Free | 100 | 10 |
| Standard | 1,000 | 50 |
| Professional | 10,000 | 200 |
| Enterprise | Unlimited | Custom |

### Handling Rate Limits

```python
import time
import requests

def make_request_with_retry(url, headers, max_retries=3):
    for attempt in range(max_retries):
        response = requests.get(url, headers=headers)

        if response.status_code == 429:
            retry_after = int(response.headers.get('Retry-After', 60))
            print(f"Rate limited. Waiting {retry_after} seconds...")
            time.sleep(retry_after)
            continue

        return response

    raise Exception("Max retries exceeded")
```

---

## SDK Examples

### Python SDK

```python
from llm_research_lab import Client

# Initialize client
client = Client(
    api_key="your-api-key",
    base_url="https://api.llm-research-lab.io"
)

# Create an experiment
experiment = client.experiments.create(
    name="My Experiment",
    description="Testing GPT-4 on math problems",
    config={
        "model_ids": ["model-uuid"],
        "dataset_ids": ["dataset-uuid"],
        "parameters": {"temperature": 0.7}
    }
)

# Start the experiment
experiment.start()

# Create a run
run = experiment.create_run(
    config_overrides={"parameters": {"temperature": 0.5}}
)

# Wait for completion
run.wait_for_completion(timeout=3600)

# Get metrics
metrics = experiment.get_metrics()
print(f"Accuracy: {metrics.accuracy}")
```

### JavaScript/TypeScript SDK

```typescript
import { LLMResearchClient } from '@llm-research-lab/sdk';

const client = new LLMResearchClient({
  apiKey: 'your-api-key',
  baseUrl: 'https://api.llm-research-lab.io'
});

// Create an experiment
const experiment = await client.experiments.create({
  name: 'My Experiment',
  description: 'Testing GPT-4 on math problems',
  config: {
    modelIds: ['model-uuid'],
    datasetIds: ['dataset-uuid'],
    parameters: { temperature: 0.7 }
  }
});

// Start and run
await experiment.start();
const run = await experiment.createRun({
  configOverrides: { parameters: { temperature: 0.5 } }
});

// Get metrics
const metrics = await experiment.getMetrics();
console.log(`Accuracy: ${metrics.accuracy}`);
```

---

## Health Check

### Liveness Probe

```bash
curl https://api.llm-research-lab.io/health
```

**Response:**
```
OK
```

### Readiness Probe

```bash
curl https://api.llm-research-lab.io/health/ready
```

**Response:**
```json
{
  "status": "healthy",
  "components": {
    "database": "healthy",
    "cache": "healthy",
    "storage": "healthy"
  }
}
```

### Detailed Health Check

```bash
curl https://api.llm-research-lab.io/health/detailed \
  -H "Authorization: Bearer $ACCESS_TOKEN"
```

---

## Metrics Endpoint

```bash
curl https://api.llm-research-lab.io/metrics
```

**Response (Prometheus format):**
```
# HELP http_requests_total Total number of HTTP requests
# TYPE http_requests_total counter
http_requests_total{method="GET",path="/experiments",status="200"} 12345

# HELP http_request_duration_seconds HTTP request latency in seconds
# TYPE http_request_duration_seconds histogram
http_request_duration_seconds_bucket{method="GET",path="/experiments",le="0.1"} 11000
http_request_duration_seconds_bucket{method="GET",path="/experiments",le="0.5"} 12000
http_request_duration_seconds_bucket{method="GET",path="/experiments",le="1.0"} 12300
http_request_duration_seconds_bucket{method="GET",path="/experiments",le="+Inf"} 12345

# HELP db_connections_active Current number of active database connections
# TYPE db_connections_active gauge
db_connections_active 15
```
