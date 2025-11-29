# LLM-Research-Lab Architecture - Part 1: System Design & Infrastructure

> **SPARC Phase 3: Architecture (1 of 2)**
> Part of the LLM DevOps Ecosystem

---

## 1. Architecture Overview

### 1.1 System Context

```
┌─────────────────────────────────────────────────────────────────────────────────┐
│                           LLM DevOps Ecosystem                                   │
│  ┌─────────────────┐  ┌─────────────────┐  ┌─────────────────┐  ┌─────────────┐ │
│  │  LLM-Test-Bench │  │LLM-Analytics-Hub│  │   LLM-Registry  │  │LLM-Data-Vault│ │
│  │   (Benchmarks)  │  │  (Visualization)│  │   (Artifacts)   │  │(Data Gov.)  │ │
│  └────────┬────────┘  └────────┬────────┘  └────────┬────────┘  └──────┬──────┘ │
│           │                    │                    │                   │        │
│           └────────────────────┼────────────────────┼───────────────────┘        │
│                                │                    │                            │
│                    ┌───────────▼────────────────────▼───────────┐                │
│                    │         LLM-Research-Lab                    │                │
│                    │  ┌─────────────────────────────────────┐   │                │
│                    │  │           API Gateway               │   │                │
│                    │  └─────────────────────────────────────┘   │                │
│                    │  ┌───────────┐ ┌───────────┐ ┌──────────┐  │                │
│                    │  │Experiment │ │  Metric   │ │ Dataset  │  │                │
│                    │  │ Tracking  │ │Benchmarking│ │Versioning│  │                │
│                    │  └───────────┘ └───────────┘ └──────────┘  │                │
│                    │  ┌───────────┐ ┌───────────┐ ┌──────────┐  │                │
│                    │  │Repro-     │ │ Workflow  │ │Statistical│  │                │
│                    │  │ducibility │ │Orchestrate│ │ Analysis │  │                │
│                    │  └───────────┘ └───────────┘ └──────────┘  │                │
│                    └────────────────────────────────────────────┘                │
└─────────────────────────────────────────────────────────────────────────────────┘
                                        │
                    ┌───────────────────┼───────────────────┐
                    │                   │                   │
              ┌─────▼─────┐      ┌──────▼──────┐     ┌──────▼──────┐
              │    AI     │      │    Data     │      │   MLOps    │
              │Researchers│      │ Scientists  │      │  Engineers │
              └───────────┘      └─────────────┘      └────────────┘
```

### 1.2 Architecture Principles

| Principle | Description | Implementation |
|-----------|-------------|----------------|
| **Microservices** | Loosely coupled, independently deployable services | Each domain (Experiment, Metric, Dataset) is a separate service |
| **Event-Driven** | Asynchronous communication via events | Apache Kafka for event streaming, webhooks for integrations |
| **API-First** | Contract-driven development | OpenAPI 3.1 specs, gRPC for internal services |
| **Cloud-Native** | Designed for Kubernetes deployment | Helm charts, operators, horizontal scaling |
| **Zero-Trust** | Never trust, always verify | mTLS, JWT tokens, RBAC at every layer |
| **Observability** | Full visibility into system behavior | OpenTelemetry, structured logging, distributed tracing |
| **Immutability** | Data and artifacts are immutable | Content-addressable storage, append-only logs |
| **Reproducibility** | Every experiment can be replayed | Complete state capture, deterministic execution |

### 1.3 Technology Stack

```yaml
# Core Technology Decisions
runtime:
  language: Rust 1.75+
  async_runtime: Tokio
  web_framework: Axum
  grpc: Tonic

data:
  primary_database: PostgreSQL 16
  time_series: ClickHouse
  cache: Redis 7
  search: Meilisearch
  object_storage: S3-compatible (MinIO/AWS S3)

messaging:
  event_streaming: Apache Kafka 3.6
  task_queue: Redis Streams
  pubsub: NATS

orchestration:
  container: Kubernetes 1.28+
  service_mesh: Linkerd (optional)
  secrets: HashiCorp Vault
  config: Kubernetes ConfigMaps + Sealed Secrets

observability:
  metrics: Prometheus + Grafana
  logging: Vector + Loki
  tracing: Jaeger (OpenTelemetry)
  profiling: Pyroscope

ci_cd:
  pipelines: GitHub Actions
  artifacts: Harbor Registry
  iac: Terraform + Pulumi
  gitops: ArgoCD
```

---

## 2. Service Architecture

### 2.1 Service Decomposition

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                              API Layer                                       │
│  ┌─────────────────────────────────────────────────────────────────────┐    │
│  │                      API Gateway (Kong/Envoy)                        │    │
│  │  • Rate Limiting  • Auth  • Routing  • TLS Termination  • CORS      │    │
│  └─────────────────────────────────────────────────────────────────────┘    │
└─────────────────────────────────────────────────────────────────────────────┘
                                      │
        ┌─────────────────────────────┼─────────────────────────────┐
        │                             │                             │
        ▼                             ▼                             ▼
┌───────────────┐           ┌───────────────┐           ┌───────────────┐
│  Experiment   │           │    Metric     │           │    Dataset    │
│   Service     │           │   Service     │           │   Service     │
│               │           │               │           │               │
│ • CRUD Exp    │           │ • Metric Reg  │           │ • Versioning  │
│ • Run Mgmt    │           │ • Benchmarks  │           │ • Transforms  │
│ • Artifacts   │           │ • Aggregation │           │ • Lineage     │
│ • Comparison  │           │ • Statistics  │           │ • Streaming   │
└───────┬───────┘           └───────┬───────┘           └───────┬───────┘
        │                           │                           │
        └───────────────────────────┼───────────────────────────┘
                                    │
        ┌─────────────────────────────┼─────────────────────────────┐
        │                             │                             │
        ▼                             ▼                             ▼
┌───────────────┐           ┌───────────────┐           ┌───────────────┐
│Reproducibility│           │   Workflow    │           │  Integration  │
│   Service     │           │   Service     │           │   Service     │
│               │           │               │           │               │
│ • State Cap   │           │ • DAG Exec    │           │ • Test-Bench  │
│ • Validation  │           │ • Scheduling  │           │ • Analytics   │
│ • Replay      │           │ • Checkpoint  │           │ • Registry    │
│ • Certs       │           │ • Recovery    │           │ • Data-Vault  │
└───────────────┘           └───────────────┘           └───────────────┘
                                    │
                                    ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│                           Shared Services                                    │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────┐ │
│  │   Auth/     │  │   Event     │  │  Storage    │  │    Notification     │ │
│  │   AuthZ     │  │   Bus       │  │  Service    │  │      Service        │ │
│  └─────────────┘  └─────────────┘  └─────────────┘  └─────────────────────┘ │
└─────────────────────────────────────────────────────────────────────────────┘
```

### 2.2 Service Specifications

#### 2.2.1 Experiment Service

```yaml
service:
  name: experiment-service
  version: 1.0.0
  port: 8001
  grpc_port: 9001

responsibilities:
  - Experiment CRUD operations
  - Run lifecycle management
  - Artifact storage coordination
  - Environment capture
  - Run comparison and analysis
  - Lineage tracking

dependencies:
  internal:
    - storage-service
    - event-bus
    - auth-service
  external:
    - postgresql (primary store)
    - redis (caching)
    - s3 (artifacts)
    - kafka (events)

api_endpoints:
  rest:
    - POST   /api/v1/experiments
    - GET    /api/v1/experiments/{id}
    - PUT    /api/v1/experiments/{id}
    - DELETE /api/v1/experiments/{id}
    - GET    /api/v1/experiments
    - POST   /api/v1/experiments/{id}/runs
    - GET    /api/v1/experiments/{id}/runs
    - GET    /api/v1/runs/{id}
    - PUT    /api/v1/runs/{id}/status
    - POST   /api/v1/runs/{id}/metrics
    - POST   /api/v1/runs/{id}/artifacts
    - GET    /api/v1/runs/{id}/artifacts
    - POST   /api/v1/runs/compare
    - GET    /api/v1/experiments/{id}/lineage

  grpc:
    - ExperimentService.CreateExperiment
    - ExperimentService.GetExperiment
    - ExperimentService.StartRun
    - ExperimentService.StreamMetrics
    - ExperimentService.UploadArtifact

scaling:
  min_replicas: 2
  max_replicas: 10
  cpu_threshold: 70%
  memory_threshold: 80%

resources:
  requests:
    cpu: 500m
    memory: 512Mi
  limits:
    cpu: 2000m
    memory: 2Gi

health:
  liveness: /health/live
  readiness: /health/ready
  startup: /health/startup
```

#### 2.2.2 Metric Service

```yaml
service:
  name: metric-service
  version: 1.0.0
  port: 8002
  grpc_port: 9002

responsibilities:
  - Metric definition registry
  - Benchmark execution
  - Statistical analysis
  - Metric aggregation
  - Cross-model comparison

dependencies:
  internal:
    - experiment-service
    - storage-service
    - event-bus
  external:
    - postgresql (definitions)
    - clickhouse (time-series)
    - redis (caching)
    - kafka (events)

api_endpoints:
  rest:
    - POST   /api/v1/metrics
    - GET    /api/v1/metrics/{id}
    - GET    /api/v1/metrics
    - POST   /api/v1/benchmarks
    - GET    /api/v1/benchmarks/{id}
    - GET    /api/v1/benchmarks/{id}/status
    - GET    /api/v1/benchmarks/{id}/results
    - POST   /api/v1/benchmarks/compare
    - POST   /api/v1/statistics/test
    - GET    /api/v1/statistics/aggregations

  grpc:
    - MetricService.RegisterMetric
    - MetricService.ComputeMetric
    - MetricService.RunBenchmark
    - MetricService.StreamResults

scaling:
  min_replicas: 2
  max_replicas: 20
  cpu_threshold: 60%

resources:
  requests:
    cpu: 1000m
    memory: 1Gi
  limits:
    cpu: 4000m
    memory: 8Gi
```

#### 2.2.3 Dataset Service

```yaml
service:
  name: dataset-service
  version: 1.0.0
  port: 8003
  grpc_port: 9003

responsibilities:
  - Dataset registration and versioning
  - Content-addressable storage
  - Data transformation pipelines
  - Lineage graph management
  - Data streaming

dependencies:
  internal:
    - storage-service
    - integration-service (data-vault)
    - event-bus
  external:
    - postgresql (metadata)
    - s3 (content)
    - redis (caching)

api_endpoints:
  rest:
    - POST   /api/v1/datasets
    - GET    /api/v1/datasets/{id}
    - GET    /api/v1/datasets
    - POST   /api/v1/datasets/{id}/versions
    - GET    /api/v1/datasets/{id}/versions
    - GET    /api/v1/datasets/{id}/versions/{version}
    - POST   /api/v1/datasets/{id}/splits
    - GET    /api/v1/datasets/{id}/lineage
    - GET    /api/v1/datasets/{id}/versions/{version}/stream
    - POST   /api/v1/datasets/{id}/versions/compare

  grpc:
    - DatasetService.RegisterDataset
    - DatasetService.CreateVersion
    - DatasetService.StreamData
    - DatasetService.GetLineage

scaling:
  min_replicas: 2
  max_replicas: 15

resources:
  requests:
    cpu: 500m
    memory: 1Gi
  limits:
    cpu: 2000m
    memory: 4Gi
```

#### 2.2.4 Workflow Service

```yaml
service:
  name: workflow-service
  version: 1.0.0
  port: 8004
  grpc_port: 9004

responsibilities:
  - Workflow definition management
  - DAG execution engine
  - Step scheduling and coordination
  - Checkpoint management
  - Recovery and retry handling

dependencies:
  internal:
    - experiment-service
    - metric-service
    - dataset-service
    - event-bus
  external:
    - postgresql (workflows, runs)
    - redis (scheduling, locks)
    - s3 (checkpoints)

api_endpoints:
  rest:
    - POST   /api/v1/workflows
    - GET    /api/v1/workflows/{id}
    - GET    /api/v1/workflows
    - POST   /api/v1/workflows/{id}/runs
    - GET    /api/v1/workflows/{id}/runs
    - GET    /api/v1/workflow-runs/{id}
    - POST   /api/v1/workflow-runs/{id}/pause
    - POST   /api/v1/workflow-runs/{id}/resume
    - POST   /api/v1/workflow-runs/{id}/cancel
    - GET    /api/v1/workflow-runs/{id}/logs

scaling:
  min_replicas: 2
  max_replicas: 8

resources:
  requests:
    cpu: 500m
    memory: 512Mi
  limits:
    cpu: 2000m
    memory: 2Gi
```

#### 2.2.5 Reproducibility Service

```yaml
service:
  name: reproducibility-service
  version: 1.0.0
  port: 8005
  grpc_port: 9005

responsibilities:
  - Experiment state capture
  - Reproducibility validation
  - Experiment replay
  - Certificate generation
  - State comparison

dependencies:
  internal:
    - experiment-service
    - dataset-service
    - storage-service
  external:
    - postgresql (states)
    - s3 (snapshots)

api_endpoints:
  rest:
    - POST   /api/v1/states
    - GET    /api/v1/states/{id}
    - GET    /api/v1/runs/{run_id}/state
    - POST   /api/v1/states/{id}/validate
    - POST   /api/v1/states/{id}/replay
    - POST   /api/v1/states/compare
    - POST   /api/v1/certificates
    - GET    /api/v1/certificates/{id}

scaling:
  min_replicas: 2
  max_replicas: 6
```

#### 2.2.6 Integration Service

```yaml
service:
  name: integration-service
  version: 1.0.0
  port: 8006
  grpc_port: 9006

responsibilities:
  - External system integration
  - Circuit breaker management
  - Retry coordination
  - Webhook handling
  - Event translation

integrations:
  - name: test-bench
    type: rest
    circuit_breaker: true
    retry: exponential

  - name: analytics-hub
    type: rest + streaming
    buffering: true

  - name: registry
    type: grpc
    streaming: true

  - name: data-vault
    type: rest
    auth: oauth2

api_endpoints:
  rest:
    - POST   /api/v1/integrations/{name}/test
    - GET    /api/v1/integrations/{name}/status
    - POST   /api/v1/webhooks
    - GET    /api/v1/webhooks/{id}

scaling:
  min_replicas: 2
  max_replicas: 10
```

---

## 3. Deployment Architecture

### 3.1 Kubernetes Topology

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                         Kubernetes Cluster                                   │
│                                                                              │
│  ┌────────────────────────────────────────────────────────────────────────┐ │
│  │                        Ingress Namespace                                │ │
│  │  ┌──────────────────────────────────────────────────────────────────┐  │ │
│  │  │  Ingress Controller (NGINX/Traefik)                              │  │ │
│  │  │  • TLS Termination                                               │  │ │
│  │  │  • Path-based Routing                                            │  │ │
│  │  │  • Rate Limiting                                                 │  │ │
│  │  └──────────────────────────────────────────────────────────────────┘  │ │
│  └────────────────────────────────────────────────────────────────────────┘ │
│                                      │                                       │
│  ┌────────────────────────────────────────────────────────────────────────┐ │
│  │                      research-lab Namespace                             │ │
│  │                                                                         │ │
│  │  ┌─────────────┐ ┌─────────────┐ ┌─────────────┐ ┌─────────────┐       │ │
│  │  │ experiment  │ │   metric    │ │   dataset   │ │  workflow   │       │ │
│  │  │  Deployment │ │ Deployment  │ │ Deployment  │ │ Deployment  │       │ │
│  │  │  (2-10)     │ │  (2-20)     │ │  (2-15)     │ │  (2-8)      │       │ │
│  │  └─────────────┘ └─────────────┘ └─────────────┘ └─────────────┘       │ │
│  │                                                                         │ │
│  │  ┌─────────────┐ ┌─────────────┐ ┌─────────────────────────────────┐   │ │
│  │  │reproducibil │ │ integration │ │         Shared Services         │   │ │
│  │  │ Deployment  │ │ Deployment  │ │  ┌─────┐ ┌─────┐ ┌──────────┐  │   │ │
│  │  │  (2-6)      │ │  (2-10)     │ │  │Redis│ │NATS │ │ConfigMaps│  │   │ │
│  │  └─────────────┘ └─────────────┘ │  └─────┘ └─────┘ └──────────┘  │   │ │
│  │                                   └─────────────────────────────────┘   │ │
│  └────────────────────────────────────────────────────────────────────────┘ │
│                                                                              │
│  ┌────────────────────────────────────────────────────────────────────────┐ │
│  │                        data Namespace                                   │ │
│  │  ┌─────────────────┐ ┌─────────────────┐ ┌─────────────────┐           │ │
│  │  │   PostgreSQL    │ │   ClickHouse    │ │     Kafka       │           │ │
│  │  │  StatefulSet    │ │  StatefulSet    │ │  StatefulSet    │           │ │
│  │  │  (3 replicas)   │ │  (3 replicas)   │ │  (3 brokers)    │           │ │
│  │  └─────────────────┘ └─────────────────┘ └─────────────────┘           │ │
│  └────────────────────────────────────────────────────────────────────────┘ │
│                                                                              │
│  ┌────────────────────────────────────────────────────────────────────────┐ │
│  │                     observability Namespace                             │ │
│  │  ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌──────────┐     │ │
│  │  │Prometheus│ │ Grafana  │ │  Jaeger  │ │   Loki   │ │ Vector   │     │ │
│  │  └──────────┘ └──────────┘ └──────────┘ └──────────┘ └──────────┘     │ │
│  └────────────────────────────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────────────────────────┘
```

### 3.2 Helm Chart Structure

```
helm/
├── Chart.yaml
├── values.yaml
├── values-dev.yaml
├── values-staging.yaml
├── values-prod.yaml
├── templates/
│   ├── _helpers.tpl
│   ├── namespace.yaml
│   ├── configmap.yaml
│   ├── secrets.yaml
│   ├── serviceaccount.yaml
│   │
│   ├── experiment/
│   │   ├── deployment.yaml
│   │   ├── service.yaml
│   │   ├── hpa.yaml
│   │   ├── pdb.yaml
│   │   └── servicemonitor.yaml
│   │
│   ├── metric/
│   │   ├── deployment.yaml
│   │   ├── service.yaml
│   │   ├── hpa.yaml
│   │   └── servicemonitor.yaml
│   │
│   ├── dataset/
│   │   ├── deployment.yaml
│   │   ├── service.yaml
│   │   ├── hpa.yaml
│   │   └── servicemonitor.yaml
│   │
│   ├── workflow/
│   │   ├── deployment.yaml
│   │   ├── service.yaml
│   │   ├── hpa.yaml
│   │   └── servicemonitor.yaml
│   │
│   ├── reproducibility/
│   │   ├── deployment.yaml
│   │   ├── service.yaml
│   │   └── hpa.yaml
│   │
│   ├── integration/
│   │   ├── deployment.yaml
│   │   ├── service.yaml
│   │   └── hpa.yaml
│   │
│   ├── ingress.yaml
│   ├── networkpolicy.yaml
│   └── podsecuritypolicy.yaml
│
└── charts/
    ├── postgresql/
    ├── clickhouse/
    ├── kafka/
    └── redis/
```

### 3.3 Resource Specifications

```yaml
# values-prod.yaml
global:
  environment: production
  imageRegistry: registry.example.com
  imagePullSecrets:
    - name: regcred

experimentService:
  replicaCount: 3
  image:
    repository: llm-research-lab/experiment-service
    tag: "1.0.0"
  resources:
    requests:
      cpu: 500m
      memory: 512Mi
    limits:
      cpu: 2000m
      memory: 2Gi
  autoscaling:
    enabled: true
    minReplicas: 2
    maxReplicas: 10
    targetCPUUtilizationPercentage: 70
    targetMemoryUtilizationPercentage: 80
  podDisruptionBudget:
    minAvailable: 1
  affinity:
    podAntiAffinity:
      preferredDuringSchedulingIgnoredDuringExecution:
        - weight: 100
          podAffinityTerm:
            labelSelector:
              matchLabels:
                app: experiment-service
            topologyKey: kubernetes.io/hostname
  topologySpreadConstraints:
    - maxSkew: 1
      topologyKey: topology.kubernetes.io/zone
      whenUnsatisfiable: ScheduleAnyway
      labelSelector:
        matchLabels:
          app: experiment-service

metricService:
  replicaCount: 3
  resources:
    requests:
      cpu: 1000m
      memory: 1Gi
    limits:
      cpu: 4000m
      memory: 8Gi
  autoscaling:
    enabled: true
    minReplicas: 2
    maxReplicas: 20
    # Custom metric for benchmark queue depth
    metrics:
      - type: External
        external:
          metric:
            name: benchmark_queue_depth
          target:
            type: AverageValue
            averageValue: 10

datasetService:
  replicaCount: 3
  resources:
    requests:
      cpu: 500m
      memory: 1Gi
    limits:
      cpu: 2000m
      memory: 4Gi
  persistence:
    enabled: true
    size: 100Gi
    storageClass: fast-ssd

workflowService:
  replicaCount: 2
  resources:
    requests:
      cpu: 500m
      memory: 512Mi
    limits:
      cpu: 2000m
      memory: 2Gi

postgresql:
  architecture: replication
  primary:
    persistence:
      size: 500Gi
      storageClass: fast-ssd
    resources:
      requests:
        cpu: 2000m
        memory: 4Gi
      limits:
        cpu: 4000m
        memory: 8Gi
  readReplicas:
    replicaCount: 2
    persistence:
      size: 500Gi

clickhouse:
  shards: 3
  replicaCount: 2
  persistence:
    size: 1Ti
    storageClass: fast-ssd
  resources:
    requests:
      cpu: 4000m
      memory: 16Gi
    limits:
      cpu: 8000m
      memory: 32Gi

kafka:
  replicaCount: 3
  persistence:
    size: 500Gi
  resources:
    requests:
      cpu: 2000m
      memory: 4Gi

redis:
  architecture: replication
  master:
    persistence:
      size: 50Gi
  replica:
    replicaCount: 2
```

### 3.4 Network Policies

```yaml
# NetworkPolicy for experiment-service
apiVersion: networking.k8s.io/v1
kind: NetworkPolicy
metadata:
  name: experiment-service-network-policy
  namespace: research-lab
spec:
  podSelector:
    matchLabels:
      app: experiment-service
  policyTypes:
    - Ingress
    - Egress
  ingress:
    # Allow from ingress controller
    - from:
        - namespaceSelector:
            matchLabels:
              name: ingress
          podSelector:
            matchLabels:
              app: ingress-nginx
      ports:
        - protocol: TCP
          port: 8001
        - protocol: TCP
          port: 9001
    # Allow from other services in namespace
    - from:
        - podSelector:
            matchLabels:
              app.kubernetes.io/part-of: research-lab
      ports:
        - protocol: TCP
          port: 9001
    # Allow Prometheus scraping
    - from:
        - namespaceSelector:
            matchLabels:
              name: observability
          podSelector:
            matchLabels:
              app: prometheus
      ports:
        - protocol: TCP
          port: 9090
  egress:
    # Allow to PostgreSQL
    - to:
        - namespaceSelector:
            matchLabels:
              name: data
          podSelector:
            matchLabels:
              app: postgresql
      ports:
        - protocol: TCP
          port: 5432
    # Allow to Redis
    - to:
        - namespaceSelector:
            matchLabels:
              name: data
          podSelector:
            matchLabels:
              app: redis
      ports:
        - protocol: TCP
          port: 6379
    # Allow to Kafka
    - to:
        - namespaceSelector:
            matchLabels:
              name: data
          podSelector:
            matchLabels:
              app: kafka
      ports:
        - protocol: TCP
          port: 9092
    # Allow to S3 (external)
    - to:
        - ipBlock:
            cidr: 0.0.0.0/0
      ports:
        - protocol: TCP
          port: 443
    # Allow DNS
    - to:
        - namespaceSelector: {}
          podSelector:
            matchLabels:
              k8s-app: kube-dns
      ports:
        - protocol: UDP
          port: 53
```

---

## 4. Data Architecture

### 4.1 Database Schema Design

#### 4.1.1 PostgreSQL Schema

```sql
-- Schema: research_lab

-- Enable required extensions
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";
CREATE EXTENSION IF NOT EXISTS "pgcrypto";
CREATE EXTENSION IF NOT EXISTS "pg_trgm";

-- ============================================
-- EXPERIMENTS DOMAIN
-- ============================================

CREATE TABLE experiments (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    name VARCHAR(256) NOT NULL,
    description TEXT,
    hypothesis TEXT,
    owner_id UUID NOT NULL,
    status VARCHAR(32) NOT NULL DEFAULT 'draft',
    config JSONB NOT NULL DEFAULT '{}',
    metadata JSONB NOT NULL DEFAULT '{}',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    archived_at TIMESTAMPTZ,

    CONSTRAINT experiments_name_check CHECK (char_length(name) >= 1),
    CONSTRAINT experiments_status_check CHECK (
        status IN ('draft', 'active', 'paused', 'completed', 'archived', 'failed')
    )
);

CREATE INDEX idx_experiments_owner_id ON experiments(owner_id);
CREATE INDEX idx_experiments_status ON experiments(status);
CREATE INDEX idx_experiments_created_at ON experiments(created_at DESC);
CREATE INDEX idx_experiments_name_trgm ON experiments USING gin(name gin_trgm_ops);
CREATE INDEX idx_experiments_metadata ON experiments USING gin(metadata jsonb_path_ops);

CREATE TABLE experiment_tags (
    experiment_id UUID NOT NULL REFERENCES experiments(id) ON DELETE CASCADE,
    tag VARCHAR(128) NOT NULL,
    PRIMARY KEY (experiment_id, tag)
);

CREATE INDEX idx_experiment_tags_tag ON experiment_tags(tag);

CREATE TABLE experiment_collaborators (
    experiment_id UUID NOT NULL REFERENCES experiments(id) ON DELETE CASCADE,
    user_id UUID NOT NULL,
    role VARCHAR(32) NOT NULL DEFAULT 'viewer',
    added_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (experiment_id, user_id),

    CONSTRAINT collaborators_role_check CHECK (role IN ('viewer', 'editor', 'admin'))
);

CREATE TABLE experiment_runs (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    experiment_id UUID NOT NULL REFERENCES experiments(id) ON DELETE CASCADE,
    run_number BIGINT NOT NULL,
    name VARCHAR(256),
    status VARCHAR(32) NOT NULL DEFAULT 'pending',
    parameters JSONB NOT NULL DEFAULT '{}',
    environment JSONB NOT NULL DEFAULT '{}',
    parent_run_id UUID REFERENCES experiment_runs(id),
    started_at TIMESTAMPTZ,
    ended_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    created_by UUID NOT NULL,
    error JSONB,

    CONSTRAINT runs_status_check CHECK (
        status IN ('pending', 'queued', 'running', 'completed', 'failed', 'cancelled', 'timed_out')
    ),
    UNIQUE (experiment_id, run_number)
);

CREATE INDEX idx_runs_experiment_id ON experiment_runs(experiment_id);
CREATE INDEX idx_runs_status ON experiment_runs(status);
CREATE INDEX idx_runs_created_at ON experiment_runs(created_at DESC);
CREATE INDEX idx_runs_parent_run_id ON experiment_runs(parent_run_id);

CREATE TABLE run_tags (
    run_id UUID NOT NULL REFERENCES experiment_runs(id) ON DELETE CASCADE,
    tag VARCHAR(128) NOT NULL,
    PRIMARY KEY (run_id, tag)
);

CREATE TABLE run_artifacts (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    run_id UUID NOT NULL REFERENCES experiment_runs(id) ON DELETE CASCADE,
    name VARCHAR(512) NOT NULL,
    artifact_type VARCHAR(64) NOT NULL,
    content_hash VARCHAR(128) NOT NULL,
    size_bytes BIGINT NOT NULL,
    storage_uri TEXT NOT NULL,
    metadata JSONB NOT NULL DEFAULT '{}',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_artifacts_run_id ON run_artifacts(run_id);
CREATE INDEX idx_artifacts_content_hash ON run_artifacts(content_hash);
CREATE INDEX idx_artifacts_type ON run_artifacts(artifact_type);

-- ============================================
-- METRICS DOMAIN
-- ============================================

CREATE TABLE metric_definitions (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    name VARCHAR(256) NOT NULL,
    description TEXT,
    version_major INTEGER NOT NULL,
    version_minor INTEGER NOT NULL,
    version_patch INTEGER NOT NULL,
    metric_type VARCHAR(64) NOT NULL,
    properties JSONB NOT NULL DEFAULT '{}',
    parameters_schema JSONB,
    created_by UUID NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    deprecated BOOLEAN NOT NULL DEFAULT FALSE,
    deprecation_message TEXT,

    UNIQUE (name, version_major, version_minor, version_patch)
);

CREATE INDEX idx_metrics_name ON metric_definitions(name);
CREATE INDEX idx_metrics_type ON metric_definitions(metric_type);
CREATE INDEX idx_metrics_deprecated ON metric_definitions(deprecated);

CREATE TABLE benchmark_suites (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    name VARCHAR(256) NOT NULL,
    description TEXT,
    metrics JSONB NOT NULL DEFAULT '[]',
    test_cases JSONB NOT NULL DEFAULT '[]',
    aggregation_config JSONB NOT NULL DEFAULT '{}',
    created_by UUID NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE benchmark_jobs (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    suite_id UUID NOT NULL REFERENCES benchmark_suites(id),
    run_id UUID REFERENCES experiment_runs(id),
    model_id VARCHAR(256) NOT NULL,
    model_config JSONB NOT NULL DEFAULT '{}',
    status VARCHAR(32) NOT NULL DEFAULT 'pending',
    progress DECIMAL(5, 2) NOT NULL DEFAULT 0,
    results JSONB,
    started_at TIMESTAMPTZ,
    completed_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    error JSONB,

    CONSTRAINT benchmark_status_check CHECK (
        status IN ('pending', 'running', 'completed', 'failed', 'cancelled')
    )
);

CREATE INDEX idx_benchmark_jobs_suite ON benchmark_jobs(suite_id);
CREATE INDEX idx_benchmark_jobs_status ON benchmark_jobs(status);
CREATE INDEX idx_benchmark_jobs_model ON benchmark_jobs(model_id);

-- ============================================
-- DATASETS DOMAIN
-- ============================================

CREATE TABLE datasets (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    name VARCHAR(256) NOT NULL,
    description TEXT,
    schema JSONB NOT NULL,
    governance JSONB NOT NULL DEFAULT '{}',
    owner_id UUID NOT NULL,
    metadata JSONB NOT NULL DEFAULT '{}',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_datasets_owner ON datasets(owner_id);
CREATE INDEX idx_datasets_name_trgm ON datasets USING gin(name gin_trgm_ops);

CREATE TABLE dataset_tags (
    dataset_id UUID NOT NULL REFERENCES datasets(id) ON DELETE CASCADE,
    tag VARCHAR(128) NOT NULL,
    PRIMARY KEY (dataset_id, tag)
);

CREATE TABLE dataset_versions (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    dataset_id UUID NOT NULL REFERENCES datasets(id) ON DELETE CASCADE,
    version_number BIGINT NOT NULL,
    content_hash VARCHAR(128) NOT NULL,
    parent_version_id UUID REFERENCES dataset_versions(id),
    transform JSONB,
    statistics JSONB NOT NULL DEFAULT '{}',
    manifest JSONB NOT NULL,
    message TEXT,
    created_by UUID NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    UNIQUE (dataset_id, version_number)
);

CREATE INDEX idx_dataset_versions_dataset ON dataset_versions(dataset_id);
CREATE INDEX idx_dataset_versions_hash ON dataset_versions(content_hash);
CREATE INDEX idx_dataset_versions_parent ON dataset_versions(parent_version_id);

CREATE TABLE dataset_version_tags (
    version_id UUID NOT NULL REFERENCES dataset_versions(id) ON DELETE CASCADE,
    tag VARCHAR(128) NOT NULL,
    PRIMARY KEY (version_id, tag)
);

-- ============================================
-- WORKFLOWS DOMAIN
-- ============================================

CREATE TABLE workflows (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    name VARCHAR(256) NOT NULL,
    description TEXT,
    version_major INTEGER NOT NULL,
    version_minor INTEGER NOT NULL,
    version_patch INTEGER NOT NULL,
    definition JSONB NOT NULL,
    parameters JSONB NOT NULL DEFAULT '[]',
    triggers JSONB NOT NULL DEFAULT '[]',
    created_by UUID NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    UNIQUE (name, version_major, version_minor, version_patch)
);

CREATE INDEX idx_workflows_name ON workflows(name);

CREATE TABLE workflow_runs (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    workflow_id UUID NOT NULL REFERENCES workflows(id),
    workflow_version VARCHAR(32) NOT NULL,
    status VARCHAR(32) NOT NULL DEFAULT 'pending',
    parameters JSONB NOT NULL DEFAULT '{}',
    step_states JSONB NOT NULL DEFAULT '{}',
    outputs JSONB NOT NULL DEFAULT '{}',
    started_at TIMESTAMPTZ,
    ended_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    created_by UUID NOT NULL,
    error JSONB,

    CONSTRAINT workflow_run_status_check CHECK (
        status IN ('pending', 'running', 'paused', 'completed', 'failed', 'cancelled')
    )
);

CREATE INDEX idx_workflow_runs_workflow ON workflow_runs(workflow_id);
CREATE INDEX idx_workflow_runs_status ON workflow_runs(status);

CREATE TABLE workflow_checkpoints (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    run_id UUID NOT NULL REFERENCES workflow_runs(id) ON DELETE CASCADE,
    state JSONB NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_checkpoints_run ON workflow_checkpoints(run_id);
CREATE INDEX idx_checkpoints_created ON workflow_checkpoints(created_at DESC);

-- ============================================
-- REPRODUCIBILITY DOMAIN
-- ============================================

CREATE TABLE experiment_states (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    experiment_id UUID NOT NULL REFERENCES experiments(id),
    run_id UUID NOT NULL REFERENCES experiment_runs(id),
    environment JSONB NOT NULL,
    code_state JSONB NOT NULL,
    data_state JSONB NOT NULL,
    configuration JSONB NOT NULL,
    random_state JSONB NOT NULL DEFAULT '{}',
    checksum VARCHAR(128) NOT NULL,
    captured_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_states_experiment ON experiment_states(experiment_id);
CREATE INDEX idx_states_run ON experiment_states(run_id);

CREATE TABLE reproducibility_certificates (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    state_id UUID NOT NULL REFERENCES experiment_states(id),
    experiment_id UUID NOT NULL REFERENCES experiments(id),
    run_id UUID NOT NULL REFERENCES experiment_runs(id),
    validation_report JSONB NOT NULL,
    environment_hash VARCHAR(128) NOT NULL,
    code_hash VARCHAR(128) NOT NULL,
    data_hash VARCHAR(128) NOT NULL,
    configuration_hash VARCHAR(128) NOT NULL,
    results_hash VARCHAR(128) NOT NULL,
    signature TEXT NOT NULL,
    issued_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_certificates_experiment ON reproducibility_certificates(experiment_id);
CREATE INDEX idx_certificates_run ON reproducibility_certificates(run_id);

-- ============================================
-- LINEAGE DOMAIN
-- ============================================

CREATE TABLE lineage_nodes (
    id VARCHAR(256) PRIMARY KEY,
    node_type VARCHAR(64) NOT NULL,
    name VARCHAR(512),
    version VARCHAR(64),
    metadata JSONB NOT NULL DEFAULT '{}',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_lineage_nodes_type ON lineage_nodes(node_type);

CREATE TABLE lineage_edges (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    source_id VARCHAR(256) NOT NULL REFERENCES lineage_nodes(id) ON DELETE CASCADE,
    target_id VARCHAR(256) NOT NULL REFERENCES lineage_nodes(id) ON DELETE CASCADE,
    edge_type VARCHAR(64) NOT NULL,
    metadata JSONB NOT NULL DEFAULT '{}',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    UNIQUE (source_id, target_id, edge_type)
);

CREATE INDEX idx_lineage_edges_source ON lineage_edges(source_id);
CREATE INDEX idx_lineage_edges_target ON lineage_edges(target_id);
CREATE INDEX idx_lineage_edges_type ON lineage_edges(edge_type);

-- ============================================
-- AUDIT LOG
-- ============================================

CREATE TABLE audit_log (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    timestamp TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    user_id UUID,
    action VARCHAR(64) NOT NULL,
    resource_type VARCHAR(64) NOT NULL,
    resource_id UUID,
    old_value JSONB,
    new_value JSONB,
    ip_address INET,
    user_agent TEXT,
    request_id UUID
);

CREATE INDEX idx_audit_timestamp ON audit_log(timestamp DESC);
CREATE INDEX idx_audit_user ON audit_log(user_id);
CREATE INDEX idx_audit_resource ON audit_log(resource_type, resource_id);

-- Partitioning for audit log (by month)
-- In production, set up automated partition management

-- ============================================
-- FUNCTIONS & TRIGGERS
-- ============================================

-- Auto-update updated_at timestamp
CREATE OR REPLACE FUNCTION update_updated_at_column()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ language 'plpgsql';

CREATE TRIGGER update_experiments_updated_at
    BEFORE UPDATE ON experiments
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_datasets_updated_at
    BEFORE UPDATE ON datasets
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_workflows_updated_at
    BEFORE UPDATE ON workflows
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_benchmark_suites_updated_at
    BEFORE UPDATE ON benchmark_suites
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

-- Auto-increment run number
CREATE OR REPLACE FUNCTION get_next_run_number(exp_id UUID)
RETURNS BIGINT AS $$
DECLARE
    next_num BIGINT;
BEGIN
    SELECT COALESCE(MAX(run_number), 0) + 1 INTO next_num
    FROM experiment_runs
    WHERE experiment_id = exp_id;
    RETURN next_num;
END;
$$ LANGUAGE plpgsql;

-- Auto-increment dataset version number
CREATE OR REPLACE FUNCTION get_next_version_number(ds_id UUID)
RETURNS BIGINT AS $$
DECLARE
    next_num BIGINT;
BEGIN
    SELECT COALESCE(MAX(version_number), 0) + 1 INTO next_num
    FROM dataset_versions
    WHERE dataset_id = ds_id;
    RETURN next_num;
END;
$$ LANGUAGE plpgsql;
```

#### 4.1.2 ClickHouse Schema (Time-Series Metrics)

```sql
-- Database for time-series metrics
CREATE DATABASE IF NOT EXISTS research_lab;

-- Run metrics table (main time-series data)
CREATE TABLE research_lab.run_metrics
(
    run_id UUID,
    metric_name LowCardinality(String),
    value Float64,
    step UInt64,
    timestamp DateTime64(3),
    context Map(String, String),

    INDEX idx_metric_name metric_name TYPE bloom_filter GRANULARITY 4,
    INDEX idx_step step TYPE minmax GRANULARITY 4
)
ENGINE = MergeTree()
PARTITION BY toYYYYMM(timestamp)
ORDER BY (run_id, metric_name, timestamp)
TTL timestamp + INTERVAL 2 YEAR
SETTINGS index_granularity = 8192;

-- Materialized view for metric aggregations per run
CREATE MATERIALIZED VIEW research_lab.run_metrics_agg
ENGINE = SummingMergeTree()
PARTITION BY toYYYYMM(timestamp)
ORDER BY (run_id, metric_name)
AS SELECT
    run_id,
    metric_name,
    min(timestamp) as first_timestamp,
    max(timestamp) as last_timestamp,
    count() as sample_count,
    min(value) as min_value,
    max(value) as max_value,
    sum(value) as sum_value,
    avg(value) as avg_value
FROM research_lab.run_metrics
GROUP BY run_id, metric_name;

-- Benchmark results table
CREATE TABLE research_lab.benchmark_results
(
    job_id UUID,
    test_case_id UUID,
    metric_name LowCardinality(String),
    value Float64,
    model_id LowCardinality(String),
    timestamp DateTime64(3),
    latency_ms UInt64,
    tokens_used UInt32,
    details String, -- JSON

    INDEX idx_model model_id TYPE bloom_filter GRANULARITY 4
)
ENGINE = MergeTree()
PARTITION BY toYYYYMM(timestamp)
ORDER BY (job_id, metric_name, timestamp)
TTL timestamp + INTERVAL 1 YEAR;

-- System metrics table
CREATE TABLE research_lab.system_metrics
(
    service LowCardinality(String),
    instance String,
    metric_name LowCardinality(String),
    value Float64,
    timestamp DateTime64(3),
    labels Map(String, String)
)
ENGINE = MergeTree()
PARTITION BY toYYYYMM(timestamp)
ORDER BY (service, metric_name, timestamp)
TTL timestamp + INTERVAL 90 DAY;

-- Query patterns table for analytics
CREATE TABLE research_lab.query_patterns
(
    user_id UUID,
    query_type LowCardinality(String),
    resource_type LowCardinality(String),
    resource_id UUID,
    duration_ms UInt32,
    timestamp DateTime64(3)
)
ENGINE = MergeTree()
PARTITION BY toYYYYMM(timestamp)
ORDER BY (query_type, timestamp)
TTL timestamp + INTERVAL 180 DAY;
```

### 4.2 Data Flow Architecture

```
┌─────────────────────────────────────────────────────────────────────────────────┐
│                              Data Flow Architecture                              │
└─────────────────────────────────────────────────────────────────────────────────┘

                              ┌───────────────────┐
                              │   API Gateway     │
                              └─────────┬─────────┘
                                        │
              ┌─────────────────────────┼─────────────────────────┐
              │                         │                         │
              ▼                         ▼                         ▼
     ┌────────────────┐       ┌────────────────┐       ┌────────────────┐
     │  Write Path    │       │  Read Path     │       │ Stream Path    │
     └───────┬────────┘       └───────┬────────┘       └───────┬────────┘
             │                        │                        │
             ▼                        ▼                        ▼
     ┌────────────────┐       ┌────────────────┐       ┌────────────────┐
     │   Services     │       │     Redis      │       │     Kafka      │
     │ (Validation)   │       │    (Cache)     │       │   (Events)     │
     └───────┬────────┘       └───────┬────────┘       └───────┬────────┘
             │                        │                        │
             ▼                        │                        ▼
     ┌────────────────┐               │                ┌────────────────┐
     │   PostgreSQL   │◄──────────────┘                │  Consumers     │
     │   (Primary)    │                                │  (Workers)     │
     └───────┬────────┘                                └───────┬────────┘
             │                                                 │
             ├──────────────────────┐                         │
             │                      │                         │
             ▼                      ▼                         ▼
     ┌────────────────┐    ┌────────────────┐         ┌────────────────┐
     │   PostgreSQL   │    │    S3/MinIO    │         │   ClickHouse   │
     │   (Replicas)   │    │  (Artifacts)   │         │ (Time-Series)  │
     └────────────────┘    └────────────────┘         └────────────────┘


Write Path:
1. Request → API Gateway → Service
2. Service validates and transforms
3. Write to PostgreSQL (primary)
4. Publish event to Kafka
5. Invalidate Redis cache
6. Store artifacts to S3

Read Path:
1. Request → API Gateway → Service
2. Check Redis cache
3. Cache miss → Query PostgreSQL replica
4. Populate cache
5. Return response

Stream Path:
1. Event published to Kafka
2. Consumers process asynchronously
3. Update ClickHouse for analytics
4. Trigger notifications/webhooks
5. Update search indices
```

### 4.3 Caching Strategy

```yaml
caching:
  layers:
    # L1: In-process cache (per pod)
    l1:
      type: in-memory
      implementation: moka
      max_size: 100MB
      ttl: 60s

    # L2: Distributed cache
    l2:
      type: redis
      cluster: true
      max_memory: 8GB
      eviction_policy: allkeys-lru

  patterns:
    # Experiment metadata
    experiments:
      key_pattern: "exp:{id}"
      ttl: 300s
      invalidation: write-through

    # Run data
    runs:
      key_pattern: "run:{id}"
      ttl: 180s
      invalidation: write-through

    # Metric definitions
    metrics:
      key_pattern: "metric:{name}:{version}"
      ttl: 3600s
      invalidation: on-update

    # Dataset metadata
    datasets:
      key_pattern: "ds:{id}"
      ttl: 600s
      invalidation: write-through

    # User sessions
    sessions:
      key_pattern: "session:{token}"
      ttl: 3600s
      sliding: true

    # Rate limiting
    rate_limits:
      key_pattern: "rl:{user_id}:{endpoint}"
      ttl: 60s
      type: sliding_window

  invalidation:
    strategies:
      - type: event-driven
        source: kafka
        topic: cache-invalidation

      - type: ttl
        check_interval: 10s

      - type: write-through
        sync: true
```

---

## 5. Storage Architecture

### 5.1 Object Storage Layout

```
s3://research-lab-artifacts/
├── experiments/
│   └── {experiment_id}/
│       └── runs/
│           └── {run_id}/
│               ├── artifacts/
│               │   ├── {artifact_id}/
│               │   │   ├── data              # Actual artifact data
│               │   │   └── metadata.json     # Artifact metadata
│               │   └── manifest.json         # Artifacts manifest
│               ├── logs/
│               │   ├── stdout.log
│               │   ├── stderr.log
│               │   └── metrics.log
│               ├── checkpoints/
│               │   └── {checkpoint_id}.json
│               └── environment.json          # Environment snapshot
│
├── datasets/
│   └── {dataset_id}/
│       └── versions/
│           └── {version_id}/
│               ├── chunks/
│               │   ├── chunk_000000/
│               │   │   └── data.parquet
│               │   ├── chunk_000001/
│               │   │   └── data.parquet
│               │   └── ...
│               ├── manifest.json
│               └── statistics.json
│
├── workflows/
│   └── {workflow_id}/
│       └── runs/
│           └── {run_id}/
│               ├── checkpoints/
│               │   └── {step_id}/
│               │       └── state.json
│               └── logs/
│                   └── {step_id}.log
│
├── reproducibility/
│   └── states/
│       └── {state_id}/
│           ├── environment.json
│           ├── code.tar.gz              # Code snapshot
│           ├── patches/
│           │   └── *.patch
│           └── certificates/
│               └── {certificate_id}.json
│
└── temp/
    └── uploads/
        └── {upload_id}/                  # Temporary upload staging
```

### 5.2 Storage Policies

```yaml
storage_policies:
  artifacts:
    storage_class: STANDARD
    encryption: AES-256
    versioning: enabled
    lifecycle:
      - rule: transition_to_ia
        days: 90
        storage_class: STANDARD_IA
      - rule: transition_to_glacier
        days: 365
        storage_class: GLACIER
      - rule: expire
        days: 2555  # 7 years

  datasets:
    storage_class: STANDARD
    encryption: AES-256
    versioning: enabled
    replication:
      enabled: true
      destination: s3://research-lab-artifacts-replica/
    lifecycle:
      - rule: transition_to_ia
        days: 180
        storage_class: STANDARD_IA

  logs:
    storage_class: STANDARD_IA
    encryption: AES-256
    versioning: disabled
    lifecycle:
      - rule: expire
        days: 90

  temp:
    storage_class: STANDARD
    encryption: AES-256
    versioning: disabled
    lifecycle:
      - rule: expire
        days: 1

  checkpoints:
    storage_class: STANDARD
    encryption: AES-256
    versioning: enabled
    lifecycle:
      - rule: transition_to_ia
        days: 30
      - rule: expire
        days: 365
```

### 5.3 Content-Addressable Storage

```rust
/// Content-addressable storage implementation
pub struct ContentAddressableStore {
    storage: Arc<dyn ObjectStorage>,
    index: Arc<ContentIndex>,
    deduplication: bool,
    compression: CompressionType,
}

impl ContentAddressableStore {
    /// Store content and return content hash
    pub async fn store(&self, data: &[u8]) -> Result<ContentReference, StorageError> {
        // 1. Compute content hash
        let hash = ContentHash::from_bytes(data);

        // 2. Check if content already exists (deduplication)
        if self.deduplication {
            if let Some(reference) = self.index.get(&hash).await? {
                // Increment reference count
                self.index.increment_refs(&hash).await?;
                return Ok(reference);
            }
        }

        // 3. Compress content
        let compressed = self.compress(data)?;

        // 4. Generate storage path
        let path = self.generate_path(&hash);

        // 5. Store to object storage
        self.storage.put(&path, &compressed).await?;

        // 6. Update index
        let reference = ContentReference {
            hash: hash.clone(),
            path,
            size_bytes: data.len() as u64,
            compressed_size: compressed.len() as u64,
            compression: self.compression,
            stored_at: Utc::now(),
        };

        self.index.put(&hash, &reference).await?;

        Ok(reference)
    }

    /// Retrieve content by hash
    pub async fn retrieve(&self, hash: &ContentHash) -> Result<Vec<u8>, StorageError> {
        // 1. Look up in index
        let reference = self.index.get(hash).await?
            .ok_or(StorageError::NotFound)?;

        // 2. Retrieve from object storage
        let compressed = self.storage.get(&reference.path).await?;

        // 3. Decompress
        let data = self.decompress(&compressed, reference.compression)?;

        // 4. Verify hash
        let computed_hash = ContentHash::from_bytes(&data);
        if computed_hash != *hash {
            return Err(StorageError::IntegrityError);
        }

        Ok(data)
    }

    /// Generate hierarchical path from hash
    fn generate_path(&self, hash: &ContentHash) -> String {
        let hex = hash.as_str();
        format!("cas/{}/{}/{}", &hex[0..2], &hex[2..4], hex)
    }
}
```

---

## Document Metadata

| Field | Value |
|-------|-------|
| **Version** | 1.0.0 |
| **Status** | Draft |
| **SPARC Phase** | Architecture (Part 1 of 2) |
| **Created** | 2025-11-28 |
| **Ecosystem** | LLM DevOps |
| **Next Part** | Architecture Part 2: APIs, Security & Observability |

---

*This architecture document is part of the SPARC methodology. Part 2 covers API Design, Security Architecture, Observability, and Operational Procedures.*
