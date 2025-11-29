# LLM Research Lab - System Architecture

## Overview

The LLM Research Lab is a platform for conducting, managing, and analyzing experiments with Large Language Models. This document describes the overall system architecture, component interactions, and deployment topology.

## High-Level Architecture

```
                                    ┌─────────────────────────────────────────────────────────────┐
                                    │                      Internet                                │
                                    └─────────────────────────────────┬───────────────────────────┘
                                                                      │
                                    ┌─────────────────────────────────▼───────────────────────────┐
                                    │                  AWS CloudFront (CDN)                        │
                                    │              Global Edge Caching + WAF                       │
                                    └─────────────────────────────────┬───────────────────────────┘
                                                                      │
                                    ┌─────────────────────────────────▼───────────────────────────┐
                                    │              Application Load Balancer (ALB)                 │
                                    │           TLS Termination + Health Checks                    │
                                    └─────────────────────────────────┬───────────────────────────┘
                                                                      │
                    ┌─────────────────────────────────────────────────┴─────────────────────────────────────────────────┐
                    │                                                                                                   │
                    │                                    EKS Kubernetes Cluster                                         │
                    │  ┌─────────────────────────────────────────────────────────────────────────────────────────────┐  │
                    │  │                                                                                             │  │
                    │  │   ┌───────────────┐    ┌───────────────┐    ┌───────────────┐    ┌───────────────┐         │  │
                    │  │   │ API Pod (1)   │    │ API Pod (2)   │    │ API Pod (3)   │    │ API Pod (N)   │         │  │
                    │  │   │               │    │               │    │               │    │               │         │  │
                    │  │   │ ┌───────────┐ │    │ ┌───────────┐ │    │ ┌───────────┐ │    │ ┌───────────┐ │         │  │
                    │  │   │ │Rate Limit │ │    │ │Rate Limit │ │    │ │Rate Limit │ │    │ │Rate Limit │ │         │  │
                    │  │   │ └─────┬─────┘ │    │ └─────┬─────┘ │    │ └─────┬─────┘ │    │ └─────┬─────┘ │         │  │
                    │  │   │ ┌─────▼─────┐ │    │ ┌─────▼─────┐ │    │ ┌─────▼─────┐ │    │ ┌─────▼─────┐ │         │  │
                    │  │   │ │   Auth    │ │    │ │   Auth    │ │    │ │   Auth    │ │    │ │   Auth    │ │         │  │
                    │  │   │ └─────┬─────┘ │    │ └─────┬─────┘ │    │ └─────┬─────┘ │    │ └─────┬─────┘ │         │  │
                    │  │   │ ┌─────▼─────┐ │    │ ┌─────▼─────┐ │    │ ┌─────▼─────┐ │    │ ┌─────▼─────┐ │         │  │
                    │  │   │ │  Handler  │ │    │ │  Handler  │ │    │ │  Handler  │ │    │ │  Handler  │ │         │  │
                    │  │   │ └───────────┘ │    │ └───────────┘ │    │ └───────────┘ │    │ └───────────┘ │         │  │
                    │  │   └───────┬───────┘    └───────┬───────┘    └───────┬───────┘    └───────┬───────┘         │  │
                    │  │           │                    │                    │                    │                 │  │
                    │  └───────────┼────────────────────┼────────────────────┼────────────────────┼─────────────────┘  │
                    │              │                    │                    │                    │                    │
                    └──────────────┼────────────────────┼────────────────────┼────────────────────┼────────────────────┘
                                   │                    │                    │                    │
           ┌───────────────────────┴────────────────────┴────────────────────┴────────────────────┴───────────────────────┐
           │                                                                                                              │
           │                                              Data Layer                                                      │
           │                                                                                                              │
           │  ┌─────────────────────────┐   ┌─────────────────────────┐   ┌─────────────────────────┐                    │
           │  │                         │   │                         │   │                         │                    │
           │  │     PostgreSQL RDS      │   │      ClickHouse         │   │       Amazon S3         │                    │
           │  │    (Transactional)      │   │    (Time-Series)        │   │    (Object Storage)     │                    │
           │  │                         │   │                         │   │                         │                    │
           │  │  ┌─────────────────┐    │   │  ┌─────────────────┐    │   │  ┌─────────────────┐    │                    │
           │  │  │    Primary      │    │   │  │    Node 1       │    │   │  │    Datasets     │    │                    │
           │  │  │    Instance     │    │   │  │                 │    │   │  │                 │    │                    │
           │  │  └────────┬────────┘    │   │  └────────┬────────┘    │   │  └─────────────────┘    │                    │
           │  │           │             │   │           │             │   │                         │                    │
           │  │  ┌────────▼────────┐    │   │  ┌────────▼────────┐    │   │  ┌─────────────────┐    │                    │
           │  │  │  Read Replica   │    │   │  │    Node 2       │    │   │  │    Artifacts    │    │                    │
           │  │  │                 │    │   │  │                 │    │   │  │                 │    │                    │
           │  │  └─────────────────┘    │   │  └────────┬────────┘    │   │  └─────────────────┘    │                    │
           │  │                         │   │           │             │   │                         │                    │
           │  │                         │   │  ┌────────▼────────┐    │   │  ┌─────────────────┐    │                    │
           │  │                         │   │  │    Node 3       │    │   │  │    Backups      │    │                    │
           │  │                         │   │  │                 │    │   │  │                 │    │                    │
           │  │                         │   │  └─────────────────┘    │   │  └─────────────────┘    │                    │
           │  └─────────────────────────┘   └─────────────────────────┘   └─────────────────────────┘                    │
           │                                                                                                              │
           └──────────────────────────────────────────────────────────────────────────────────────────────────────────────┘
```

## Component Breakdown

### 1. API Layer

The API layer is built with Rust and Axum framework, deployed as stateless containers in Kubernetes.

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                            API Pod Architecture                              │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                              │
│  ┌────────────────────────────────────────────────────────────────────────┐ │
│  │                        Middleware Stack                                 │ │
│  │  ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌──────────────┐  │ │
│  │  │ Tracing  │→│Compression│→│Rate Limit│→│   Auth   │→│Security Hdrs │  │ │
│  │  └──────────┘ └──────────┘ └──────────┘ └──────────┘ └──────────────┘  │ │
│  └────────────────────────────────────┬───────────────────────────────────┘ │
│                                       │                                      │
│  ┌────────────────────────────────────▼───────────────────────────────────┐ │
│  │                          Router Layer                                   │ │
│  │  ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌──────────────┐  │ │
│  │  │/experiments│/models    │ │/datasets │ │/prompts  │ │/evaluations  │  │ │
│  │  └──────────┘ └──────────┘ └──────────┘ └──────────┘ └──────────────┘  │ │
│  └────────────────────────────────────┬───────────────────────────────────┘ │
│                                       │                                      │
│  ┌────────────────────────────────────▼───────────────────────────────────┐ │
│  │                         Handler Layer                                   │ │
│  │  ┌─────────────────────────────────────────────────────────────────┐   │ │
│  │  │  Request Validation │ Business Logic │ Response Serialization  │   │ │
│  │  └─────────────────────────────────────────────────────────────────┘   │ │
│  └────────────────────────────────────┬───────────────────────────────────┘ │
│                                       │                                      │
│  ┌────────────────────────────────────▼───────────────────────────────────┐ │
│  │                        Service Layer                                    │ │
│  │  ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌──────────────┐  │ │
│  │  │Experiment│ │  Model   │ │ Dataset  │ │  Prompt  │ │ Evaluation   │  │ │
│  │  │ Service  │ │ Service  │ │ Service  │ │ Service  │ │   Service    │  │ │
│  │  └──────────┘ └──────────┘ └──────────┘ └──────────┘ └──────────────┘  │ │
│  └────────────────────────────────────────────────────────────────────────┘ │
│                                                                              │
└─────────────────────────────────────────────────────────────────────────────┘
```

### 2. Workspace Structure

```
llm-research-lab/
├── llm-research-core/          # Domain models, interfaces, shared types
│   ├── src/
│   │   ├── domain/             # Domain entities
│   │   │   ├── experiment.rs
│   │   │   ├── model.rs
│   │   │   ├── dataset.rs
│   │   │   └── ...
│   │   ├── repository.rs       # Repository traits
│   │   └── lib.rs
│   └── Cargo.toml
│
├── llm-research-storage/       # Database implementations
│   ├── src/
│   │   ├── repositories/       # PostgreSQL implementations
│   │   ├── clickhouse/         # ClickHouse implementations
│   │   ├── s3/                 # S3 implementations
│   │   └── lib.rs
│   └── Cargo.toml
│
├── llm-research-metrics/       # Metrics calculation
│   ├── src/
│   │   ├── calculators/        # Metric calculators
│   │   │   ├── bleu.rs
│   │   │   ├── accuracy.rs
│   │   │   ├── latency.rs
│   │   │   └── ...
│   │   ├── aggregators.rs
│   │   └── lib.rs
│   └── Cargo.toml
│
├── llm-research-workflow/      # Experiment orchestration
│   ├── src/
│   │   ├── engine.rs           # Workflow engine
│   │   ├── executor.rs         # Task execution
│   │   ├── pipeline.rs         # Pipeline definition
│   │   └── lib.rs
│   └── Cargo.toml
│
├── llm-research-api/           # HTTP API layer
│   ├── src/
│   │   ├── handlers/           # Request handlers
│   │   ├── dto/                # DTOs
│   │   ├── middleware/         # Middleware
│   │   ├── security/           # Security (auth, rate limit)
│   │   ├── observability/      # Metrics, tracing, logging
│   │   ├── performance/        # Caching, connection pools
│   │   ├── resilience/         # Circuit breaker, retry
│   │   ├── reliability/        # Bulkhead, load shedding
│   │   ├── response/           # Compression, pagination
│   │   └── lib.rs
│   └── Cargo.toml
│
├── llm-research-lab/           # Binary entrypoint
│   ├── src/
│   │   ├── main.rs
│   │   ├── config.rs
│   │   └── server.rs
│   └── Cargo.toml
│
├── docs/                       # Documentation
│   ├── api/                    # API documentation
│   ├── architecture/           # Architecture docs
│   ├── operations/             # Operational docs
│   └── development/            # Developer docs
│
└── Cargo.toml                  # Workspace manifest
```

### 3. Data Flow Diagrams

#### Experiment Creation Flow

```
┌──────────┐     ┌──────────┐     ┌──────────┐     ┌──────────┐     ┌──────────┐
│  Client  │     │   API    │     │ Validator│     │ Service  │     │PostgreSQL│
└────┬─────┘     └────┬─────┘     └────┬─────┘     └────┬─────┘     └────┬─────┘
     │                │                │                │                │
     │ POST /experiments               │                │                │
     │───────────────▶│                │                │                │
     │                │                │                │                │
     │                │ Validate JWT   │                │                │
     │                │───────────────▶│                │                │
     │                │                │                │                │
     │                │ Validate Body  │                │                │
     │                │───────────────▶│                │                │
     │                │                │                │                │
     │                │                │ Create         │                │
     │                │                │───────────────▶│                │
     │                │                │                │                │
     │                │                │                │ INSERT         │
     │                │                │                │───────────────▶│
     │                │                │                │                │
     │                │                │                │    OK          │
     │                │                │                │◀───────────────│
     │                │                │                │                │
     │                │                │  Experiment    │                │
     │                │                │◀───────────────│                │
     │                │                │                │                │
     │  201 Created + JSON             │                │                │
     │◀────────────────────────────────│                │                │
     │                │                │                │                │
```

#### Experiment Execution Flow

```
┌──────────┐     ┌──────────┐     ┌──────────┐     ┌──────────┐     ┌──────────┐
│  Client  │     │   API    │     │ Workflow │     │ Executor │     │ LLM API  │
└────┬─────┘     └────┬─────┘     └────┬─────┘     └────┬─────┘     └────┬─────┘
     │                │                │                │                │
     │ POST /experiments/{id}/start    │                │                │
     │───────────────▶│                │                │                │
     │                │                │                │                │
     │                │ Start Pipeline │                │                │
     │                │───────────────▶│                │                │
     │                │                │                │                │
     │                │                │ Execute Tasks  │                │
     │                │                │───────────────▶│                │
     │                │                │                │                │
     │                │                │                │ Call Model     │
     │                │                │                │───────────────▶│
     │                │                │                │                │
     │                │                │                │   Response     │
     │                │                │                │◀───────────────│
     │                │                │                │                │
     │                │                │  Results       │                │
     │                │                │◀───────────────│                │
     │                │                │                │                │
     │                │ Store Metrics  │                │                │
     │                │◀───────────────│                │                │
     │                │                │                │                │
     │  202 Accepted                   │                │                │
     │◀────────────────────────────────│                │                │
```

### 4. Deployment Architecture

```
┌─────────────────────────────────────────────────────────────────────────────────────────────┐
│                                    AWS Account                                               │
│                                                                                              │
│  ┌───────────────────────────────────────────────────────────────────────────────────────┐  │
│  │                                  VPC (10.0.0.0/16)                                     │  │
│  │                                                                                        │  │
│  │  ┌─────────────────────────────────┐   ┌─────────────────────────────────┐           │  │
│  │  │     Public Subnet (10.0.1.0/24) │   │    Public Subnet (10.0.2.0/24)  │           │  │
│  │  │                                 │   │                                  │           │  │
│  │  │  ┌───────────────────────────┐  │   │  ┌───────────────────────────┐  │           │  │
│  │  │  │    NAT Gateway (AZ-a)     │  │   │  │    NAT Gateway (AZ-b)     │  │           │  │
│  │  │  └───────────────────────────┘  │   │  └───────────────────────────┘  │           │  │
│  │  │                                 │   │                                  │           │  │
│  │  │  ┌───────────────────────────┐  │   │  ┌───────────────────────────┐  │           │  │
│  │  │  │         ALB (AZ-a)        │  │   │  │         ALB (AZ-b)        │  │           │  │
│  │  │  └───────────────────────────┘  │   │  └───────────────────────────┘  │           │  │
│  │  └─────────────────────────────────┘   └─────────────────────────────────┘           │  │
│  │                                                                                        │  │
│  │  ┌─────────────────────────────────┐   ┌─────────────────────────────────┐           │  │
│  │  │   Private Subnet (10.0.10.0/24) │   │  Private Subnet (10.0.20.0/24)  │           │  │
│  │  │                                 │   │                                  │           │  │
│  │  │  ┌───────────────────────────┐  │   │  ┌───────────────────────────┐  │           │  │
│  │  │  │     EKS Node Group (AZ-a) │  │   │  │    EKS Node Group (AZ-b)  │  │           │  │
│  │  │  │                           │  │   │  │                           │  │           │  │
│  │  │  │  ┌─────┐ ┌─────┐ ┌─────┐  │  │   │  │  ┌─────┐ ┌─────┐ ┌─────┐  │  │           │  │
│  │  │  │  │ Pod │ │ Pod │ │ Pod │  │  │   │  │  │ Pod │ │ Pod │ │ Pod │  │  │           │  │
│  │  │  │  └─────┘ └─────┘ └─────┘  │  │   │  │  └─────┘ └─────┘ └─────┘  │  │           │  │
│  │  │  └───────────────────────────┘  │   │  └───────────────────────────┘  │           │  │
│  │  └─────────────────────────────────┘   └─────────────────────────────────┘           │  │
│  │                                                                                        │  │
│  │  ┌─────────────────────────────────┐   ┌─────────────────────────────────┐           │  │
│  │  │     Data Subnet (10.0.100.0/24) │   │    Data Subnet (10.0.200.0/24)  │           │  │
│  │  │                                 │   │                                  │           │  │
│  │  │  ┌───────────────────────────┐  │   │  ┌───────────────────────────┐  │           │  │
│  │  │  │   RDS Primary (AZ-a)      │  │   │  │   RDS Standby (AZ-b)      │  │           │  │
│  │  │  └───────────────────────────┘  │   │  └───────────────────────────┘  │           │  │
│  │  │                                 │   │                                  │           │  │
│  │  │  ┌───────────────────────────┐  │   │  ┌───────────────────────────┐  │           │  │
│  │  │  │  ClickHouse Node 1        │  │   │  │  ClickHouse Node 2        │  │           │  │
│  │  │  └───────────────────────────┘  │   │  └───────────────────────────┘  │           │  │
│  │  └─────────────────────────────────┘   └─────────────────────────────────┘           │  │
│  │                                                                                        │  │
│  └───────────────────────────────────────────────────────────────────────────────────────┘  │
│                                                                                              │
│  ┌───────────────────────────┐  ┌───────────────────────────┐                              │
│  │        Amazon S3          │  │     AWS Secrets Manager    │                              │
│  │   (Dataset Storage)       │  │      (Secrets Store)       │                              │
│  └───────────────────────────┘  └───────────────────────────┘                              │
│                                                                                              │
└─────────────────────────────────────────────────────────────────────────────────────────────┘
```

### 5. Security Architecture

```
┌────────────────────────────────────────────────────────────────────────────────┐
│                              Security Layers                                    │
│                                                                                 │
│  ┌─────────────────────────────────────────────────────────────────────────┐   │
│  │  Layer 1: Network Security                                               │   │
│  │  ┌───────────┐  ┌───────────┐  ┌───────────┐  ┌───────────────────────┐ │   │
│  │  │    WAF    │  │  Shield   │  │ CloudFront│  │   Security Groups     │ │   │
│  │  │           │  │  (DDoS)   │  │  (CDN)    │  │   (Network ACLs)      │ │   │
│  │  └───────────┘  └───────────┘  └───────────┘  └───────────────────────┘ │   │
│  └─────────────────────────────────────────────────────────────────────────┘   │
│                                                                                 │
│  ┌─────────────────────────────────────────────────────────────────────────┐   │
│  │  Layer 2: Transport Security                                             │   │
│  │  ┌───────────────────────────────┐  ┌───────────────────────────────┐   │   │
│  │  │  TLS 1.3 (ALB Termination)    │  │  mTLS (Service Mesh)          │   │   │
│  │  └───────────────────────────────┘  └───────────────────────────────┘   │   │
│  └─────────────────────────────────────────────────────────────────────────┘   │
│                                                                                 │
│  ┌─────────────────────────────────────────────────────────────────────────┐   │
│  │  Layer 3: Application Security                                           │   │
│  │  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌───────────┐ │   │
│  │  │   Rate   │  │   Auth   │  │   RBAC   │  │  Input   │  │  Audit    │ │   │
│  │  │  Limit   │  │ (JWT/Key)│  │ (Roles)  │  │Validation│  │  Logging  │ │   │
│  │  └──────────┘  └──────────┘  └──────────┘  └──────────┘  └───────────┘ │   │
│  └─────────────────────────────────────────────────────────────────────────┘   │
│                                                                                 │
│  ┌─────────────────────────────────────────────────────────────────────────┐   │
│  │  Layer 4: Data Security                                                  │   │
│  │  ┌──────────────────────┐  ┌──────────────────────┐  ┌─────────────┐   │   │
│  │  │ Encryption at Rest   │  │ Encryption in Transit│  │ Key Mgmt    │   │   │
│  │  │ (RDS, S3, EBS)       │  │ (TLS everywhere)     │  │ (KMS)       │   │   │
│  │  └──────────────────────┘  └──────────────────────┘  └─────────────┘   │   │
│  └─────────────────────────────────────────────────────────────────────────┘   │
│                                                                                 │
└────────────────────────────────────────────────────────────────────────────────┘
```

## Technology Stack

| Layer | Technology | Purpose |
|-------|------------|---------|
| Language | Rust | Performance, safety |
| Web Framework | Axum | HTTP API |
| Async Runtime | Tokio | Async I/O |
| Database (OLTP) | PostgreSQL | Transactional data |
| Database (OLAP) | ClickHouse | Time-series metrics |
| Object Storage | Amazon S3 | File storage |
| Caching | In-memory (LRU) | Query caching |
| Container Runtime | Docker | Containerization |
| Orchestration | Kubernetes (EKS) | Container orchestration |
| Load Balancer | AWS ALB | Traffic distribution |
| CDN | CloudFront | Edge caching |
| Secrets | AWS Secrets Manager | Secret storage |
| Monitoring | Prometheus + Grafana | Metrics & dashboards |
| Tracing | OpenTelemetry + Jaeger | Distributed tracing |
| Logging | Structured JSON | Application logs |

## Scalability Considerations

### Horizontal Scaling

- API pods scale based on CPU/memory/custom metrics
- Database read replicas for read scaling
- ClickHouse cluster scales by adding nodes
- S3 scales automatically

### Vertical Scaling

- RDS instance class upgrades
- EKS node instance types
- ClickHouse node resources

### Performance Optimizations

- Connection pooling (PostgreSQL, ClickHouse)
- Query result caching
- Response compression
- Efficient pagination
- Async I/O throughout

## Reliability Features

- Multi-AZ deployment
- Auto-healing pods
- Circuit breakers for external dependencies
- Retry with exponential backoff
- Graceful degradation
- Load shedding under pressure
- Health checks at all layers
