# ADR-002: Polyglot Database Architecture

## Status
Accepted

## Date
2025-01-15

## Context

The LLM Research Lab platform has diverse data storage requirements:

1. **Transactional Data**: Experiments, models, datasets, users
   - ACID compliance required
   - Complex relationships and joins
   - Moderate write volume, high read volume

2. **Time-Series Metrics**: Model performance metrics, latency measurements
   - High write volume (millions of events/day)
   - Time-range queries and aggregations
   - Data retention policies needed

3. **Binary Data**: Datasets, model artifacts, experiment outputs
   - Large file storage (GB-scale individual files)
   - Versioning and lifecycle management
   - Global distribution requirements

## Decision

We implement a **polyglot persistence** architecture with three specialized storage systems:

| Data Type | Storage System | Rationale |
|-----------|---------------|-----------|
| Transactional | PostgreSQL | ACID, rich query capabilities |
| Time-Series | ClickHouse | Columnar storage, fast aggregations |
| Binary/Files | Amazon S3 | Scalable object storage |

### PostgreSQL (Primary Database)

**Use Cases:**
- Experiment configurations and state
- Model registry
- Dataset metadata
- User management and authentication
- Prompt templates

**Configuration:**
```
Instance: db.r6g.xlarge (4 vCPU, 32 GB RAM)
Storage: 500 GB gp3 (3000 IOPS)
Multi-AZ: Enabled
Read Replicas: 1 (same region)
```

**Schema Design Principles:**
- UUID primary keys for distributed ID generation
- JSONB for flexible metadata storage
- Proper indexing strategy (B-tree, GIN for JSONB)
- Foreign key constraints for referential integrity

### ClickHouse (Analytics Database)

**Use Cases:**
- Experiment run metrics
- Model performance time-series
- API usage analytics
- Audit log analytics

**Configuration:**
```
Cluster: 3-node ReplicatedMergeTree
Storage: 2 TB NVMe per node
Replication: 3-way for fault tolerance
```

**Table Design:**
```sql
CREATE TABLE experiment_metrics (
    timestamp DateTime,
    experiment_id UUID,
    run_id UUID,
    metric_name LowCardinality(String),
    metric_value Float64,
    tags Map(String, String)
)
ENGINE = ReplicatedMergeTree('/clickhouse/tables/{shard}/experiment_metrics', '{replica}')
PARTITION BY toYYYYMM(timestamp)
ORDER BY (experiment_id, metric_name, timestamp)
TTL timestamp + INTERVAL 1 YEAR;
```

### Amazon S3 (Object Storage)

**Use Cases:**
- Dataset file storage
- Model artifacts
- Experiment outputs
- Backup storage

**Configuration:**
```
Bucket: llm-research-datasets-prod
Region: us-east-1
Versioning: Enabled
Lifecycle:
  - Transition to IA: 90 days
  - Transition to Glacier: 365 days
  - Expiration: 7 years
```

**Key Structure:**
```
datasets/{dataset_id}/{version}/data.{format}
models/{model_id}/artifacts/
experiments/{experiment_id}/runs/{run_id}/outputs/
backups/postgresql/{date}/
```

## Alternatives Considered

### Single Database (PostgreSQL Only)
- **Pro**: Simpler architecture, single technology
- **Con**: Poor time-series performance, expensive for file storage
- **Decision**: Rejected due to performance constraints

### TimescaleDB Instead of ClickHouse
- **Pro**: PostgreSQL extension, familiar SQL
- **Con**: Less efficient compression, slower aggregations
- **Decision**: Rejected; ClickHouse offers 10-100x better performance

### MongoDB for Transactional Data
- **Pro**: Flexible schema, horizontal scaling
- **Con**: Weaker consistency guarantees, less mature Rust ecosystem
- **Decision**: Rejected; PostgreSQL better fits our consistency requirements

### MinIO Instead of S3
- **Pro**: Self-hosted, S3-compatible
- **Con**: Operational overhead, no global CDN
- **Decision**: Rejected for production; acceptable for development

## Consequences

### Positive
- **Performance**: Each database optimized for its workload
- **Scalability**: Independent scaling of each tier
- **Cost Efficiency**: Right tool for each job
- **Feature Rich**: Best-in-class capabilities per use case

### Negative
- **Complexity**: Three systems to manage and monitor
- **Consistency**: Eventual consistency between systems
- **Transactions**: No cross-database transactions
- **Learning Curve**: Team needs expertise in multiple systems

### Mitigations

**Complexity Management:**
- Infrastructure as Code (Terraform) for all databases
- Unified monitoring through Prometheus/Grafana
- Standardized backup procedures

**Consistency Handling:**
- Saga pattern for cross-database operations
- Idempotent operations with retry logic
- Event sourcing for audit trail

**Operational Procedures:**
- Automated failover for PostgreSQL
- ClickHouse cluster management via ClickHouse Operator
- S3 cross-region replication for disaster recovery

## Data Flow

```
┌─────────────────┐    ┌──────────────────┐    ┌─────────────────┐
│   API Server    │───▶│   PostgreSQL     │    │   ClickHouse    │
│                 │    │  (Transactional) │    │  (Analytics)    │
└────────┬────────┘    └──────────────────┘    └────────▲────────┘
         │                                              │
         │             ┌──────────────────┐             │
         │             │    Amazon S3     │             │
         └────────────▶│  (File Storage)  │             │
                       └──────────────────┘             │
                                                        │
┌─────────────────┐                                     │
│  Metrics Agent  │─────────────────────────────────────┘
│                 │
└─────────────────┘
```

## Implementation Notes

### Connection Pooling
```rust
// PostgreSQL
let pool = PgPoolOptions::new()
    .max_connections(50)
    .min_connections(10)
    .acquire_timeout(Duration::from_secs(3))
    .connect(&database_url)
    .await?;

// ClickHouse
let client = clickhouse::Client::default()
    .with_url(&clickhouse_url)
    .with_database(&clickhouse_db);
```

### Health Checks
```rust
// Check all databases in parallel
let (pg_health, ch_health, s3_health) = tokio::join!(
    check_postgres(&pool),
    check_clickhouse(&client),
    check_s3(&s3_client, &bucket),
);
```

## References
- [PostgreSQL Performance Tuning](https://www.postgresql.org/docs/current/performance-tips.html)
- [ClickHouse MergeTree](https://clickhouse.com/docs/en/engines/table-engines/mergetree-family/mergetree)
- [S3 Best Practices](https://docs.aws.amazon.com/AmazonS3/latest/userguide/optimizing-performance.html)
