# SPARC Phase 5: Completion - Section 4: Production Infrastructure

> **LLM-Research-Lab Production Infrastructure Specification**
> Part of SPARC Phase 5 (Completion) - Enterprise-Grade Deployment
> Target: 99.9% SLA, Kubernetes-based, Multi-AZ High Availability

---

## Table of Contents

- [4.1 Kubernetes Configuration](#41-kubernetes-configuration)
- [4.2 Database Production Setup](#42-database-production-setup)
- [4.3 Message Queue Production](#43-message-queue-production)
- [4.4 Caching Layer](#44-caching-layer)
- [4.5 Load Balancing & Ingress](#45-load-balancing--ingress)
- [4.6 Service Mesh (Optional)](#46-service-mesh-optional)

---

## 4.1 Kubernetes Configuration

### 4.1.1 Namespace Organization

Isolate environments and components using a structured namespace hierarchy.

#### Namespace Structure

```yaml
# namespaces/base-namespaces.yaml
apiVersion: v1
kind: Namespace
metadata:
  name: llm-research-lab-prod
  labels:
    environment: production
    team: ai-research
    compliance: soc2
    cost-center: research-ops
---
apiVersion: v1
kind: Namespace
metadata:
  name: llm-research-lab-staging
  labels:
    environment: staging
    team: ai-research
---
apiVersion: v1
kind: Namespace
metadata:
  name: llm-research-lab-data
  labels:
    environment: production
    team: data-platform
    compliance: soc2
    pii: "true"
---
apiVersion: v1
kind: Namespace
metadata:
  name: llm-research-lab-monitoring
  labels:
    environment: production
    team: platform
```

#### Namespace Isolation Network Policies

```yaml
# namespaces/network-policies.yaml
apiVersion: networking.k8s.io/v1
kind: NetworkPolicy
metadata:
  name: default-deny-all
  namespace: llm-research-lab-prod
spec:
  podSelector: {}
  policyTypes:
  - Ingress
  - Egress
---
apiVersion: networking.k8s.io/v1
kind: NetworkPolicy
metadata:
  name: allow-same-namespace
  namespace: llm-research-lab-prod
spec:
  podSelector: {}
  policyTypes:
  - Ingress
  ingress:
  - from:
    - podSelector: {}
---
apiVersion: networking.k8s.io/v1
kind: NetworkPolicy
metadata:
  name: allow-to-data-layer
  namespace: llm-research-lab-prod
spec:
  podSelector:
    matchLabels:
      tier: backend
  policyTypes:
  - Egress
  egress:
  - to:
    - namespaceSelector:
        matchLabels:
          environment: production
    - podSelector:
        matchLabels:
          tier: database
    ports:
    - protocol: TCP
      port: 5432  # PostgreSQL
    - protocol: TCP
      port: 8123  # ClickHouse HTTP
    - protocol: TCP
      port: 9000  # ClickHouse Native
```

### 4.1.2 Resource Quotas and Limits

Prevent resource exhaustion and ensure fair allocation across tenants.

#### Namespace Resource Quotas

```yaml
# quotas/production-quota.yaml
apiVersion: v1
kind: ResourceQuota
metadata:
  name: compute-quota
  namespace: llm-research-lab-prod
spec:
  hard:
    requests.cpu: "100"
    requests.memory: 200Gi
    limits.cpu: "200"
    limits.memory: 400Gi
    requests.nvidia.com/gpu: "10"
    persistentvolumeclaims: "50"
    requests.storage: 1Ti
---
apiVersion: v1
kind: ResourceQuota
metadata:
  name: object-quota
  namespace: llm-research-lab-prod
spec:
  hard:
    pods: "200"
    services: "50"
    configmaps: "100"
    secrets: "100"
    persistentvolumeclaims: "50"
```

#### LimitRange for Default Container Resources

```yaml
# quotas/limit-range.yaml
apiVersion: v1
kind: LimitRange
metadata:
  name: default-limits
  namespace: llm-research-lab-prod
spec:
  limits:
  # Container limits
  - type: Container
    default:
      cpu: "1"
      memory: 2Gi
    defaultRequest:
      cpu: "500m"
      memory: 1Gi
    max:
      cpu: "16"
      memory: 64Gi
    min:
      cpu: "100m"
      memory: 128Mi
  # Pod limits
  - type: Pod
    max:
      cpu: "32"
      memory: 128Gi
      nvidia.com/gpu: "4"
  # PVC limits
  - type: PersistentVolumeClaim
    max:
      storage: 500Gi
    min:
      storage: 1Gi
```

### 4.1.3 Pod Disruption Budgets

Ensure high availability during voluntary disruptions (node drains, upgrades).

```yaml
# disruption/api-pdb.yaml
apiVersion: policy/v1
kind: PodDisruptionBudget
metadata:
  name: llm-api-pdb
  namespace: llm-research-lab-prod
spec:
  minAvailable: 2
  selector:
    matchLabels:
      app: llm-research-api
      tier: backend
---
# disruption/worker-pdb.yaml
apiVersion: policy/v1
kind: PodDisruptionBudget
metadata:
  name: experiment-worker-pdb
  namespace: llm-research-lab-prod
spec:
  maxUnavailable: 25%
  selector:
    matchLabels:
      app: experiment-worker
      tier: worker
---
# disruption/postgres-pdb.yaml
apiVersion: policy/v1
kind: PodDisruptionBudget
metadata:
  name: postgres-pdb
  namespace: llm-research-lab-data
spec:
  minAvailable: 1
  selector:
    matchLabels:
      app.kubernetes.io/name: postgresql
```

### 4.1.4 Horizontal Pod Autoscaler Configuration

Scale based on CPU, memory, and custom metrics.

#### CPU-Based Autoscaling

```yaml
# autoscaling/api-hpa.yaml
apiVersion: autoscaling/v2
kind: HorizontalPodAutoscaler
metadata:
  name: llm-api-hpa
  namespace: llm-research-lab-prod
spec:
  scaleTargetRef:
    apiVersion: apps/v1
    kind: Deployment
    name: llm-research-api
  minReplicas: 3
  maxReplicas: 20
  metrics:
  - type: Resource
    resource:
      name: cpu
      target:
        type: Utilization
        averageUtilization: 70
  - type: Resource
    resource:
      name: memory
      target:
        type: Utilization
        averageUtilization: 80
  behavior:
    scaleDown:
      stabilizationWindowSeconds: 300
      policies:
      - type: Percent
        value: 50
        periodSeconds: 60
    scaleUp:
      stabilizationWindowSeconds: 60
      policies:
      - type: Percent
        value: 100
        periodSeconds: 30
      - type: Pods
        value: 4
        periodSeconds: 30
      selectPolicy: Max
```

#### Custom Metrics Autoscaling (Prometheus Adapter)

```yaml
# autoscaling/custom-metrics-hpa.yaml
apiVersion: autoscaling/v2
kind: HorizontalPodAutoscaler
metadata:
  name: experiment-worker-hpa
  namespace: llm-research-lab-prod
spec:
  scaleTargetRef:
    apiVersion: apps/v1
    kind: Deployment
    name: experiment-worker
  minReplicas: 2
  maxReplicas: 50
  metrics:
  # Scale based on Kafka consumer lag
  - type: External
    external:
      metric:
        name: kafka_consumergroup_lag
        selector:
          matchLabels:
            topic: experiments
      target:
        type: AverageValue
        averageValue: "100"  # Target 100 messages lag per pod
  # Scale based on experiment queue depth
  - type: Pods
    pods:
      metric:
        name: experiment_queue_depth
      target:
        type: AverageValue
        averageValue: "50"
```

### 4.1.5 Priority Classes

Define scheduling priorities for critical vs batch workloads.

```yaml
# priority/priority-classes.yaml
apiVersion: scheduling.k8s.io/v1
kind: PriorityClass
metadata:
  name: system-critical
value: 1000000000
globalDefault: false
description: "Reserved for system-critical components (monitoring, logging)"
---
apiVersion: scheduling.k8s.io/v1
kind: PriorityClass
metadata:
  name: production-high
value: 100000
globalDefault: false
description: "High priority for production API services"
---
apiVersion: scheduling.k8s.io/v1
kind: PriorityClass
metadata:
  name: production-normal
value: 10000
globalDefault: true
description: "Default priority for production workloads"
---
apiVersion: scheduling.k8s.io/v1
kind: PriorityClass
metadata:
  name: batch-low
value: 1000
globalDefault: false
description: "Low priority for batch jobs and experiments"
preemptionPolicy: PreemptLowerPriority
```

#### Example Deployment with Priority

```yaml
# deployments/api-with-priority.yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: llm-research-api
  namespace: llm-research-lab-prod
spec:
  replicas: 3
  selector:
    matchLabels:
      app: llm-research-api
  template:
    metadata:
      labels:
        app: llm-research-api
        tier: backend
        version: v1.0.0
    spec:
      priorityClassName: production-high
      affinity:
        podAntiAffinity:
          requiredDuringSchedulingIgnoredDuringExecution:
          - labelSelector:
              matchExpressions:
              - key: app
                operator: In
                values:
                - llm-research-api
            topologyKey: kubernetes.io/hostname
      containers:
      - name: api
        image: llm-research-lab/api:v1.0.0
        resources:
          requests:
            cpu: "2"
            memory: 4Gi
          limits:
            cpu: "4"
            memory: 8Gi
        livenessProbe:
          httpGet:
            path: /health/live
            port: 8080
          initialDelaySeconds: 30
          periodSeconds: 10
        readinessProbe:
          httpGet:
            path: /health/ready
            port: 8080
          initialDelaySeconds: 10
          periodSeconds: 5
```

---

## 4.2 Database Production Setup

### 4.2.1 PostgreSQL High Availability (CloudNativePG)

Deploy PostgreSQL with automatic failover and streaming replication.

#### CloudNativePG Cluster Configuration

```yaml
# databases/postgres-cluster.yaml
apiVersion: postgresql.cnpg.io/v1
kind: Cluster
metadata:
  name: llm-research-postgres
  namespace: llm-research-lab-data
spec:
  instances: 3
  imageName: ghcr.io/cloudnative-pg/postgresql:15.4

  # Storage configuration
  storage:
    size: 500Gi
    storageClass: fast-ssd

  # Replication configuration
  postgresql:
    parameters:
      max_connections: "500"
      shared_buffers: "4GB"
      effective_cache_size: "12GB"
      maintenance_work_mem: "1GB"
      checkpoint_completion_target: "0.9"
      wal_buffers: "16MB"
      default_statistics_target: "100"
      random_page_cost: "1.1"
      effective_io_concurrency: "200"
      work_mem: "8MB"
      min_wal_size: "1GB"
      max_wal_size: "4GB"
      max_worker_processes: "4"
      max_parallel_workers_per_gather: "2"
      max_parallel_workers: "4"
      max_parallel_maintenance_workers: "2"

  # High availability
  enableSuperuserAccess: true
  primaryUpdateStrategy: unsupervised

  # Backup configuration
  backup:
    barmanObjectStore:
      destinationPath: s3://llm-research-backups/postgres
      s3Credentials:
        accessKeyId:
          name: postgres-backup-s3
          key: ACCESS_KEY_ID
        secretAccessKey:
          name: postgres-backup-s3
          key: ACCESS_SECRET_KEY
      wal:
        compression: gzip
        maxParallel: 4
    retentionPolicy: "30d"

  # Monitoring
  monitoring:
    enablePodMonitor: true

  # Resources
  resources:
    requests:
      memory: "8Gi"
      cpu: "4"
    limits:
      memory: "16Gi"
      cpu: "8"

  # Node affinity
  affinity:
    podAntiAffinity:
      requiredDuringSchedulingIgnoredDuringExecution:
      - labelSelector:
          matchExpressions:
          - key: cnpg.io/cluster
            operator: In
            values:
            - llm-research-postgres
        topologyKey: kubernetes.io/hostname
```

#### Scheduled Backups

```yaml
# databases/postgres-scheduled-backup.yaml
apiVersion: postgresql.cnpg.io/v1
kind: ScheduledBackup
metadata:
  name: llm-postgres-backup
  namespace: llm-research-lab-data
spec:
  schedule: "0 2 * * *"  # Daily at 2 AM
  backupOwnerReference: self
  cluster:
    name: llm-research-postgres
```

#### Point-in-Time Recovery Configuration

```yaml
# databases/postgres-pitr.yaml
# PITR is enabled automatically with WAL archiving
# To restore to a specific point in time, create a new cluster:

apiVersion: postgresql.cnpg.io/v1
kind: Cluster
metadata:
  name: llm-research-postgres-restored
  namespace: llm-research-lab-data
spec:
  instances: 3
  imageName: ghcr.io/cloudnative-pg/postgresql:15.4

  bootstrap:
    recovery:
      source: llm-research-postgres
      recoveryTarget:
        targetTime: "2025-11-28 14:30:00.000000+00"

  externalClusters:
  - name: llm-research-postgres
    barmanObjectStore:
      destinationPath: s3://llm-research-backups/postgres
      s3Credentials:
        accessKeyId:
          name: postgres-backup-s3
          key: ACCESS_KEY_ID
        secretAccessKey:
          name: postgres-backup-s3
          key: ACCESS_SECRET_KEY
```

### 4.2.2 ClickHouse Cluster Configuration

Deploy distributed ClickHouse for analytics and time-series data.

#### ClickHouse Operator Installation

```yaml
# databases/clickhouse-operator.yaml
apiVersion: v1
kind: Namespace
metadata:
  name: clickhouse-operator
---
apiVersion: apps/v1
kind: Deployment
metadata:
  name: clickhouse-operator
  namespace: clickhouse-operator
spec:
  replicas: 1
  selector:
    matchLabels:
      app: clickhouse-operator
  template:
    metadata:
      labels:
        app: clickhouse-operator
    spec:
      serviceAccountName: clickhouse-operator
      containers:
      - name: clickhouse-operator
        image: altinity/clickhouse-operator:0.23.0
        imagePullPolicy: IfNotPresent
        env:
        - name: OPERATOR_POD_NODE_NAME
          valueFrom:
            fieldRef:
              fieldPath: spec.nodeName
        - name: OPERATOR_POD_NAME
          valueFrom:
            fieldRef:
              fieldPath: metadata.name
        - name: OPERATOR_POD_NAMESPACE
          valueFrom:
            fieldRef:
              fieldPath: metadata.namespace
```

#### ClickHouse Cluster Deployment

```yaml
# databases/clickhouse-cluster.yaml
apiVersion: clickhouse.altinity.com/v1
kind: ClickHouseInstallation
metadata:
  name: llm-research-clickhouse
  namespace: llm-research-lab-data
spec:
  configuration:
    users:
      admin/password: k8s-secret:clickhouse-admin:password
      admin/networks/ip: "::/0"
      readonly/password: k8s-secret:clickhouse-readonly:password
      readonly/profile: readonly
      readonly/networks/ip: "::/0"

    profiles:
      readonly:
        readonly: 1
      default:
        max_memory_usage: 10000000000
        use_uncompressed_cache: 0
        load_balancing: random

    quotas:
      default:
        interval:
          duration: 3600
          queries: 1000
          errors: 100
          result_rows: 1000000000
          read_rows: 1000000000
          execution_time: 3600

    clusters:
      - name: llm-cluster
        layout:
          shardsCount: 3
          replicasCount: 2

    zookeeper:
      nodes:
      - host: zookeeper-0.zookeeper-headless.llm-research-lab-data.svc.cluster.local
        port: 2181
      - host: zookeeper-1.zookeeper-headless.llm-research-lab-data.svc.cluster.local
        port: 2181
      - host: zookeeper-2.zookeeper-headless.llm-research-lab-data.svc.cluster.local
        port: 2181

  defaults:
    templates:
      dataVolumeClaimTemplate: data-volume
      logVolumeClaimTemplate: log-volume
      serviceTemplate: chi-service-template

  templates:
    volumeClaimTemplates:
    - name: data-volume
      spec:
        accessModes:
        - ReadWriteOnce
        storageClassName: fast-ssd
        resources:
          requests:
            storage: 1Ti
    - name: log-volume
      spec:
        accessModes:
        - ReadWriteOnce
        storageClassName: standard
        resources:
          requests:
            storage: 100Gi

    serviceTemplates:
    - name: chi-service-template
      spec:
        type: ClusterIP
        ports:
        - name: http
          port: 8123
        - name: client
          port: 9000
        - name: interserver
          port: 9009

    podTemplates:
    - name: default
      spec:
        containers:
        - name: clickhouse
          image: clickhouse/clickhouse-server:23.8
          resources:
            requests:
              memory: "16Gi"
              cpu: "4"
            limits:
              memory: "32Gi"
              cpu: "8"
```

#### ClickHouse Distributed Table Example

```sql
-- Run on each ClickHouse instance
-- databases/clickhouse-schema.sql

-- Local table on each shard
CREATE TABLE IF NOT EXISTS experiment_metrics_local ON CLUSTER llm-cluster
(
    experiment_id UUID,
    metric_name LowCardinality(String),
    metric_value Float64,
    timestamp DateTime64(3),
    labels Map(String, String),
    INDEX idx_timestamp timestamp TYPE minmax GRANULARITY 3,
    INDEX idx_metric metric_name TYPE set(0) GRANULARITY 1
)
ENGINE = ReplicatedMergeTree('/clickhouse/tables/{shard}/experiment_metrics', '{replica}')
PARTITION BY toYYYYMM(timestamp)
ORDER BY (experiment_id, metric_name, timestamp)
TTL timestamp + INTERVAL 90 DAY
SETTINGS index_granularity = 8192;

-- Distributed table for queries
CREATE TABLE IF NOT EXISTS experiment_metrics ON CLUSTER llm-cluster
AS experiment_metrics_local
ENGINE = Distributed(llm-cluster, default, experiment_metrics_local, rand());
```

### 4.2.3 Connection Pooling (PgBouncer)

Deploy PgBouncer for PostgreSQL connection pooling.

```yaml
# databases/pgbouncer.yaml
apiVersion: v1
kind: ConfigMap
metadata:
  name: pgbouncer-config
  namespace: llm-research-lab-data
data:
  pgbouncer.ini: |
    [databases]
    llm_research = host=llm-research-postgres-rw.llm-research-lab-data.svc.cluster.local port=5432 dbname=llm_research

    [pgbouncer]
    listen_addr = 0.0.0.0
    listen_port = 5432
    auth_type = scram-sha-256
    auth_file = /etc/pgbouncer/userlist.txt

    pool_mode = transaction
    max_client_conn = 2000
    default_pool_size = 25
    min_pool_size = 10
    reserve_pool_size = 5
    reserve_pool_timeout = 3
    max_db_connections = 100
    max_user_connections = 100

    server_reset_query = DISCARD ALL
    server_check_delay = 10
    server_check_query = SELECT 1

    log_connections = 1
    log_disconnections = 1
    log_pooler_errors = 1
    stats_period = 60

    ignore_startup_parameters = extra_float_digits
---
apiVersion: apps/v1
kind: Deployment
metadata:
  name: pgbouncer
  namespace: llm-research-lab-data
spec:
  replicas: 3
  selector:
    matchLabels:
      app: pgbouncer
  template:
    metadata:
      labels:
        app: pgbouncer
    spec:
      containers:
      - name: pgbouncer
        image: edoburu/pgbouncer:1.21.0
        ports:
        - containerPort: 5432
          name: postgres
        resources:
          requests:
            cpu: "500m"
            memory: 512Mi
          limits:
            cpu: "2"
            memory: 2Gi
        volumeMounts:
        - name: config
          mountPath: /etc/pgbouncer
        livenessProbe:
          tcpSocket:
            port: 5432
          initialDelaySeconds: 15
          periodSeconds: 10
        readinessProbe:
          tcpSocket:
            port: 5432
          initialDelaySeconds: 5
          periodSeconds: 5
      volumes:
      - name: config
        configMap:
          name: pgbouncer-config
---
apiVersion: v1
kind: Service
metadata:
  name: pgbouncer
  namespace: llm-research-lab-data
spec:
  type: ClusterIP
  ports:
  - port: 5432
    targetPort: 5432
    protocol: TCP
    name: postgres
  selector:
    app: pgbouncer
```

### 4.2.4 Database Monitoring

Deploy Prometheus exporters for database metrics.

```yaml
# monitoring/postgres-exporter.yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: postgres-exporter
  namespace: llm-research-lab-data
spec:
  replicas: 1
  selector:
    matchLabels:
      app: postgres-exporter
  template:
    metadata:
      labels:
        app: postgres-exporter
      annotations:
        prometheus.io/scrape: "true"
        prometheus.io/port: "9187"
    spec:
      containers:
      - name: postgres-exporter
        image: prometheuscommunity/postgres-exporter:v0.15.0
        ports:
        - containerPort: 9187
          name: metrics
        env:
        - name: DATA_SOURCE_NAME
          valueFrom:
            secretKeyRef:
              name: postgres-exporter-secret
              key: connection-string
        resources:
          requests:
            cpu: 100m
            memory: 128Mi
          limits:
            cpu: 500m
            memory: 512Mi
---
# monitoring/clickhouse-exporter.yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: clickhouse-exporter
  namespace: llm-research-lab-data
spec:
  replicas: 1
  selector:
    matchLabels:
      app: clickhouse-exporter
  template:
    metadata:
      labels:
        app: clickhouse-exporter
      annotations:
        prometheus.io/scrape: "true"
        prometheus.io/port: "9116"
    spec:
      containers:
      - name: clickhouse-exporter
        image: f1yegor/clickhouse-exporter:latest
        ports:
        - containerPort: 9116
          name: metrics
        env:
        - name: CLICKHOUSE_URI
          value: "http://clickhouse-llm-research-clickhouse.llm-research-lab-data.svc.cluster.local:8123"
        resources:
          requests:
            cpu: 100m
            memory: 128Mi
          limits:
            cpu: 500m
            memory: 512Mi
```

---

## 4.3 Message Queue Production

### 4.3.1 Kafka Cluster Sizing

Deploy production-grade Kafka with Strimzi operator.

#### Strimzi Kafka Cluster

```yaml
# kafka/kafka-cluster.yaml
apiVersion: kafka.strimzi.io/v1beta2
kind: Kafka
metadata:
  name: llm-research-kafka
  namespace: llm-research-lab-prod
spec:
  kafka:
    version: 3.6.0
    replicas: 5

    listeners:
    - name: plain
      port: 9092
      type: internal
      tls: false
    - name: tls
      port: 9093
      type: internal
      tls: true
      authentication:
        type: tls

    config:
      # Replication settings
      offsets.topic.replication.factor: 3
      transaction.state.log.replication.factor: 3
      transaction.state.log.min.isr: 2
      default.replication.factor: 3
      min.insync.replicas: 2

      # Performance settings
      num.network.threads: 8
      num.io.threads: 16
      socket.send.buffer.bytes: 102400
      socket.receive.buffer.bytes: 102400
      socket.request.max.bytes: 104857600

      # Retention settings
      log.retention.hours: 168  # 7 days
      log.segment.bytes: 1073741824  # 1GB
      log.retention.check.interval.ms: 300000

      # Compression
      compression.type: snappy

      # Leader election
      unclean.leader.election.enable: false
      auto.create.topics.enable: false

    storage:
      type: jbod
      volumes:
      - id: 0
        type: persistent-claim
        size: 1Ti
        class: fast-ssd
        deleteClaim: false

    resources:
      requests:
        memory: 16Gi
        cpu: "4"
      limits:
        memory: 32Gi
        cpu: "8"

    jvmOptions:
      -Xms: 8192m
      -Xmx: 8192m
      gcLoggingEnabled: true

    metricsConfig:
      type: jmxPrometheusExporter
      valueFrom:
        configMapKeyRef:
          name: kafka-metrics
          key: kafka-metrics-config.yml

  zookeeper:
    replicas: 3

    storage:
      type: persistent-claim
      size: 100Gi
      class: fast-ssd
      deleteClaim: false

    resources:
      requests:
        memory: 4Gi
        cpu: "2"
      limits:
        memory: 8Gi
        cpu: "4"

    metricsConfig:
      type: jmxPrometheusExporter
      valueFrom:
        configMapKeyRef:
          name: kafka-metrics
          key: zookeeper-metrics-config.yml

  entityOperator:
    topicOperator:
      resources:
        requests:
          memory: 512Mi
          cpu: "200m"
        limits:
          memory: 1Gi
          cpu: "1"
    userOperator:
      resources:
        requests:
          memory: 512Mi
          cpu: "200m"
        limits:
          memory: 1Gi
          cpu: "1"
```

### 4.3.2 Topic Configuration and Partitioning

```yaml
# kafka/topics.yaml
apiVersion: kafka.strimzi.io/v1beta2
kind: KafkaTopic
metadata:
  name: experiments
  namespace: llm-research-lab-prod
  labels:
    strimzi.io/cluster: llm-research-kafka
spec:
  partitions: 30
  replicas: 3
  config:
    retention.ms: 604800000  # 7 days
    segment.ms: 3600000      # 1 hour
    compression.type: snappy
    min.insync.replicas: 2
    cleanup.policy: delete
---
apiVersion: kafka.strimzi.io/v1beta2
kind: KafkaTopic
metadata:
  name: experiment-results
  namespace: llm-research-lab-prod
  labels:
    strimzi.io/cluster: llm-research-kafka
spec:
  partitions: 20
  replicas: 3
  config:
    retention.ms: 2592000000  # 30 days
    segment.ms: 3600000
    compression.type: snappy
    min.insync.replicas: 2
    cleanup.policy: delete
---
apiVersion: kafka.strimzi.io/v1beta2
kind: KafkaTopic
metadata:
  name: metrics-events
  namespace: llm-research-lab-prod
  labels:
    strimzi.io/cluster: llm-research-kafka
spec:
  partitions: 50
  replicas: 3
  config:
    retention.ms: 259200000  # 3 days
    segment.ms: 1800000      # 30 minutes
    compression.type: lz4
    min.insync.replicas: 2
    cleanup.policy: delete
---
# High-volume topic for telemetry
apiVersion: kafka.strimzi.io/v1beta2
kind: KafkaTopic
metadata:
  name: telemetry-stream
  namespace: llm-research-lab-prod
  labels:
    strimzi.io/cluster: llm-research-kafka
spec:
  partitions: 100
  replicas: 3
  config:
    retention.ms: 86400000   # 1 day
    segment.ms: 600000       # 10 minutes
    compression.type: lz4
    min.insync.replicas: 2
    cleanup.policy: delete
```

### 4.3.3 Consumer Group Management

```yaml
# kafka/consumer-groups.yaml
# Rust application configuration for consumer groups

# Application-side configuration example (config.toml)
[kafka.consumer]
group_id = "experiment-processor-v1"
enable_auto_commit = false
auto_offset_reset = "earliest"
session_timeout_ms = 30000
max_poll_interval_ms = 300000
max_poll_records = 500

[kafka.consumer.processing]
max_concurrency = 10
batch_size = 100
commit_interval_ms = 5000

# Kafka Connect for CDC and integration
---
apiVersion: kafka.strimzi.io/v1beta2
kind: KafkaConnect
metadata:
  name: llm-research-connect
  namespace: llm-research-lab-prod
  annotations:
    strimzi.io/use-connector-resources: "true"
spec:
  version: 3.6.0
  replicas: 3
  bootstrapServers: llm-research-kafka-kafka-bootstrap:9093
  tls:
    trustedCertificates:
    - secretName: llm-research-kafka-cluster-ca-cert
      certificate: ca.crt

  config:
    group.id: connect-cluster
    offset.storage.topic: connect-cluster-offsets
    config.storage.topic: connect-cluster-configs
    status.storage.topic: connect-cluster-status

    config.storage.replication.factor: 3
    offset.storage.replication.factor: 3
    status.storage.replication.factor: 3

  resources:
    requests:
      memory: 4Gi
      cpu: "2"
    limits:
      memory: 8Gi
      cpu: "4"
```

### 4.3.4 Replication and Durability Settings

```yaml
# kafka/kafka-producer-config.yaml
# Producer configuration for Rust application

[kafka.producer]
bootstrap_servers = "llm-research-kafka-kafka-bootstrap:9093"
acks = "all"  # Wait for all in-sync replicas
retries = 2147483647
max_in_flight_requests_per_connection = 5
enable_idempotence = true
compression_type = "snappy"
linger_ms = 10
batch_size = 32768

# Durability guarantees
[kafka.producer.durability]
min_insync_replicas = 2
request_timeout_ms = 30000
delivery_timeout_ms = 120000
```

#### Kafka MirrorMaker 2 for DR

```yaml
# kafka/mirror-maker-2.yaml
apiVersion: kafka.strimzi.io/v1beta2
kind: KafkaMirrorMaker2
metadata:
  name: llm-kafka-mirror
  namespace: llm-research-lab-prod
spec:
  version: 3.6.0
  replicas: 3
  connectCluster: "target"

  clusters:
  - alias: "source"
    bootstrapServers: llm-research-kafka-kafka-bootstrap:9093
    tls:
      trustedCertificates:
      - secretName: source-cluster-ca-cert
        certificate: ca.crt

  - alias: "target"
    bootstrapServers: dr-kafka-bootstrap.dr-namespace:9093
    tls:
      trustedCertificates:
      - secretName: target-cluster-ca-cert
        certificate: ca.crt
    config:
      config.storage.replication.factor: 3
      offset.storage.replication.factor: 3
      status.storage.replication.factor: 3

  mirrors:
  - sourceCluster: "source"
    targetCluster: "target"
    sourceConnector:
      config:
        replication.factor: 3
        offset-syncs.topic.replication.factor: 3
        sync.topic.acls.enabled: "false"

    heartbeatConnector:
      config:
        heartbeats.topic.replication.factor: 3

    checkpointConnector:
      config:
        checkpoints.topic.replication.factor: 3
        sync.group.offsets.enabled: "true"

    topicsPattern: ".*"
    groupsPattern: ".*"
```

---

## 4.4 Caching Layer

### 4.4.1 Redis Cluster Configuration

Deploy Redis in cluster mode for high availability.

```yaml
# redis/redis-cluster.yaml
apiVersion: redis.redis.opstreelabs.in/v1beta1
kind: RedisCluster
metadata:
  name: llm-research-redis
  namespace: llm-research-lab-prod
spec:
  clusterSize: 6  # 3 masters + 3 replicas
  clusterVersion: v7.2

  kubernetesConfig:
    image: redis:7.2-alpine
    imagePullPolicy: IfNotPresent

    redisSecret:
      name: redis-secret
      key: password

    resources:
      requests:
        cpu: "1"
        memory: 4Gi
      limits:
        cpu: "2"
        memory: 8Gi

  storage:
    volumeClaimTemplate:
      spec:
        accessModes:
        - ReadWriteOnce
        storageClassName: fast-ssd
        resources:
          requests:
            storage: 100Gi

  redisExporter:
    enabled: true
    image: oliver006/redis_exporter:v1.55.0
    resources:
      requests:
        cpu: 100m
        memory: 128Mi
      limits:
        cpu: 500m
        memory: 512Mi

  redisConfig:
    # Memory management
    maxmemory: "6gb"
    maxmemory-policy: "allkeys-lru"

    # Persistence
    save: "900 1 300 10 60 10000"
    appendonly: "yes"
    appendfsync: "everysec"

    # Performance
    tcp-backlog: "511"
    timeout: "300"
    tcp-keepalive: "300"

    # Slow log
    slowlog-log-slower-than: "10000"
    slowlog-max-len: "128"

    # Client output buffer limits
    client-output-buffer-limit: "normal 0 0 0 slave 256mb 64mb 60 pubsub 32mb 8mb 60"
```

### 4.4.2 Redis Sentinel for Master Election

Alternative: Sentinel mode for simpler high availability.

```yaml
# redis/redis-sentinel.yaml
apiVersion: redis.redis.opstreelabs.in/v1beta1
kind: Redis
metadata:
  name: llm-research-redis-sentinel
  namespace: llm-research-lab-prod
spec:
  kubernetesConfig:
    image: redis:7.2-alpine
    imagePullPolicy: IfNotPresent

    redisSecret:
      name: redis-secret
      key: password

    resources:
      requests:
        cpu: "1"
        memory: 4Gi
      limits:
        cpu: "2"
        memory: 8Gi

  redisConfig:
    maxmemory: "6gb"
    maxmemory-policy: "allkeys-lru"
    appendonly: "yes"

  storage:
    volumeClaimTemplate:
      spec:
        accessModes:
        - ReadWriteOnce
        storageClassName: fast-ssd
        resources:
          requests:
            storage: 100Gi

  redisExporter:
    enabled: true
    image: oliver006/redis_exporter:v1.55.0
---
apiVersion: redis.redis.opstreelabs.in/v1beta1
kind: RedisSentinel
metadata:
  name: llm-research-sentinel
  namespace: llm-research-lab-prod
spec:
  clusterSize: 3

  kubernetesConfig:
    image: redis:7.2-alpine
    imagePullPolicy: IfNotPresent

    resources:
      requests:
        cpu: "200m"
        memory: 512Mi
      limits:
        cpu: "1"
        memory: 1Gi

  redisSentinelConfig:
    down-after-milliseconds: "5000"
    failover-timeout: "10000"
    parallel-syncs: "1"
    quorum: "2"

    monitor:
      master: llm-research-redis-sentinel
      namespace: llm-research-lab-prod
```

### 4.4.3 Cache Eviction Policies

Configure different eviction strategies per use case.

```toml
# Application configuration (config.toml)

[cache.sessions]
# Volatile LRU for session data
redis_db = 0
maxmemory_policy = "volatile-lru"
ttl_seconds = 3600

[cache.experiments]
# Allkeys LRU for experiment metadata
redis_db = 1
maxmemory_policy = "allkeys-lru"
ttl_seconds = 86400

[cache.metrics]
# Volatile TTL for time-series metrics
redis_db = 2
maxmemory_policy = "volatile-ttl"
ttl_seconds = 1800

[cache.rate_limiting]
# No eviction for rate limit counters
redis_db = 3
maxmemory_policy = "noeviction"
ttl_seconds = 60
```

### 4.4.4 Persistence Configuration

```yaml
# redis/redis-persistence-config.yaml
apiVersion: v1
kind: ConfigMap
metadata:
  name: redis-persistence-config
  namespace: llm-research-lab-prod
data:
  redis.conf: |
    # RDB Snapshots
    save 900 1       # Save after 900s if at least 1 key changed
    save 300 10      # Save after 300s if at least 10 keys changed
    save 60 10000    # Save after 60s if at least 10000 keys changed

    rdbcompression yes
    rdbchecksum yes
    dbfilename dump.rdb
    dir /data

    # AOF Persistence
    appendonly yes
    appendfilename "appendonly.aof"
    appendfsync everysec  # Balance between performance and durability

    # AOF rewrite configuration
    auto-aof-rewrite-percentage 100
    auto-aof-rewrite-min-size 64mb
    aof-load-truncated yes
    aof-use-rdb-preamble yes  # Hybrid RDB+AOF for faster restarts
```

### 4.4.5 Memory Management

```yaml
# redis/redis-memory-policy.yaml
# Configured via RedisConfig in operator

# Monitoring ConfigMap for memory alerts
apiVersion: v1
kind: ConfigMap
metadata:
  name: redis-memory-alerts
  namespace: llm-research-lab-monitoring
data:
  alerts.yml: |
    groups:
    - name: redis_memory
      interval: 30s
      rules:
      - alert: RedisMemoryHigh
        expr: redis_memory_used_bytes / redis_memory_max_bytes > 0.85
        for: 5m
        labels:
          severity: warning
        annotations:
          summary: "Redis memory usage is above 85%"
          description: "Redis instance {{ $labels.instance }} is using {{ $value | humanizePercentage }} of available memory"

      - alert: RedisMemoryCritical
        expr: redis_memory_used_bytes / redis_memory_max_bytes > 0.95
        for: 2m
        labels:
          severity: critical
        annotations:
          summary: "Redis memory usage is critical"
          description: "Redis instance {{ $labels.instance }} is using {{ $value | humanizePercentage }} of available memory"

      - alert: RedisEvictionRateHigh
        expr: rate(redis_evicted_keys_total[5m]) > 100
        for: 5m
        labels:
          severity: warning
        annotations:
          summary: "High eviction rate detected"
          description: "Redis is evicting {{ $value }} keys per second"
```

---

## 4.5 Load Balancing & Ingress

### 4.5.1 Ingress Controller Configuration

Deploy NGINX Ingress Controller with production settings.

```yaml
# ingress/nginx-ingress-controller.yaml
apiVersion: v1
kind: Namespace
metadata:
  name: ingress-nginx
---
apiVersion: helm.cattle.io/v1
kind: HelmChart
metadata:
  name: nginx-ingress
  namespace: ingress-nginx
spec:
  repo: https://kubernetes.github.io/ingress-nginx
  chart: ingress-nginx
  version: 4.8.0

  valuesContent: |-
    controller:
      replicaCount: 3

      resources:
        requests:
          cpu: "1"
          memory: 2Gi
        limits:
          cpu: "2"
          memory: 4Gi

      affinity:
        podAntiAffinity:
          requiredDuringSchedulingIgnoredDuringExecution:
          - labelSelector:
              matchExpressions:
              - key: app.kubernetes.io/name
                operator: In
                values:
                - ingress-nginx
            topologyKey: kubernetes.io/hostname

      config:
        # Performance tuning
        worker-processes: "auto"
        worker-connections: "65536"
        max-worker-connections: "65536"

        # Timeouts
        proxy-connect-timeout: "10"
        proxy-send-timeout: "60"
        proxy-read-timeout: "60"
        client-body-timeout: "60"

        # Buffer sizes
        proxy-buffer-size: "8k"
        proxy-buffers-number: "4"
        client-body-buffer-size: "1m"

        # Compression
        use-gzip: "true"
        gzip-level: "5"
        gzip-types: "application/json text/plain text/css application/javascript"

        # Security
        ssl-protocols: "TLSv1.2 TLSv1.3"
        ssl-ciphers: "ECDHE-ECDSA-AES128-GCM-SHA256:ECDHE-RSA-AES128-GCM-SHA256:ECDHE-ECDSA-AES256-GCM-SHA384:ECDHE-RSA-AES256-GCM-SHA384"
        ssl-prefer-server-ciphers: "true"

        # Rate limiting
        limit-req-status-code: "429"
        limit-conn-status-code: "429"

        # Logging
        log-format-upstream: '$remote_addr - $remote_user [$time_local] "$request" $status $body_bytes_sent "$http_referer" "$http_user_agent" $request_length $request_time [$proxy_upstream_name] [$proxy_alternative_upstream_name] $upstream_addr $upstream_response_length $upstream_response_time $upstream_status $req_id'

        # Header size
        large-client-header-buffers: "4 16k"

      metrics:
        enabled: true
        serviceMonitor:
          enabled: true

      service:
        type: LoadBalancer
        annotations:
          service.beta.kubernetes.io/aws-load-balancer-type: "nlb"
          service.beta.kubernetes.io/aws-load-balancer-cross-zone-load-balancing-enabled: "true"
```

### 4.5.2 TLS Termination

Configure TLS certificates with cert-manager.

```yaml
# ingress/cert-manager.yaml
apiVersion: v1
kind: Namespace
metadata:
  name: cert-manager
---
apiVersion: helm.cattle.io/v1
kind: HelmChart
metadata:
  name: cert-manager
  namespace: cert-manager
spec:
  repo: https://charts.jetstack.io
  chart: cert-manager
  version: v1.13.0

  valuesContent: |-
    installCRDs: true

    resources:
      requests:
        cpu: 100m
        memory: 128Mi
      limits:
        cpu: 500m
        memory: 512Mi
---
# ClusterIssuer for Let's Encrypt
apiVersion: cert-manager.io/v1
kind: ClusterIssuer
metadata:
  name: letsencrypt-prod
spec:
  acme:
    server: https://acme-v02.api.letsencrypt.org/directory
    email: platform@company.com
    privateKeySecretRef:
      name: letsencrypt-prod-key
    solvers:
    - http01:
        ingress:
          class: nginx
---
# Certificate resource
apiVersion: cert-manager.io/v1
kind: Certificate
metadata:
  name: llm-research-lab-tls
  namespace: llm-research-lab-prod
spec:
  secretName: llm-research-lab-tls
  issuerRef:
    name: letsencrypt-prod
    kind: ClusterIssuer
  dnsNames:
  - api.llm-research-lab.company.com
  - research.llm-research-lab.company.com
```

### 4.5.3 Rate Limiting

Implement rate limiting at ingress level.

```yaml
# ingress/api-ingress-with-rate-limit.yaml
apiVersion: networking.k8s.io/v1
kind: Ingress
metadata:
  name: llm-research-api
  namespace: llm-research-lab-prod
  annotations:
    cert-manager.io/cluster-issuer: letsencrypt-prod
    nginx.ingress.kubernetes.io/ssl-redirect: "true"
    nginx.ingress.kubernetes.io/force-ssl-redirect: "true"

    # Rate limiting - 100 requests per minute per IP
    nginx.ingress.kubernetes.io/limit-rps: "100"
    nginx.ingress.kubernetes.io/limit-burst-multiplier: "5"

    # Connection limiting
    nginx.ingress.kubernetes.io/limit-connections: "20"

    # Request body size
    nginx.ingress.kubernetes.io/proxy-body-size: "10m"

    # CORS
    nginx.ingress.kubernetes.io/enable-cors: "true"
    nginx.ingress.kubernetes.io/cors-allow-origin: "https://dashboard.company.com"
    nginx.ingress.kubernetes.io/cors-allow-methods: "GET, POST, PUT, DELETE, OPTIONS"
    nginx.ingress.kubernetes.io/cors-allow-credentials: "true"

    # Security headers
    nginx.ingress.kubernetes.io/configuration-snippet: |
      more_set_headers "X-Frame-Options: DENY";
      more_set_headers "X-Content-Type-Options: nosniff";
      more_set_headers "X-XSS-Protection: 1; mode=block";
      more_set_headers "Strict-Transport-Security: max-age=31536000; includeSubDomains";
spec:
  ingressClassName: nginx
  tls:
  - hosts:
    - api.llm-research-lab.company.com
    secretName: llm-research-lab-tls
  rules:
  - host: api.llm-research-lab.company.com
    http:
      paths:
      - path: /
        pathType: Prefix
        backend:
          service:
            name: llm-research-api
            port:
              number: 8080
```

### 4.5.4 Request Routing Rules

Advanced routing with canary deployments and A/B testing.

```yaml
# ingress/canary-routing.yaml
apiVersion: networking.k8s.io/v1
kind: Ingress
metadata:
  name: llm-research-api-stable
  namespace: llm-research-lab-prod
  annotations:
    cert-manager.io/cluster-issuer: letsencrypt-prod
spec:
  ingressClassName: nginx
  tls:
  - hosts:
    - api.llm-research-lab.company.com
    secretName: llm-research-lab-tls
  rules:
  - host: api.llm-research-lab.company.com
    http:
      paths:
      - path: /
        pathType: Prefix
        backend:
          service:
            name: llm-research-api-stable
            port:
              number: 8080
---
# Canary ingress - 10% traffic to new version
apiVersion: networking.k8s.io/v1
kind: Ingress
metadata:
  name: llm-research-api-canary
  namespace: llm-research-lab-prod
  annotations:
    cert-manager.io/cluster-issuer: letsencrypt-prod
    nginx.ingress.kubernetes.io/canary: "true"
    nginx.ingress.kubernetes.io/canary-weight: "10"
    # Alternative: Header-based routing
    # nginx.ingress.kubernetes.io/canary-by-header: "X-Canary"
    # nginx.ingress.kubernetes.io/canary-by-header-value: "always"
spec:
  ingressClassName: nginx
  tls:
  - hosts:
    - api.llm-research-lab.company.com
    secretName: llm-research-lab-tls
  rules:
  - host: api.llm-research-lab.company.com
    http:
      paths:
      - path: /
        pathType: Prefix
        backend:
          service:
            name: llm-research-api-canary
            port:
              number: 8080
```

### 4.5.5 Health Check Configuration

```yaml
# ingress/health-check-ingress.yaml
apiVersion: networking.k8s.io/v1
kind: Ingress
metadata:
  name: llm-research-health
  namespace: llm-research-lab-prod
  annotations:
    nginx.ingress.kubernetes.io/upstream-health-checks: "true"
    nginx.ingress.kubernetes.io/upstream-health-checks-interval: "10s"
    nginx.ingress.kubernetes.io/upstream-health-checks-timeout: "5s"
    nginx.ingress.kubernetes.io/upstream-health-checks-port: "8080"
    nginx.ingress.kubernetes.io/upstream-health-checks-path: "/health/ready"
    nginx.ingress.kubernetes.io/upstream-health-checks-mandatory: "true"
spec:
  ingressClassName: nginx
  rules:
  - host: api.llm-research-lab.company.com
    http:
      paths:
      - path: /health
        pathType: Prefix
        backend:
          service:
            name: llm-research-api
            port:
              number: 8080
```

---

## 4.6 Service Mesh (Optional)

### 4.6.1 Istio Configuration

Deploy Istio for advanced traffic management and security.

#### Istio Installation

```yaml
# service-mesh/istio-operator.yaml
apiVersion: install.istio.io/v1alpha1
kind: IstioOperator
metadata:
  name: llm-research-istio
  namespace: istio-system
spec:
  profile: production

  components:
    pilot:
      k8s:
        resources:
          requests:
            cpu: 500m
            memory: 2Gi
          limits:
            cpu: 2000m
            memory: 4Gi
        hpaSpec:
          minReplicas: 2
          maxReplicas: 5

    ingressGateways:
    - name: istio-ingressgateway
      enabled: true
      k8s:
        resources:
          requests:
            cpu: 1000m
            memory: 1Gi
          limits:
            cpu: 2000m
            memory: 2Gi
        hpaSpec:
          minReplicas: 3
          maxReplicas: 10
        service:
          type: LoadBalancer

    egressGateways:
    - name: istio-egressgateway
      enabled: true
      k8s:
        resources:
          requests:
            cpu: 500m
            memory: 512Mi
          limits:
            cpu: 1000m
            memory: 1Gi

  meshConfig:
    accessLogFile: /dev/stdout
    accessLogEncoding: JSON

    enableTracing: true
    defaultConfig:
      tracing:
        zipkin:
          address: jaeger-collector.observability:9411
        sampling: 1.0  # 100% sampling for production

    # Telemetry
    enablePrometheusMerge: true

  values:
    global:
      proxy:
        resources:
          requests:
            cpu: 100m
            memory: 128Mi
          limits:
            cpu: 2000m
            memory: 1Gi

        # Lifecycle configuration
        holdApplicationUntilProxyStarts: true

      # mTLS settings
      mtls:
        enabled: true
        auto: true
```

### 4.6.2 mTLS Between Services

Enforce mutual TLS for service-to-service communication.

```yaml
# service-mesh/peer-authentication.yaml
apiVersion: security.istio.io/v1beta1
kind: PeerAuthentication
metadata:
  name: default
  namespace: llm-research-lab-prod
spec:
  mtls:
    mode: STRICT
---
# Authorization policy
apiVersion: security.istio.io/v1beta1
kind: AuthorizationPolicy
metadata:
  name: allow-same-namespace
  namespace: llm-research-lab-prod
spec:
  action: ALLOW
  rules:
  - from:
    - source:
        namespaces: ["llm-research-lab-prod"]
---
# Allow specific service communication
apiVersion: security.istio.io/v1beta1
kind: AuthorizationPolicy
metadata:
  name: api-to-database
  namespace: llm-research-lab-data
spec:
  selector:
    matchLabels:
      app: pgbouncer
  action: ALLOW
  rules:
  - from:
    - source:
        principals: ["cluster.local/ns/llm-research-lab-prod/sa/llm-api-sa"]
    to:
    - operation:
        ports: ["5432"]
```

### 4.6.3 Traffic Management Policies

#### Circuit Breaking

```yaml
# service-mesh/destination-rule-circuit-breaker.yaml
apiVersion: networking.istio.io/v1beta1
kind: DestinationRule
metadata:
  name: llm-api-circuit-breaker
  namespace: llm-research-lab-prod
spec:
  host: llm-research-api.llm-research-lab-prod.svc.cluster.local
  trafficPolicy:
    connectionPool:
      tcp:
        maxConnections: 100
      http:
        http1MaxPendingRequests: 50
        http2MaxRequests: 100
        maxRequestsPerConnection: 2

    outlierDetection:
      consecutiveErrors: 5
      interval: 30s
      baseEjectionTime: 60s
      maxEjectionPercent: 50
      minHealthPercent: 30

    loadBalancer:
      simple: LEAST_REQUEST
```

#### Request Timeout and Retry

```yaml
# service-mesh/virtual-service-retry.yaml
apiVersion: networking.istio.io/v1beta1
kind: VirtualService
metadata:
  name: llm-api-resilience
  namespace: llm-research-lab-prod
spec:
  hosts:
  - llm-research-api.llm-research-lab-prod.svc.cluster.local
  http:
  - match:
    - uri:
        prefix: /api/v1/experiments
    timeout: 30s
    retries:
      attempts: 3
      perTryTimeout: 10s
      retryOn: 5xx,reset,connect-failure,refused-stream
    route:
    - destination:
        host: llm-research-api.llm-research-lab-prod.svc.cluster.local
        port:
          number: 8080
```

#### Traffic Splitting

```yaml
# service-mesh/virtual-service-traffic-split.yaml
apiVersion: networking.istio.io/v1beta1
kind: VirtualService
metadata:
  name: llm-api-traffic-split
  namespace: llm-research-lab-prod
spec:
  hosts:
  - api.llm-research-lab.company.com
  gateways:
  - llm-gateway
  http:
  - match:
    - headers:
        user-agent:
          regex: ".*mobile.*"
    route:
    - destination:
        host: llm-research-api-mobile
        subset: v2
      weight: 100

  - route:
    - destination:
        host: llm-research-api
        subset: v1
      weight: 90
    - destination:
        host: llm-research-api
        subset: v2
      weight: 10
---
apiVersion: networking.istio.io/v1beta1
kind: DestinationRule
metadata:
  name: llm-api-subsets
  namespace: llm-research-lab-prod
spec:
  host: llm-research-api.llm-research-lab-prod.svc.cluster.local
  subsets:
  - name: v1
    labels:
      version: v1.0.0
  - name: v2
    labels:
      version: v2.0.0
```

#### Rate Limiting with Envoy Filter

```yaml
# service-mesh/envoy-rate-limit.yaml
apiVersion: networking.istio.io/v1alpha3
kind: EnvoyFilter
metadata:
  name: rate-limit-filter
  namespace: istio-system
spec:
  workloadSelector:
    labels:
      app: istio-ingressgateway
  configPatches:
  - applyTo: HTTP_FILTER
    match:
      context: GATEWAY
      listener:
        filterChain:
          filter:
            name: "envoy.filters.network.http_connection_manager"
            subFilter:
              name: "envoy.filters.http.router"
    patch:
      operation: INSERT_BEFORE
      value:
        name: envoy.filters.http.local_ratelimit
        typed_config:
          "@type": type.googleapis.com/udpa.type.v1.TypedStruct
          type_url: type.googleapis.com/envoy.extensions.filters.http.local_ratelimit.v3.LocalRateLimit
          value:
            stat_prefix: http_local_rate_limiter
            token_bucket:
              max_tokens: 1000
              tokens_per_fill: 1000
              fill_interval: 60s
            filter_enabled:
              runtime_key: local_rate_limit_enabled
              default_value:
                numerator: 100
                denominator: HUNDRED
            filter_enforced:
              runtime_key: local_rate_limit_enforced
              default_value:
                numerator: 100
                denominator: HUNDRED
```

### 4.6.4 Gateway Configuration

```yaml
# service-mesh/istio-gateway.yaml
apiVersion: networking.istio.io/v1beta1
kind: Gateway
metadata:
  name: llm-gateway
  namespace: llm-research-lab-prod
spec:
  selector:
    istio: ingressgateway
  servers:
  - port:
      number: 443
      name: https
      protocol: HTTPS
    tls:
      mode: SIMPLE
      credentialName: llm-research-lab-tls
    hosts:
    - api.llm-research-lab.company.com
    - research.llm-research-lab.company.com

  - port:
      number: 80
      name: http
      protocol: HTTP
    hosts:
    - api.llm-research-lab.company.com
    - research.llm-research-lab.company.com
    tls:
      httpsRedirect: true
---
apiVersion: networking.istio.io/v1beta1
kind: VirtualService
metadata:
  name: llm-gateway-routes
  namespace: llm-research-lab-prod
spec:
  hosts:
  - api.llm-research-lab.company.com
  gateways:
  - llm-gateway
  http:
  - match:
    - uri:
        prefix: /api/v1/
    route:
    - destination:
        host: llm-research-api.llm-research-lab-prod.svc.cluster.local
        port:
          number: 8080
```

### 4.6.5 Observability with Istio

```yaml
# service-mesh/telemetry.yaml
apiVersion: telemetry.istio.io/v1alpha1
kind: Telemetry
metadata:
  name: mesh-telemetry
  namespace: istio-system
spec:
  accessLogging:
  - providers:
    - name: envoy

  metrics:
  - providers:
    - name: prometheus
    dimensions:
      request_protocol: request.protocol
      response_code: response.code
      source_workload: source.workload.name
      destination_workload: destination.workload.name

  tracing:
  - providers:
    - name: jaeger
    randomSamplingPercentage: 100.0
    customTags:
      environment:
        literal:
          value: production
      version:
        environment:
          name: APP_VERSION
```

---

## 4.7 Helm Chart for Complete Deployment

### 4.7.1 Helm Values Production

```yaml
# helm/values-production.yaml
global:
  environment: production
  domain: llm-research-lab.company.com
  storageClass: fast-ssd

  registry:
    url: ghcr.io/company
    imagePullSecrets:
    - name: ghcr-pull-secret

api:
  replicaCount: 3
  image:
    repository: ghcr.io/company/llm-research-api
    tag: "v1.0.0"
    pullPolicy: IfNotPresent

  resources:
    requests:
      cpu: "2"
      memory: 4Gi
    limits:
      cpu: "4"
      memory: 8Gi

  autoscaling:
    enabled: true
    minReplicas: 3
    maxReplicas: 20
    targetCPU: 70
    targetMemory: 80

  service:
    type: ClusterIP
    port: 8080

  ingress:
    enabled: true
    className: nginx
    annotations:
      cert-manager.io/cluster-issuer: letsencrypt-prod
      nginx.ingress.kubernetes.io/limit-rps: "100"
    hosts:
    - host: api.llm-research-lab.company.com
      paths:
      - path: /
        pathType: Prefix
    tls:
    - secretName: llm-api-tls
      hosts:
      - api.llm-research-lab.company.com

worker:
  replicaCount: 5
  image:
    repository: ghcr.io/company/llm-research-worker
    tag: "v1.0.0"

  resources:
    requests:
      cpu: "4"
      memory: 8Gi
    limits:
      cpu: "8"
      memory: 16Gi

  autoscaling:
    enabled: true
    minReplicas: 2
    maxReplicas: 50

postgresql:
  enabled: true
  instances: 3
  storage:
    size: 500Gi
  backup:
    enabled: true
    schedule: "0 2 * * *"
    retentionDays: 30

clickhouse:
  enabled: true
  shards: 3
  replicas: 2
  storage:
    size: 1Ti

kafka:
  enabled: true
  replicas: 5
  zookeeper:
    replicas: 3
  storage:
    size: 1Ti

redis:
  enabled: true
  cluster:
    enabled: true
    nodes: 6
  storage:
    size: 100Gi

monitoring:
  prometheus:
    enabled: true
    retention: 30d
    storage: 500Gi

  grafana:
    enabled: true
    adminPassword: <from-secret>

  jaeger:
    enabled: true
    storage:
      type: elasticsearch

serviceMesh:
  istio:
    enabled: false  # Optional
```

---

## Document Metadata

| Field | Value |
|-------|-------|
| **Version** | 1.0.0 |
| **Status** | Complete |
| **SPARC Phase** | Phase 5 - Completion |
| **Section** | 4 - Production Infrastructure |
| **Created** | 2025-11-28 |
| **Target SLA** | 99.9% |
| **Tech Stack** | Kubernetes, PostgreSQL, ClickHouse, Kafka, Redis |

---

## Next Steps

1. **Review and Validate**: Infrastructure team reviews all configurations
2. **Provision Infrastructure**: Set up Kubernetes clusters and node pools
3. **Deploy Base Components**: Install operators (Strimzi, CloudNativePG, Redis Operator)
4. **Configure Networking**: Set up ingress controllers and service mesh
5. **Deploy Data Layer**: PostgreSQL, ClickHouse, and connection pooling
6. **Deploy Message Queue**: Kafka cluster with topics and consumer groups
7. **Deploy Caching**: Redis cluster/sentinel configuration
8. **Security Hardening**: Network policies, RBAC, secrets management
9. **Monitoring Setup**: Prometheus, Grafana, Jaeger deployment
10. **Load Testing**: Validate infrastructure under expected load
11. **Disaster Recovery**: Test backup/restore and failover procedures
12. **Documentation**: Create runbooks and operational procedures

---

*This document is part of the SPARC Phase 5 (Completion) specification for LLM-Research-Lab, providing production-ready infrastructure configurations for enterprise-grade deployment.*
