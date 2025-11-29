# Health Check System Architecture

## System Overview

```
┌─────────────────────────────────────────────────────────────────┐
│                     Kubernetes Pod                              │
│                                                                 │
│  ┌───────────────────────────────────────────────────────────┐ │
│  │                 LLM Research API                          │ │
│  │                                                           │ │
│  │  ┌─────────────────────────────────────────────────┐     │ │
│  │  │         Health Check Registry                   │     │ │
│  │  │                                                 │     │ │
│  │  │  ┌──────────────┐  ┌──────────────┐  ┌───────┐│     │ │
│  │  │  │  Postgres    │  │ ClickHouse   │  │  S3   ││     │ │
│  │  │  │ HealthCheck  │  │ HealthCheck  │  │ Check ││     │ │
│  │  │  │  (critical)  │  │(non-critical)│  │(crit) ││     │ │
│  │  │  └──────────────┘  └──────────────┘  └───────┘│     │ │
│  │  │                                                 │     │ │
│  │  │  Cache: {                                      │     │ │
│  │  │    "postgres": CachedHealth(TTL=30s),         │     │ │
│  │  │    "clickhouse": CachedHealth(TTL=60s),       │     │ │
│  │  │    "s3": CachedHealth(TTL=45s)                │     │ │
│  │  │  }                                             │     │ │
│  │  └─────────────────────────────────────────────────┘     │ │
│  │                                                           │ │
│  │  ┌─────────────────────────────────────────────────┐     │ │
│  │  │            HTTP Endpoints                       │     │ │
│  │  │                                                 │     │ │
│  │  │  GET /health/live  ────┐                       │     │ │
│  │  │  GET /health/ready ────┤──────> Handlers       │     │ │
│  │  │  GET /health       ────┘                       │     │ │
│  │  └─────────────────────────────────────────────────┘     │ │
│  └───────────────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────────────┘
         ▲              ▲              ▲
         │              │              │
    ┌────┴────┐    ┌────┴────┐   ┌────┴────┐
    │Liveness │    │Readiness│   │  HTTP   │
    │ Probe   │    │  Probe  │   │ Clients │
    │(K8s)    │    │  (K8s)  │   │         │
    └─────────┘    └─────────┘   └─────────┘
```

## Component Interaction Flow

### Liveness Check Flow
```
1. K8s sends GET /health/live
         │
         ▼
2. liveness_handler()
         │
         ▼
3. registry.check_liveness()
         │
         ▼
4. Returns ComponentHealth("application", Healthy)
         │
         ▼
5. HTTP 200 OK with minimal JSON
```

### Readiness Check Flow
```
1. K8s sends GET /health/ready
         │
         ▼
2. readiness_handler()
         │
         ▼
3. registry.check_readiness()
         │
         ▼
4. Filter only critical checks
         │
         ├─> PostgresHealthCheck
         │   ├─> Check cache (30s TTL)
         │   │   ├─> Cache hit? Return cached
         │   │   └─> Cache miss? Execute check
         │   │       ├─> SELECT 1 query (3s timeout)
         │   │       └─> Update cache
         │
         └─> S3HealthCheck
             ├─> Check cache (45s TTL)
             └─> HEAD bucket (5s timeout)
         │
         ▼
5. Aggregate results
   ├─> All healthy? → HTTP 200 OK
   └─> Any unhealthy? → HTTP 503 Service Unavailable
```

### Detailed Health Check Flow
```
1. Client sends GET /health
         │
         ▼
2. health_handler()
         │
         ▼
3. registry.check_all()
         │
         ├─────────────────┬─────────────────┐
         │                 │                 │
         ▼                 ▼                 ▼
   PostgresCheck    ClickHouseCheck    S3Check
         │                 │                 │
   (concurrent execution with futures::join_all)
         │                 │                 │
         ▼                 ▼                 ▼
   Check cache       Check cache       Check cache
         │                 │                 │
   Cache hit?        Cache hit?        Cache hit?
    ├─Yes─┐          ├─Yes─┐          ├─Yes─┐
    │     │          │     │          │     │
    ▼     ▼          ▼     ▼          ▼     ▼
  Return Execute   Return Execute   Return Execute
  cached  check    cached  check    cached  check
         │                 │                 │
         └─────────────────┴─────────────────┘
                         │
                         ▼
4. Combine all results
         │
         ▼
5. Add metadata (version, uptime, timestamp)
         │
         ▼
6. Return JSON with appropriate status code
```

## Caching Strategy

```
Time-based cache with per-component TTL:

t=0s    Request arrives
        ├─> Check cache
        └─> Miss → Execute health check (150ms)
            └─> Store in cache

t=5s    Request arrives
        ├─> Check cache
        └─> Hit → Return cached (2ms)

t=10s   Request arrives
        ├─> Check cache
        └─> Hit → Return cached (2ms)

t=30s   Request arrives (PostgreSQL cache expired)
        ├─> Check cache
        └─> Miss → Execute health check (150ms)
            └─> Update cache

t=45s   Request arrives (S3 cache expired)
        ├─> Check cache
        └─> Miss → Execute health check (150ms)
            └─> Update cache

t=60s   Request arrives (ClickHouse cache expired)
        ├─> Check cache
        └─> Miss → Execute health check (150ms)
            └─> Update cache
```

## State Management

```
HealthCheckState
    │
    └─> Arc<HealthCheckRegistry>
            │
            ├─> Vec<Arc<dyn HealthCheck>>
            │       │
            │       ├─> PostgresHealthCheck
            │       │       ├─> PgPool
            │       │       └─> HealthCheckConfig
            │       │
            │       ├─> ClickHouseHealthCheck
            │       │       ├─> clickhouse::Client
            │       │       └─> HealthCheckConfig
            │       │
            │       └─> S3HealthCheck
            │               ├─> S3Client
            │               ├─> bucket: String
            │               └─> HealthCheckConfig
            │
            ├─> Arc<RwLock<HashMap<String, CachedHealth>>>
            │       │
            │       ├─> "postgres" → CachedHealth {
            │       │       result: ComponentHealth,
            │       │       cached_at: Instant
            │       │   }
            │       │
            │       ├─> "clickhouse" → CachedHealth { ... }
            │       │
            │       └─> "s3" → CachedHealth { ... }
            │
            ├─> start_time: Instant
            └─> version: String
```

## Health Status Decision Tree

```
┌─────────────────────────────────────────────┐
│     Check All Components                    │
└──────────────┬──────────────────────────────┘
               │
       ┌───────┴───────┐
       │               │
       ▼               ▼
  Critical      Non-Critical
  Components    Components
       │               │
       ├─ Postgres     ├─ ClickHouse
       └─ S3           └─ (none)
       │               │
       ▼               ▼
  All Healthy?    All Healthy?
       │               │
   ┌───┴───┐       ┌───┴───┐
   │       │       │       │
  Yes     No      Yes     No
   │       │       │       │
   ▼       ▼       ▼       ▼
   │   UNHEALTHY   │   DEGRADED
   │       │       │       │
   └───────┴───────┴───────┘
           │
           ▼
    Final Status
     │
     ├─> All components healthy → HEALTHY (200)
     ├─> Any non-critical degraded → DEGRADED (200)
     └─> Any critical unhealthy → UNHEALTHY (503)
```

## Timeout and Error Handling

```
Health Check Execution:

┌─────────────────────────────────────────┐
│  tokio::time::timeout(                  │
│      check_timeout,  // e.g., 3 seconds │
│      actual_check()                     │
│  )                                      │
└──────────────┬──────────────────────────┘
               │
       ┌───────┴───────┐
       │               │
       ▼               ▼
   Completed       Timed Out
   within          after 3s
   timeout
       │               │
       ▼               ▼
   Check Result    Return:
       │           ComponentHealth::unhealthy(
   ┌───┴───┐          "component",
   │       │          "Health check timed out"
  Ok     Err         )
   │       │
   ▼       ▼
 Healthy  Unhealthy
  (200)    (503)
```

## Concurrent Execution Pattern

```
registry.check_all() execution:

┌──────────────────────────────────────────┐
│  futures::future::join_all([             │
│      async { postgres.check().await },   │ ─┐
│      async { clickhouse.check().await }, │  │ Run
│      async { s3.check().await }          │  │ concurrently
│  ])                                      │ ─┘
└──────────────┬───────────────────────────┘
               │
               ▼
    Wait for all futures to complete
         (max time = slowest check)
               │
               ▼
       Collect all results
               │
       ┌───────┴────────┐
       │                │
       ▼                ▼
  [postgres: 15ms,  clickhouse: 120ms,
   s3: 87ms]
       │
       ▼
  Total time: 120ms (not 222ms!)
```

## Kubernetes Integration

```
┌─────────────────────────────────────────────────┐
│              Kubernetes Control Plane           │
│                                                 │
│  ┌───────────────────────────────────────────┐ │
│  │          Pod Lifecycle Manager            │ │
│  │                                           │ │
│  │  Every 10s: GET /health/live              │ │
│  │  ├─> 200 OK → Pod is alive                │ │
│  │  └─> Timeout/5xx → Restart pod            │ │
│  │                                           │ │
│  │  Every 5s: GET /health/ready              │ │
│  │  ├─> 200 OK → Add to service endpoints   │ │
│  │  └─> 503/Timeout → Remove from service   │ │
│  └───────────────────────────────────────────┘ │
└─────────────────────────────────────────────────┘
                    │
                    ▼
        ┌───────────────────────┐
        │    Service/Ingress    │
        │  (Load Balancer)      │
        │                       │
        │  Routes traffic only  │
        │  to ready pods        │
        └───────────────────────┘
```

## Data Flow Diagram

```
Client Request
      │
      ▼
┌──────────────┐
│  Axum Router │
└──────┬───────┘
       │
       ├─── /health/live ──────┐
       │                       │
       ├─── /health/ready ─────┤
       │                       │
       └─── /health ───────────┤
                               │
                               ▼
                    ┌──────────────────┐
                    │   Handler        │
                    │  (extracts State)│
                    └────────┬─────────┘
                             │
                             ▼
                    ┌──────────────────┐
                    │ HealthCheckState │
                    └────────┬─────────┘
                             │
                             ▼
                    ┌──────────────────┐
                    │   Registry       │
                    │  .check_xxx()    │
                    └────────┬─────────┘
                             │
                    ┌────────┴─────────┐
                    │                  │
                    ▼                  ▼
            Check Cache         Execute Checks
                    │                  │
                    │                  ▼
                    │          Run concurrently
                    │                  │
                    │                  ▼
                    │          Update Cache
                    │                  │
                    └────────┬─────────┘
                             │
                             ▼
                    ┌──────────────────┐
                    │ Aggregate Results│
                    └────────┬─────────┘
                             │
                             ▼
                    ┌──────────────────┐
                    │  OverallHealth   │
                    │    (with JSON)   │
                    └────────┬─────────┘
                             │
                             ▼
                    ┌──────────────────┐
                    │ HTTP Response    │
                    │ (200 or 503)     │
                    └──────────────────┘
```

## Memory Layout

```
Heap Memory Organization:

Arc<HealthCheckRegistry>  (shared across threads)
    │
    ├─> Vec<Arc<dyn HealthCheck>>  (~24 bytes + pointers)
    │       │
    │       ├─> Arc<PostgresHealthCheck>  (~48 bytes)
    │       │       ├─> PgPool (Arc internally)
    │       │       └─> HealthCheckConfig (32 bytes)
    │       │
    │       ├─> Arc<ClickHouseHealthCheck>  (~48 bytes)
    │       │
    │       └─> Arc<S3HealthCheck>  (~48 bytes)
    │
    ├─> Arc<RwLock<HashMap<String, CachedHealth>>>
    │       │
    │       └─> HashMap (capacity: 8)
    │               ├─> "postgres" → CachedHealth (~80 bytes)
    │               ├─> "clickhouse" → CachedHealth (~80 bytes)
    │               └─> "s3" → CachedHealth (~80 bytes)
    │
    ├─> Instant (16 bytes)
    └─> String ("1.0.0", ~24 bytes)

Total estimated memory: < 1 KB per registry
Cache overhead: ~240 bytes for 3 components
```

## Performance Profile

```
Latency Distribution:

Liveness Check:
├─ Always cached: 1-2 ms
└─ P99: < 5 ms

Readiness Check (2 components):
├─ Cache hit: 2-5 ms
├─ Cache miss: 50-150 ms
│   ├─ Postgres: 10-30 ms
│   └─ S3: 50-150 ms
└─ P99: < 200 ms

Detailed Health (3 components):
├─ Cache hit: 2-5 ms
├─ Cache miss: 100-200 ms
│   ├─ Postgres: 10-30 ms
│   ├─ ClickHouse: 20-100 ms
│   └─ S3: 50-150 ms
└─ P99: < 300 ms

Throughput (with caching):
├─ Liveness: > 10,000 req/s
├─ Readiness: > 5,000 req/s
└─ Detailed: > 5,000 req/s
```
