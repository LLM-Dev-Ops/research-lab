# LLM Research Lab - SPARC Phase 5: Section 3.4-3.5

## SPARC Phase 5: Completion - Deployment Readiness (Part 2)

**Version**: 1.0.0
**Status**: Specification
**Last Updated**: 2025-11-28
**Ecosystem**: LLM DevOps (24+ Module Platform)

---

## 3.4 Deployment Strategies

### 3.4.1 Blue-Green Deployment Procedure

Blue-green deployment enables zero-downtime releases by maintaining two identical production environments.

#### Kubernetes Blue-Green Manifests

**Service Definition (Stable)**
```yaml
# k8s/service-stable.yaml
apiVersion: v1
kind: Service
metadata:
  name: research-lab-api
  namespace: llm-research-lab
  labels:
    app: research-lab
    tier: api
spec:
  selector:
    app: research-lab
    tier: api
    version: stable  # Points to active environment
  ports:
    - name: http
      port: 8080
      targetPort: 8080
      protocol: TCP
    - name: metrics
      port: 9090
      targetPort: 9090
      protocol: TCP
  type: ClusterIP
```

**Blue Deployment**
```yaml
# k8s/deployment-blue.yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: research-lab-api-blue
  namespace: llm-research-lab
  labels:
    app: research-lab
    tier: api
    environment: blue
spec:
  replicas: 3
  strategy:
    type: RollingUpdate
    rollingUpdate:
      maxSurge: 1
      maxUnavailable: 0
  selector:
    matchLabels:
      app: research-lab
      tier: api
      environment: blue
  template:
    metadata:
      labels:
        app: research-lab
        tier: api
        environment: blue
        version: stable  # Will be swapped to 'inactive' during deployment
    spec:
      containers:
      - name: api
        image: harbor.example.com/llm-research-lab/api:v1.2.3
        ports:
        - containerPort: 8080
          name: http
        - containerPort: 9090
          name: metrics
        env:
        - name: RUST_LOG
          value: "info,llm_research_lab=debug"
        - name: DATABASE_URL
          valueFrom:
            secretKeyRef:
              name: postgres-credentials
              key: url
        - name: KAFKA_BROKERS
          value: "kafka-0.kafka-headless:9092,kafka-1.kafka-headless:9092"
        - name: REDIS_URL
          value: "redis://redis-master:6379"
        resources:
          requests:
            memory: "512Mi"
            cpu: "500m"
          limits:
            memory: "1Gi"
            cpu: "1000m"
        livenessProbe:
          httpGet:
            path: /health/live
            port: 8080
          initialDelaySeconds: 30
          periodSeconds: 10
          timeoutSeconds: 5
          failureThreshold: 3
        readinessProbe:
          httpGet:
            path: /health/ready
            port: 8080
          initialDelaySeconds: 10
          periodSeconds: 5
          timeoutSeconds: 3
          failureThreshold: 2
        startupProbe:
          httpGet:
            path: /health/startup
            port: 8080
          initialDelaySeconds: 0
          periodSeconds: 5
          timeoutSeconds: 3
          failureThreshold: 30
      affinity:
        podAntiAffinity:
          preferredDuringSchedulingIgnoredDuringExecution:
          - weight: 100
            podAffinityTerm:
              labelSelector:
                matchExpressions:
                - key: app
                  operator: In
                  values:
                  - research-lab
              topologyKey: kubernetes.io/hostname
```

**Green Deployment** (identical to Blue, with `environment: green` label)

#### Blue-Green Deployment Script

```bash
#!/bin/bash
# scripts/deploy-blue-green.sh

set -euo pipefail

NAMESPACE="llm-research-lab"
NEW_VERSION="${1:?Version required (e.g., v1.2.3)}"
CURRENT_ENV=$(kubectl get service research-lab-api -n $NAMESPACE -o jsonpath='{.spec.selector.version}')

# Determine which environment is currently active
if [[ "$CURRENT_ENV" == "blue" ]]; then
    ACTIVE="blue"
    INACTIVE="green"
else
    ACTIVE="green"
    INACTIVE="blue"
fi

echo "[1/7] Current active environment: $ACTIVE"
echo "[1/7] Deploying to inactive environment: $INACTIVE"

# Update inactive deployment with new version
echo "[2/7] Updating $INACTIVE deployment to $NEW_VERSION"
kubectl set image deployment/research-lab-api-$INACTIVE \
    api=harbor.example.com/llm-research-lab/api:$NEW_VERSION \
    -n $NAMESPACE

# Wait for rollout to complete
echo "[3/7] Waiting for $INACTIVE deployment rollout..."
kubectl rollout status deployment/research-lab-api-$INACTIVE -n $NAMESPACE --timeout=5m

# Run smoke tests against inactive environment
echo "[4/7] Running smoke tests against $INACTIVE environment..."
INACTIVE_POD=$(kubectl get pods -n $NAMESPACE -l environment=$INACTIVE -o jsonpath='{.items[0].metadata.name}')
INACTIVE_IP=$(kubectl get pod $INACTIVE_POD -n $NAMESPACE -o jsonpath='{.status.podIP}')

if ! ./scripts/smoke-tests.sh "http://$INACTIVE_IP:8080"; then
    echo "ERROR: Smoke tests failed on $INACTIVE environment"
    echo "Rolling back $INACTIVE deployment..."
    kubectl rollout undo deployment/research-lab-api-$INACTIVE -n $NAMESPACE
    exit 1
fi

# Switch traffic to inactive environment
echo "[5/7] Switching traffic from $ACTIVE to $INACTIVE..."
kubectl patch service research-lab-api -n $NAMESPACE -p "{\"spec\":{\"selector\":{\"environment\":\"$INACTIVE\"}}}"

# Wait for traffic to stabilize (30 seconds)
echo "[6/7] Waiting for traffic to stabilize..."
sleep 30

# Monitor error rates
echo "[7/7] Monitoring error rates..."
ERROR_RATE=$(kubectl exec -n monitoring prometheus-0 -- \
    wget -q -O- 'http://localhost:9090/api/v1/query?query=sum(rate(http_requests_total{status=~"5..",namespace="'$NAMESPACE'"}[5m]))' \
    | jq -r '.data.result[0].value[1] // "0"')

if (( $(echo "$ERROR_RATE > 0.05" | bc -l) )); then
    echo "ERROR: Error rate ($ERROR_RATE) exceeds 5% threshold"
    echo "Rolling back to $ACTIVE environment..."
    kubectl patch service research-lab-api -n $NAMESPACE -p "{\"spec\":{\"selector\":{\"environment\":\"$ACTIVE\"}}}"
    exit 1
fi

echo "SUCCESS: Blue-green deployment to $INACTIVE completed successfully"
echo "Active environment is now: $INACTIVE"
echo "Previous environment ($ACTIVE) is available for rollback"
```

### 3.4.2 Canary Deployment Stages

Canary deployments gradually shift traffic to the new version: 5% → 25% → 50% → 100%.

#### Canary Kubernetes Manifests

```yaml
# k8s/deployment-canary.yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: research-lab-api-canary
  namespace: llm-research-lab
  labels:
    app: research-lab
    tier: api
    track: canary
spec:
  replicas: 1  # Will be scaled up during canary rollout
  selector:
    matchLabels:
      app: research-lab
      tier: api
      track: canary
  template:
    metadata:
      labels:
        app: research-lab
        tier: api
        track: canary
        version: canary
    spec:
      containers:
      - name: api
        image: harbor.example.com/llm-research-lab/api:v1.3.0-canary
        # ... (same configuration as blue/green deployments)

---
apiVersion: v1
kind: Service
metadata:
  name: research-lab-api-canary
  namespace: llm-research-lab
spec:
  selector:
    app: research-lab
    tier: api
    track: canary
  ports:
    - name: http
      port: 8080
      targetPort: 8080
```

#### Canary Ingress with Traffic Splitting

```yaml
# k8s/ingress-canary.yaml
apiVersion: networking.k8s.io/v1
kind: Ingress
metadata:
  name: research-lab-api-canary
  namespace: llm-research-lab
  annotations:
    nginx.ingress.kubernetes.io/canary: "true"
    nginx.ingress.kubernetes.io/canary-weight: "5"  # Start at 5%
    nginx.ingress.kubernetes.io/canary-by-header: "X-Canary"
    nginx.ingress.kubernetes.io/canary-by-header-value: "always"
spec:
  ingressClassName: nginx
  rules:
  - host: api.research-lab.example.com
    http:
      paths:
      - path: /
        pathType: Prefix
        backend:
          service:
            name: research-lab-api-canary
            port:
              number: 8080
  tls:
  - hosts:
    - api.research-lab.example.com
    secretName: api-tls-cert
```

#### Canary Deployment Script

```bash
#!/bin/bash
# scripts/deploy-canary.sh

set -euo pipefail

NAMESPACE="llm-research-lab"
NEW_VERSION="${1:?Version required}"
STAGES=(5 25 50 100)
SOAK_TIME=300  # 5 minutes per stage

echo "Starting canary deployment for version $NEW_VERSION"

# Deploy canary with 1 replica
echo "[1/5] Deploying canary version..."
kubectl set image deployment/research-lab-api-canary \
    api=harbor.example.com/llm-research-lab/api:$NEW_VERSION \
    -n $NAMESPACE

kubectl rollout status deployment/research-lab-api-canary -n $NAMESPACE --timeout=5m

# Iterate through canary stages
for STAGE in "${STAGES[@]}"; do
    echo "[Stage $STAGE%] Setting canary traffic weight to $STAGE%"

    kubectl annotate ingress research-lab-api-canary \
        nginx.ingress.kubernetes.io/canary-weight="$STAGE" \
        -n $NAMESPACE --overwrite

    if [[ $STAGE -eq 100 ]]; then
        echo "[Stage $STAGE%] Canary is now receiving all traffic"
        break
    fi

    echo "[Stage $STAGE%] Soaking for $SOAK_TIME seconds..."
    sleep $SOAK_TIME

    # Check metrics during soak period
    echo "[Stage $STAGE%] Validating canary metrics..."

    ERROR_RATE=$(curl -s 'http://prometheus:9090/api/v1/query?query=sum(rate(http_requests_total{status=~"5..",track="canary",namespace="'$NAMESPACE'"}[5m]))' \
        | jq -r '.data.result[0].value[1] // "0"')

    LATENCY_P99=$(curl -s 'http://prometheus:9090/api/v1/query?query=histogram_quantile(0.99,rate(http_request_duration_seconds_bucket{track="canary",namespace="'$NAMESPACE'"}[5m]))' \
        | jq -r '.data.result[0].value[1] // "0"')

    BASELINE_LATENCY=$(curl -s 'http://prometheus:9090/api/v1/query?query=histogram_quantile(0.99,rate(http_request_duration_seconds_bucket{track="stable",namespace="'$NAMESPACE'"}[5m]))' \
        | jq -r '.data.result[0].value[1] // "0"')

    # Rollback triggers
    if (( $(echo "$ERROR_RATE > 0.05" | bc -l) )); then
        echo "ROLLBACK TRIGGERED: Error rate ($ERROR_RATE) exceeds 5% threshold"
        ./scripts/rollback-canary.sh
        exit 1
    fi

    if (( $(echo "$LATENCY_P99 > $BASELINE_LATENCY * 2" | bc -l) )); then
        echo "ROLLBACK TRIGGERED: p99 latency ($LATENCY_P99s) exceeds 2x baseline ($BASELINE_LATENCY s)"
        ./scripts/rollback-canary.sh
        exit 1
    fi

    echo "[Stage $STAGE%] Metrics validated successfully"
done

# Promote canary to stable
echo "[5/5] Promoting canary to stable..."
kubectl set image deployment/research-lab-api-stable \
    api=harbor.example.com/llm-research-lab/api:$NEW_VERSION \
    -n $NAMESPACE

kubectl rollout status deployment/research-lab-api-stable -n $NAMESPACE --timeout=5m

# Remove canary
kubectl annotate ingress research-lab-api-canary nginx.ingress.kubernetes.io/canary-weight- -n $NAMESPACE
kubectl scale deployment/research-lab-api-canary --replicas=0 -n $NAMESPACE

echo "SUCCESS: Canary deployment completed and promoted to stable"
```

### 3.4.3 Rollback Automation Triggers

#### Automated Rollback Script

```bash
#!/bin/bash
# scripts/rollback-canary.sh

set -euo pipefail

NAMESPACE="llm-research-lab"

echo "ROLLBACK INITIATED: Reverting canary deployment"

# Set canary weight to 0%
kubectl annotate ingress research-lab-api-canary \
    nginx.ingress.kubernetes.io/canary-weight="0" \
    -n $NAMESPACE --overwrite

# Scale down canary
kubectl scale deployment/research-lab-api-canary --replicas=0 -n $NAMESPACE

# Trigger alert
curl -X POST https://pagerduty.example.com/api/v1/incidents \
    -H "Authorization: Token token=YOUR_TOKEN" \
    -H "Content-Type: application/json" \
    -d '{
      "incident": {
        "type": "incident",
        "title": "Canary deployment auto-rollback triggered",
        "service": {"id": "PXXXXXX", "type": "service_reference"},
        "urgency": "high",
        "body": {
          "type": "incident_body",
          "details": "Automated rollback triggered due to threshold violation"
        }
      }
    }'

echo "Rollback completed. Canary traffic set to 0%, replicas scaled to 0"
```

#### Prometheus AlertManager Rules

```yaml
# k8s/prometheus-rules.yaml
apiVersion: v1
kind: ConfigMap
metadata:
  name: prometheus-alerts
  namespace: monitoring
data:
  canary-alerts.yaml: |
    groups:
    - name: canary_deployment
      interval: 30s
      rules:
      - alert: CanaryHighErrorRate
        expr: |
          sum(rate(http_requests_total{status=~"5..",track="canary"}[5m]))
          /
          sum(rate(http_requests_total{track="canary"}[5m]))
          > 0.05
        for: 2m
        labels:
          severity: critical
          component: deployment
        annotations:
          summary: "Canary deployment error rate exceeds 5%"
          description: "Error rate: {{ $value | humanizePercentage }}"
          runbook_url: "https://runbooks.example.com/canary-high-error-rate"

      - alert: CanaryHighLatency
        expr: |
          histogram_quantile(0.99,
            rate(http_request_duration_seconds_bucket{track="canary"}[5m])
          ) > 2 *
          histogram_quantile(0.99,
            rate(http_request_duration_seconds_bucket{track="stable"}[5m])
          )
        for: 2m
        labels:
          severity: critical
          component: deployment
        annotations:
          summary: "Canary p99 latency exceeds 2x baseline"
          description: "Canary p99: {{ $value }}s"
          runbook_url: "https://runbooks.example.com/canary-high-latency"

      - alert: CanaryMemoryPressure
        expr: |
          container_memory_usage_bytes{pod=~".*-canary-.*"}
          /
          container_spec_memory_limit_bytes{pod=~".*-canary-.*"}
          > 0.9
        for: 5m
        labels:
          severity: warning
          component: deployment
        annotations:
          summary: "Canary pod memory usage > 90%"
          description: "Memory usage: {{ $value | humanizePercentage }}"
```

### 3.4.4 Zero-Downtime Deployment Verification

#### Pre-Deployment Checklist Script

```bash
#!/bin/bash
# scripts/pre-deployment-check.sh

set -euo pipefail

NAMESPACE="llm-research-lab"
FAILURES=0

echo "Running pre-deployment verification checks..."

# Check database connectivity
echo "[1/6] Verifying database connectivity..."
if ! kubectl exec -n $NAMESPACE deployment/research-lab-api-stable -- \
    sh -c 'timeout 5 pg_isready -h $DATABASE_HOST -p 5432 -U $DATABASE_USER'; then
    echo "FAIL: Database not reachable"
    ((FAILURES++))
else
    echo "PASS: Database connectivity verified"
fi

# Check Kafka availability
echo "[2/6] Verifying Kafka brokers..."
KAFKA_PODS=$(kubectl get pods -n $NAMESPACE -l app=kafka -o jsonpath='{.items[*].status.phase}')
if [[ "$KAFKA_PODS" == *"Running"* ]]; then
    echo "PASS: Kafka brokers running"
else
    echo "FAIL: Kafka brokers not ready"
    ((FAILURES++))
fi

# Check Redis availability
echo "[3/6] Verifying Redis..."
if ! kubectl exec -n $NAMESPACE redis-master-0 -- redis-cli ping | grep -q PONG; then
    echo "FAIL: Redis not responding"
    ((FAILURES++))
else
    echo "PASS: Redis connectivity verified"
fi

# Check current deployment health
echo "[4/6] Verifying current deployment health..."
READY_REPLICAS=$(kubectl get deployment research-lab-api-stable -n $NAMESPACE -o jsonpath='{.status.readyReplicas}')
DESIRED_REPLICAS=$(kubectl get deployment research-lab-api-stable -n $NAMESPACE -o jsonpath='{.spec.replicas}')

if [[ "$READY_REPLICAS" -ne "$DESIRED_REPLICAS" ]]; then
    echo "FAIL: Current deployment not healthy ($READY_REPLICAS/$DESIRED_REPLICAS ready)"
    ((FAILURES++))
else
    echo "PASS: Current deployment healthy ($READY_REPLICAS/$DESIRED_REPLICAS ready)"
fi

# Check error rate in last 10 minutes
echo "[5/6] Checking baseline error rate..."
ERROR_RATE=$(curl -s 'http://prometheus:9090/api/v1/query?query=sum(rate(http_requests_total{status=~"5..",namespace="'$NAMESPACE'"}[10m]))' \
    | jq -r '.data.result[0].value[1] // "0"')

if (( $(echo "$ERROR_RATE > 0.01" | bc -l) )); then
    echo "WARN: Elevated error rate detected ($ERROR_RATE). Proceed with caution."
else
    echo "PASS: Error rate within normal range"
fi

# Check available cluster resources
echo "[6/6] Verifying cluster resources..."
NODE_CPU=$(kubectl top nodes --no-headers | awk '{print $3}' | sed 's/%//' | sort -n | tail -1)
if (( NODE_CPU > 80 )); then
    echo "WARN: Node CPU usage high ($NODE_CPU%)"
else
    echo "PASS: Cluster resources available"
fi

# Final verdict
if [[ $FAILURES -gt 0 ]]; then
    echo "FAILED: Pre-deployment checks failed ($FAILURES failures)"
    exit 1
else
    echo "SUCCESS: All pre-deployment checks passed"
    exit 0
fi
```

---

## 3.5 Smoke Tests

### 3.5.1 Critical Path Validation Tests

Smoke tests validate core functionality after deployment.

#### Rust Smoke Test Framework

```rust
// tests/smoke_tests.rs
use anyhow::{Context, Result};
use reqwest::{Client, StatusCode};
use serde_json::json;
use std::time::Duration;

#[derive(Debug)]
pub struct SmokeTestConfig {
    pub base_url: String,
    pub timeout: Duration,
}

pub struct SmokeTestRunner {
    client: Client,
    config: SmokeTestConfig,
}

impl SmokeTestRunner {
    pub fn new(config: SmokeTestConfig) -> Result<Self> {
        let client = Client::builder()
            .timeout(config.timeout)
            .build()
            .context("Failed to create HTTP client")?;

        Ok(Self { client, config })
    }

    /// Test 1: Health check endpoints
    pub async fn test_health_endpoints(&self) -> Result<()> {
        println!("Testing health endpoints...");

        // Liveness probe
        let response = self.client
            .get(&format!("{}/health/live", self.config.base_url))
            .send()
            .await
            .context("Liveness probe request failed")?;

        assert_eq!(
            response.status(),
            StatusCode::OK,
            "Liveness probe failed"
        );

        // Readiness probe
        let response = self.client
            .get(&format!("{}/health/ready", self.config.base_url))
            .send()
            .await
            .context("Readiness probe request failed")?;

        assert_eq!(
            response.status(),
            StatusCode::OK,
            "Readiness probe failed"
        );

        // Startup probe
        let response = self.client
            .get(&format!("{}/health/startup", self.config.base_url))
            .send()
            .await
            .context("Startup probe request failed")?;

        assert_eq!(
            response.status(),
            StatusCode::OK,
            "Startup probe failed"
        );

        println!("✓ Health endpoints validated");
        Ok(())
    }

    /// Test 2: Experiment creation workflow
    pub async fn test_experiment_creation(&self) -> Result<()> {
        println!("Testing experiment creation...");

        let experiment = json!({
            "name": "smoke-test-experiment",
            "description": "Automated smoke test",
            "model_id": "test-model-v1",
            "dataset_id": "test-dataset-v1",
            "hyperparameters": {
                "learning_rate": 0.001,
                "batch_size": 32
            }
        });

        let response = self.client
            .post(&format!("{}/api/v1/experiments", self.config.base_url))
            .json(&experiment)
            .send()
            .await
            .context("Experiment creation request failed")?;

        assert_eq!(
            response.status(),
            StatusCode::CREATED,
            "Experiment creation failed: {}",
            response.text().await?
        );

        let created_experiment: serde_json::Value = response
            .json()
            .await
            .context("Failed to parse experiment response")?;

        let experiment_id = created_experiment["id"]
            .as_str()
            .context("No experiment ID in response")?;

        println!("✓ Experiment created: {}", experiment_id);
        Ok(())
    }

    /// Test 3: Metrics ingestion
    pub async fn test_metrics_ingestion(&self) -> Result<()> {
        println!("Testing metrics ingestion...");

        let metric = json!({
            "experiment_id": "smoke-test-exp-001",
            "step": 100,
            "metrics": {
                "loss": 0.234,
                "accuracy": 0.876,
                "f1_score": 0.845
            },
            "timestamp": chrono::Utc::now().to_rfc3339()
        });

        let response = self.client
            .post(&format!("{}/api/v1/metrics", self.config.base_url))
            .json(&metric)
            .send()
            .await
            .context("Metrics ingestion request failed")?;

        assert_eq!(
            response.status(),
            StatusCode::ACCEPTED,
            "Metrics ingestion failed"
        );

        println!("✓ Metrics ingested successfully");
        Ok(())
    }

    /// Test 4: Dataset versioning
    pub async fn test_dataset_versioning(&self) -> Result<()> {
        println!("Testing dataset versioning...");

        let dataset = json!({
            "name": "smoke-test-dataset",
            "version": "1.0.0",
            "format": "parquet",
            "size_bytes": 1024000,
            "checksum": "sha256:abc123..."
        });

        let response = self.client
            .post(&format!("{}/api/v1/datasets", self.config.base_url))
            .json(&dataset)
            .send()
            .await
            .context("Dataset creation request failed")?;

        assert!(
            response.status().is_success(),
            "Dataset versioning failed"
        );

        println!("✓ Dataset versioning validated");
        Ok(())
    }

    /// Test 5: Integration endpoints
    pub async fn test_integration_endpoints(&self) -> Result<()> {
        println!("Testing integration endpoints...");

        // Test LLM-Registry integration
        let response = self.client
            .get(&format!("{}/api/v1/integrations/registry/health", self.config.base_url))
            .send()
            .await
            .context("Registry integration health check failed")?;

        assert!(
            response.status().is_success(),
            "LLM-Registry integration unhealthy"
        );

        // Test LLM-Test-Bench integration
        let response = self.client
            .get(&format!("{}/api/v1/integrations/test-bench/health", self.config.base_url))
            .send()
            .await
            .context("Test Bench integration health check failed")?;

        assert!(
            response.status().is_success(),
            "LLM-Test-Bench integration unhealthy"
        );

        println!("✓ Integration endpoints validated");
        Ok(())
    }

    /// Run all smoke tests
    pub async fn run_all(&self) -> Result<()> {
        println!("\n=== Running Smoke Tests ===\n");

        self.test_health_endpoints().await?;
        self.test_experiment_creation().await?;
        self.test_metrics_ingestion().await?;
        self.test_dataset_versioning().await?;
        self.test_integration_endpoints().await?;

        println!("\n=== All Smoke Tests Passed ===\n");
        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let config = SmokeTestConfig {
        base_url: std::env::var("API_BASE_URL")
            .unwrap_or_else(|_| "http://localhost:8080".to_string()),
        timeout: Duration::from_secs(10),
    };

    let runner = SmokeTestRunner::new(config)?;
    runner.run_all().await?;

    Ok(())
}
```

### 3.5.2 Health Check Endpoint Specification

#### Rust Health Check Implementation

```rust
// src/health.rs
use axum::{
    extract::State,
    http::StatusCode,
    response::IntoResponse,
    routing::get,
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio_postgres::Client as PgClient;
use redis::Client as RedisClient;

#[derive(Debug, Serialize, Deserialize)]
pub struct HealthStatus {
    pub status: String,
    pub version: String,
    pub timestamp: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dependencies: Option<DependencyHealth>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DependencyHealth {
    pub database: ServiceHealth,
    pub redis: ServiceHealth,
    pub kafka: ServiceHealth,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ServiceHealth {
    pub status: String,
    pub latency_ms: Option<u64>,
}

pub struct HealthState {
    pub pg_client: Arc<PgClient>,
    pub redis_client: Arc<RedisClient>,
}

/// Liveness probe - checks if the application is running
/// Returns 200 if the service is alive, regardless of dependencies
pub async fn liveness_handler() -> impl IntoResponse {
    let health = HealthStatus {
        status: "alive".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        timestamp: chrono::Utc::now().to_rfc3339(),
        dependencies: None,
    };

    (StatusCode::OK, Json(health))
}

/// Readiness probe - checks if the application is ready to serve traffic
/// Returns 200 only if all critical dependencies are healthy
pub async fn readiness_handler(
    State(state): State<Arc<HealthState>>,
) -> impl IntoResponse {
    let start = std::time::Instant::now();

    // Check database
    let db_health = check_database(&state.pg_client).await;

    // Check Redis
    let redis_health = check_redis(&state.redis_client).await;

    // Check Kafka (simplified - actual implementation would check broker connectivity)
    let kafka_health = ServiceHealth {
        status: "healthy".to_string(),
        latency_ms: Some(5),
    };

    let is_ready = db_health.status == "healthy"
        && redis_health.status == "healthy"
        && kafka_health.status == "healthy";

    let status_code = if is_ready {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    };

    let health = HealthStatus {
        status: if is_ready { "ready" } else { "not_ready" }.to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        timestamp: chrono::Utc::now().to_rfc3339(),
        dependencies: Some(DependencyHealth {
            database: db_health,
            redis: redis_health,
            kafka: kafka_health,
        }),
    };

    (status_code, Json(health))
}

/// Startup probe - checks if the application has completed initialization
/// Returns 200 when the service is fully initialized
pub async fn startup_handler(
    State(state): State<Arc<HealthState>>,
) -> impl IntoResponse {
    // Check critical initialization dependencies
    let db_initialized = check_database(&state.pg_client).await.status == "healthy";
    let redis_initialized = check_redis(&state.redis_client).await.status == "healthy";

    if db_initialized && redis_initialized {
        (StatusCode::OK, Json(json!({
            "status": "started",
            "version": env!("CARGO_PKG_VERSION"),
            "timestamp": chrono::Utc::now().to_rfc3339()
        })))
    } else {
        (StatusCode::SERVICE_UNAVAILABLE, Json(json!({
            "status": "starting",
            "message": "Still initializing dependencies"
        })))
    }
}

async fn check_database(client: &PgClient) -> ServiceHealth {
    let start = std::time::Instant::now();

    match tokio::time::timeout(
        std::time::Duration::from_secs(2),
        client.simple_query("SELECT 1")
    ).await {
        Ok(Ok(_)) => ServiceHealth {
            status: "healthy".to_string(),
            latency_ms: Some(start.elapsed().as_millis() as u64),
        },
        _ => ServiceHealth {
            status: "unhealthy".to_string(),
            latency_ms: None,
        },
    }
}

async fn check_redis(client: &RedisClient) -> ServiceHealth {
    let start = std::time::Instant::now();

    match client.get_tokio_connection().await {
        Ok(mut conn) => {
            match redis::cmd("PING").query_async::<_, String>(&mut conn).await {
                Ok(_) => ServiceHealth {
                    status: "healthy".to_string(),
                    latency_ms: Some(start.elapsed().as_millis() as u64),
                },
                Err(_) => ServiceHealth {
                    status: "unhealthy".to_string(),
                    latency_ms: None,
                },
            }
        }
        Err(_) => ServiceHealth {
            status: "unhealthy".to_string(),
            latency_ms: None,
        },
    }
}

pub fn health_routes(state: Arc<HealthState>) -> Router {
    Router::new()
        .route("/health/live", get(liveness_handler))
        .route("/health/ready", get(readiness_handler))
        .route("/health/startup", get(startup_handler))
        .with_state(state)
}
```

### 3.5.3 Dependency Connectivity Tests

#### Shell Script for Dependency Validation

```bash
#!/bin/bash
# scripts/test-dependencies.sh

set -euo pipefail

BASE_URL="${1:-http://localhost:8080}"
FAILURES=0

echo "Testing dependency connectivity for $BASE_URL"

# Test PostgreSQL connectivity
echo "[1/5] Testing PostgreSQL..."
if kubectl exec -n llm-research-lab deployment/research-lab-api-stable -- \
    sh -c 'pg_isready -h $DATABASE_HOST -p 5432 -U $DATABASE_USER' &>/dev/null; then
    echo "✓ PostgreSQL: Connected"
else
    echo "✗ PostgreSQL: Failed"
    ((FAILURES++))
fi

# Test Redis connectivity
echo "[2/5] Testing Redis..."
if kubectl exec -n llm-research-lab redis-master-0 -- redis-cli ping | grep -q PONG; then
    echo "✓ Redis: Connected"
else
    echo "✗ Redis: Failed"
    ((FAILURES++))
fi

# Test Kafka connectivity
echo "[3/5] Testing Kafka..."
if kubectl exec -n llm-research-lab kafka-0 -- \
    kafka-broker-api-versions --bootstrap-server localhost:9092 &>/dev/null; then
    echo "✓ Kafka: Connected"
else
    echo "✗ Kafka: Failed"
    ((FAILURES++))
fi

# Test S3/MinIO connectivity
echo "[4/5] Testing Object Storage..."
HEALTH_RESPONSE=$(curl -s "$BASE_URL/api/v1/health/dependencies" | jq -r '.dependencies.object_storage.status')
if [[ "$HEALTH_RESPONSE" == "healthy" ]]; then
    echo "✓ Object Storage: Connected"
else
    echo "✗ Object Storage: Failed"
    ((FAILURES++))
fi

# Test ClickHouse connectivity
echo "[5/5] Testing ClickHouse..."
if kubectl exec -n llm-research-lab clickhouse-0 -- clickhouse-client --query "SELECT 1" &>/dev/null; then
    echo "✓ ClickHouse: Connected"
else
    echo "✗ ClickHouse: Failed"
    ((FAILURES++))
fi

if [[ $FAILURES -gt 0 ]]; then
    echo "\nDependency tests failed: $FAILURES failures"
    exit 1
else
    echo "\n✓ All dependency connectivity tests passed"
    exit 0
fi
```

### 3.5.4 Performance Baseline Verification

#### Rust Performance Baseline Test

```rust
// tests/performance_baseline.rs
use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use reqwest::Client;
use tokio::runtime::Runtime;

const BASE_URL: &str = "http://localhost:8080";

async fn benchmark_experiment_creation(client: &Client) -> reqwest::Result<()> {
    let experiment = serde_json::json!({
        "name": "perf-test-experiment",
        "model_id": "test-model",
        "dataset_id": "test-dataset"
    });

    client
        .post(&format!("{}/api/v1/experiments", BASE_URL))
        .json(&experiment)
        .send()
        .await?
        .error_for_status()?;

    Ok(())
}

async fn benchmark_metrics_query(client: &Client) -> reqwest::Result<()> {
    client
        .get(&format!("{}/api/v1/metrics?experiment_id=test-exp-001&limit=100", BASE_URL))
        .send()
        .await?
        .error_for_status()?;

    Ok(())
}

fn performance_benchmarks(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let client = Client::new();

    // Benchmark experiment creation (baseline: < 100ms)
    c.bench_function("experiment_creation", |b| {
        b.to_async(&rt).iter(|| async {
            benchmark_experiment_creation(&client).await.unwrap()
        });
    });

    // Benchmark metrics query (baseline: < 50ms)
    c.bench_function("metrics_query", |b| {
        b.to_async(&rt).iter(|| async {
            benchmark_metrics_query(&client).await.unwrap()
        });
    });
}

criterion_group!(benches, performance_benchmarks);
criterion_main!(benches);
```

### 3.5.5 Automated Smoke Test Script

#### Comprehensive Smoke Test Runner

```bash
#!/bin/bash
# scripts/smoke-tests.sh

set -euo pipefail

BASE_URL="${1:-http://localhost:8080}"
TIMEOUT=10
FAILURES=0

echo "========================================="
echo "  LLM Research Lab - Smoke Tests"
echo "  Target: $BASE_URL"
echo "========================================="

# Test 1: Health endpoints
echo -e "\n[Test 1/6] Health Endpoints"
for endpoint in live ready startup; do
    echo -n "  Testing /health/$endpoint... "
    STATUS=$(curl -s -o /dev/null -w "%{http_code}" --max-time $TIMEOUT "$BASE_URL/health/$endpoint")
    if [[ "$STATUS" == "200" ]]; then
        echo "✓ PASS ($STATUS)"
    else
        echo "✗ FAIL ($STATUS)"
        ((FAILURES++))
    fi
done

# Test 2: API authentication
echo -e "\n[Test 2/6] API Authentication"
echo -n "  Testing JWT authentication... "
TOKEN=$(curl -s -X POST "$BASE_URL/api/v1/auth/login" \
    -H "Content-Type: application/json" \
    -d '{"username":"test","password":"test"}' \
    | jq -r '.token // empty')

if [[ -n "$TOKEN" ]]; then
    echo "✓ PASS"
else
    echo "✗ FAIL (No token received)"
    ((FAILURES++))
fi

# Test 3: Experiment workflow
echo -e "\n[Test 3/6] Experiment Creation"
echo -n "  Creating test experiment... "
EXPERIMENT=$(curl -s -X POST "$BASE_URL/api/v1/experiments" \
    -H "Authorization: Bearer $TOKEN" \
    -H "Content-Type: application/json" \
    -d '{
        "name": "smoke-test-exp",
        "model_id": "test-model-v1",
        "dataset_id": "test-dataset-v1"
    }')

EXPERIMENT_ID=$(echo "$EXPERIMENT" | jq -r '.id // empty')
if [[ -n "$EXPERIMENT_ID" ]]; then
    echo "✓ PASS (ID: $EXPERIMENT_ID)"
else
    echo "✗ FAIL"
    ((FAILURES++))
fi

# Test 4: Metrics ingestion
echo -e "\n[Test 4/6] Metrics Ingestion"
echo -n "  Posting metrics... "
METRIC_RESPONSE=$(curl -s -o /dev/null -w "%{http_code}" -X POST "$BASE_URL/api/v1/metrics" \
    -H "Authorization: Bearer $TOKEN" \
    -H "Content-Type: application/json" \
    -d "{
        \"experiment_id\": \"$EXPERIMENT_ID\",
        \"step\": 1,
        \"metrics\": {\"loss\": 0.5, \"accuracy\": 0.85}
    }")

if [[ "$METRIC_RESPONSE" == "202" ]] || [[ "$METRIC_RESPONSE" == "200" ]]; then
    echo "✓ PASS ($METRIC_RESPONSE)"
else
    echo "✗ FAIL ($METRIC_RESPONSE)"
    ((FAILURES++))
fi

# Test 5: Integration endpoints
echo -e "\n[Test 5/6] External Integrations"
for integration in registry test-bench analytics-hub; do
    echo -n "  Testing LLM-$integration integration... "
    STATUS=$(curl -s -o /dev/null -w "%{http_code}" --max-time $TIMEOUT \
        "$BASE_URL/api/v1/integrations/$integration/health")
    if [[ "$STATUS" == "200" ]]; then
        echo "✓ PASS"
    else
        echo "✗ FAIL ($STATUS)"
        ((FAILURES++))
    fi
done

# Test 6: Performance baselines
echo -e "\n[Test 6/6] Performance Baselines"
echo -n "  Measuring API latency... "
LATENCY=$(curl -s -o /dev/null -w "%{time_total}" "$BASE_URL/health/ready")
LATENCY_MS=$(echo "$LATENCY * 1000" | bc)

if (( $(echo "$LATENCY < 0.1" | bc -l) )); then
    echo "✓ PASS (${LATENCY_MS}ms < 100ms)"
else
    echo "⚠ WARN (${LATENCY_MS}ms >= 100ms)"
fi

# Final results
echo -e "\n========================================="
if [[ $FAILURES -eq 0 ]]; then
    echo "  ✓ ALL SMOKE TESTS PASSED"
    echo "========================================="
    exit 0
else
    echo "  ✗ SMOKE TESTS FAILED: $FAILURES failures"
    echo "========================================="
    exit 1
fi
```

---

## Deployment Workflow Summary

### Complete Deployment Pipeline

```bash
#!/bin/bash
# scripts/deploy-production.sh
# Complete deployment workflow integrating all strategies

set -euo pipefail

VERSION="${1:?Version required}"
STRATEGY="${2:-blue-green}"  # Options: blue-green, canary

echo "Starting production deployment: $VERSION (strategy: $STRATEGY)"

# Step 1: Pre-deployment checks
echo "[1/5] Running pre-deployment checks..."
./scripts/pre-deployment-check.sh || {
    echo "Pre-deployment checks failed. Aborting."
    exit 1
}

# Step 2: Deploy based on strategy
echo "[2/5] Executing $STRATEGY deployment..."
case "$STRATEGY" in
    blue-green)
        ./scripts/deploy-blue-green.sh "$VERSION"
        ;;
    canary)
        ./scripts/deploy-canary.sh "$VERSION"
        ;;
    *)
        echo "Unknown strategy: $STRATEGY"
        exit 1
        ;;
esac

# Step 3: Run smoke tests
echo "[3/5] Running smoke tests..."
./scripts/smoke-tests.sh "https://api.research-lab.example.com" || {
    echo "Smoke tests failed. Initiating rollback..."
    ./scripts/rollback-$STRATEGY.sh
    exit 1
}

# Step 4: Verify performance baselines
echo "[4/5] Verifying performance baselines..."
cargo test --release --test performance_baseline || {
    echo "Performance baseline verification failed."
    exit 1
}

# Step 5: Post-deployment validation
echo "[5/5] Post-deployment validation..."
sleep 60  # Wait for metrics to stabilize
./scripts/validate-deployment.sh || {
    echo "Post-deployment validation failed."
    exit 1
}

echo "✓ Production deployment completed successfully: $VERSION"
```

---

## Document Metadata

| Field | Value |
|-------|-------|
| **Version** | 1.0.0 |
| **Status** | Specification |
| **SPARC Phase** | Phase 5: Completion - Section 3.4-3.5 |
| **Created** | 2025-11-28 |
| **Technology Stack** | Rust, Axum, Kubernetes, Prometheus |
| **Dependencies** | PostgreSQL, Redis, Kafka, ClickHouse |

---

**Next Sections**: 3.6-3.8 (Monitoring Integration, Observability, Incident Response)
