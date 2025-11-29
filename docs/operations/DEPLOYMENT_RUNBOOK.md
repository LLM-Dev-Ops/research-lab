# LLM Research Lab - Deployment Runbook

## Overview

This runbook provides comprehensive procedures for deploying, updating, and managing the LLM Research Lab platform in production environments.

## Table of Contents
- [Prerequisites](#prerequisites)
- [Environment Configuration](#environment-configuration)
- [Deployment Procedures](#deployment-procedures)
- [Health Verification](#health-verification)
- [Scaling Operations](#scaling-operations)
- [Database Operations](#database-operations)
- [Monitoring Setup](#monitoring-setup)
- [Maintenance Windows](#maintenance-windows)
- [Emergency Procedures](#emergency-procedures)

---

## Prerequisites

### Required Tools

| Tool | Version | Purpose |
|------|---------|---------|
| kubectl | >= 1.28 | Kubernetes management |
| helm | >= 3.12 | Chart deployment |
| aws-cli | >= 2.0 | AWS resource management |
| docker | >= 24.0 | Container operations |
| psql | >= 15 | Database administration |
| redis-cli | >= 7.0 | Cache management |

### Access Requirements

- [ ] Kubernetes cluster admin credentials
- [ ] AWS IAM role with required permissions
- [ ] Database admin credentials
- [ ] VPN access to production network
- [ ] PagerDuty/Opsgenie access for alerts
- [ ] Access to secrets management (Vault/AWS Secrets Manager)

### Pre-Deployment Checklist

```bash
# Verify kubectl access
kubectl auth can-i '*' '*' --all-namespaces

# Check cluster health
kubectl get nodes
kubectl top nodes

# Verify Helm repositories
helm repo update

# Check database connectivity
psql $DATABASE_URL -c "SELECT 1"

# Verify S3 access
aws s3 ls s3://$S3_BUCKET/

# Check secrets availability
aws secretsmanager get-secret-value --secret-id llm-research-lab/production
```

---

## Environment Configuration

### Environment Variables

```bash
# Application Configuration
export APP_NAME="llm-research-lab"
export ENVIRONMENT="production"
export NAMESPACE="llm-research"
export REPLICAS=3

# Database Configuration
export DATABASE_URL="postgresql://user:pass@db.cluster-xxx.region.rds.amazonaws.com:5432/llm_research"
export DATABASE_MAX_CONNECTIONS=100
export DATABASE_MIN_CONNECTIONS=10

# ClickHouse Configuration
export CLICKHOUSE_URL="https://clickhouse.cluster.internal:8443"
export CLICKHOUSE_USER="llm_research"
export CLICKHOUSE_DATABASE="llm_metrics"

# S3 Configuration
export S3_BUCKET="llm-research-datasets-prod"
export S3_REGION="us-east-1"

# Redis Configuration
export REDIS_URL="redis://redis.cluster.internal:6379"
export REDIS_CLUSTER_ENABLED="true"

# Observability
export OTEL_EXPORTER_ENDPOINT="https://otel-collector.internal:4317"
export PROMETHEUS_ENABLED="true"
export LOG_LEVEL="info"
export LOG_FORMAT="json"

# Security
export JWT_SECRET_ARN="arn:aws:secretsmanager:us-east-1:123456789:secret:jwt-secret"
export API_KEY_SECRET_ARN="arn:aws:secretsmanager:us-east-1:123456789:secret:api-key-secret"

# Rate Limiting
export RATE_LIMIT_REQUESTS_PER_SECOND=100
export RATE_LIMIT_BURST=200
```

### Kubernetes ConfigMap

```yaml
# config/production-config.yaml
apiVersion: v1
kind: ConfigMap
metadata:
  name: llm-research-config
  namespace: llm-research
data:
  RUST_LOG: "info,llm_research_api=debug"
  SERVER_HOST: "0.0.0.0"
  SERVER_PORT: "8080"
  METRICS_PORT: "9090"
  HEALTH_CHECK_INTERVAL: "30s"
  GRACEFUL_SHUTDOWN_TIMEOUT: "30s"
  CONNECTION_POOL_SIZE: "50"
  CACHE_TTL_SECONDS: "300"
  COMPRESSION_ENABLED: "true"
  COMPRESSION_MIN_SIZE: "1024"
```

### Kubernetes Secrets

```bash
# Create secrets from AWS Secrets Manager
kubectl create secret generic llm-research-secrets \
  --from-literal=DATABASE_URL=$(aws secretsmanager get-secret-value \
    --secret-id llm-research/database-url --query SecretString --output text) \
  --from-literal=JWT_SECRET=$(aws secretsmanager get-secret-value \
    --secret-id llm-research/jwt-secret --query SecretString --output text) \
  --namespace llm-research
```

---

## Deployment Procedures

### Standard Deployment (Blue-Green)

#### Step 1: Prepare New Version

```bash
# Set version
export NEW_VERSION="v1.2.3"
export OLD_VERSION=$(kubectl get deployment llm-research-api -n llm-research \
  -o jsonpath='{.spec.template.spec.containers[0].image}' | cut -d: -f2)

# Pull and verify new image
docker pull ghcr.io/llm-research-lab/api:${NEW_VERSION}
docker inspect ghcr.io/llm-research-lab/api:${NEW_VERSION}

# Verify image vulnerabilities
trivy image ghcr.io/llm-research-lab/api:${NEW_VERSION}
```

#### Step 2: Deploy Green Environment

```bash
# Create green deployment
cat <<EOF | kubectl apply -f -
apiVersion: apps/v1
kind: Deployment
metadata:
  name: llm-research-api-green
  namespace: llm-research
  labels:
    app: llm-research-api
    version: green
spec:
  replicas: ${REPLICAS}
  selector:
    matchLabels:
      app: llm-research-api
      version: green
  template:
    metadata:
      labels:
        app: llm-research-api
        version: green
    spec:
      containers:
      - name: api
        image: ghcr.io/llm-research-lab/api:${NEW_VERSION}
        ports:
        - containerPort: 8080
        - containerPort: 9090
        envFrom:
        - configMapRef:
            name: llm-research-config
        - secretRef:
            name: llm-research-secrets
        resources:
          requests:
            memory: "512Mi"
            cpu: "500m"
          limits:
            memory: "2Gi"
            cpu: "2000m"
        livenessProbe:
          httpGet:
            path: /health
            port: 8080
          initialDelaySeconds: 10
          periodSeconds: 10
          failureThreshold: 3
        readinessProbe:
          httpGet:
            path: /health/ready
            port: 8080
          initialDelaySeconds: 5
          periodSeconds: 5
          failureThreshold: 3
        lifecycle:
          preStop:
            exec:
              command: ["/bin/sh", "-c", "sleep 15"]
EOF
```

#### Step 3: Verify Green Deployment

```bash
# Wait for rollout
kubectl rollout status deployment/llm-research-api-green -n llm-research --timeout=300s

# Check pod status
kubectl get pods -n llm-research -l version=green

# Run smoke tests against green pods
GREEN_POD=$(kubectl get pod -n llm-research -l version=green -o jsonpath='{.items[0].metadata.name}')
kubectl port-forward -n llm-research $GREEN_POD 8081:8080 &
PF_PID=$!
sleep 5

# Health check
curl -f http://localhost:8081/health
curl -f http://localhost:8081/health/ready

# API smoke tests
curl -f http://localhost:8081/models/providers

kill $PF_PID
```

#### Step 4: Switch Traffic

```bash
# Update service selector to green
kubectl patch service llm-research-api -n llm-research \
  -p '{"spec":{"selector":{"version":"green"}}}'

# Verify traffic switch
kubectl get endpoints llm-research-api -n llm-research

# Monitor error rates for 5 minutes
echo "Monitoring error rates..."
for i in {1..30}; do
  ERROR_RATE=$(curl -s http://prometheus:9090/api/v1/query?query=rate(http_requests_total{status=~"5.."}[1m]) | jq '.data.result[0].value[1]')
  echo "Error rate: $ERROR_RATE"
  sleep 10
done
```

#### Step 5: Cleanup Blue Environment

```bash
# After successful verification (wait at least 30 minutes)
kubectl delete deployment llm-research-api-blue -n llm-research

# Rename green to blue for next deployment
kubectl patch deployment llm-research-api-green -n llm-research \
  --type=json -p='[{"op": "replace", "path": "/metadata/name", "value": "llm-research-api-blue"}]'
```

### Rolling Update Deployment

```bash
# For minor updates, use rolling deployment
kubectl set image deployment/llm-research-api \
  api=ghcr.io/llm-research-lab/api:${NEW_VERSION} \
  -n llm-research

# Monitor rollout
kubectl rollout status deployment/llm-research-api -n llm-research

# If issues occur, rollback immediately
kubectl rollout undo deployment/llm-research-api -n llm-research
```

### Canary Deployment

```bash
# Deploy canary with 10% traffic
cat <<EOF | kubectl apply -f -
apiVersion: networking.istio.io/v1beta1
kind: VirtualService
metadata:
  name: llm-research-api
  namespace: llm-research
spec:
  hosts:
  - llm-research-api
  http:
  - match:
    - headers:
        x-canary:
          exact: "true"
    route:
    - destination:
        host: llm-research-api-canary
        port:
          number: 8080
  - route:
    - destination:
        host: llm-research-api
        port:
          number: 8080
      weight: 90
    - destination:
        host: llm-research-api-canary
        port:
          number: 8080
      weight: 10
EOF

# Monitor canary metrics
watch -n 5 'kubectl top pods -n llm-research -l version=canary'

# Gradually increase canary traffic: 10% -> 25% -> 50% -> 100%
```

---

## Health Verification

### Post-Deployment Verification

```bash
#!/bin/bash
# health-check.sh

set -e

API_URL="${API_URL:-https://api.llm-research-lab.io}"
TIMEOUT=10

echo "Starting health verification..."

# 1. Liveness check
echo "Checking liveness..."
curl -sf --max-time $TIMEOUT "${API_URL}/health" || {
  echo "FAILED: Liveness check"
  exit 1
}
echo "OK: Liveness"

# 2. Readiness check
echo "Checking readiness..."
READY_RESPONSE=$(curl -sf --max-time $TIMEOUT "${API_URL}/health/ready")
if ! echo "$READY_RESPONSE" | jq -e '.status == "healthy"' > /dev/null; then
  echo "FAILED: Readiness check"
  echo "$READY_RESPONSE"
  exit 1
fi
echo "OK: Readiness"

# 3. API endpoint verification
echo "Checking API endpoints..."

# Models endpoint
curl -sf --max-time $TIMEOUT "${API_URL}/models/providers" \
  -H "Authorization: Bearer ${TEST_TOKEN}" || {
  echo "FAILED: Models providers endpoint"
  exit 1
}
echo "OK: Models endpoint"

# 4. Database connectivity
echo "Checking database via API..."
EXPERIMENTS=$(curl -sf --max-time $TIMEOUT "${API_URL}/experiments?limit=1" \
  -H "Authorization: Bearer ${TEST_TOKEN}")
if [ $? -ne 0 ]; then
  echo "FAILED: Database connectivity"
  exit 1
fi
echo "OK: Database"

# 5. Metrics endpoint
echo "Checking metrics..."
METRICS=$(curl -sf --max-time $TIMEOUT "${API_URL}/metrics")
if ! echo "$METRICS" | grep -q "http_requests_total"; then
  echo "FAILED: Metrics endpoint"
  exit 1
fi
echo "OK: Metrics"

# 6. Response time check
echo "Checking response times..."
for i in {1..5}; do
  TIME=$(curl -o /dev/null -sf -w '%{time_total}' --max-time $TIMEOUT "${API_URL}/health")
  if (( $(echo "$TIME > 1.0" | bc -l) )); then
    echo "WARNING: Slow response time: ${TIME}s"
  fi
done
echo "OK: Response times acceptable"

echo ""
echo "========================================="
echo "All health checks passed!"
echo "========================================="
```

### Continuous Health Monitoring

```bash
# Set up continuous monitoring during deployment window
watch -n 5 '
  echo "=== Deployment Health Monitor ==="
  echo ""
  echo "Pod Status:"
  kubectl get pods -n llm-research -l app=llm-research-api
  echo ""
  echo "Resource Usage:"
  kubectl top pods -n llm-research -l app=llm-research-api
  echo ""
  echo "Recent Events:"
  kubectl get events -n llm-research --sort-by=.lastTimestamp | tail -10
'
```

---

## Scaling Operations

### Horizontal Pod Autoscaler

```yaml
# hpa.yaml
apiVersion: autoscaling/v2
kind: HorizontalPodAutoscaler
metadata:
  name: llm-research-api-hpa
  namespace: llm-research
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
  - type: Pods
    pods:
      metric:
        name: http_requests_per_second
      target:
        type: AverageValue
        averageValue: "100"
  behavior:
    scaleUp:
      stabilizationWindowSeconds: 60
      policies:
      - type: Percent
        value: 100
        periodSeconds: 60
      - type: Pods
        value: 4
        periodSeconds: 60
      selectPolicy: Max
    scaleDown:
      stabilizationWindowSeconds: 300
      policies:
      - type: Percent
        value: 10
        periodSeconds: 60
      selectPolicy: Min
```

### Manual Scaling

```bash
# Scale up for expected high load
kubectl scale deployment llm-research-api -n llm-research --replicas=10

# Verify scaling
kubectl get deployment llm-research-api -n llm-research

# Scale down after load subsides
kubectl scale deployment llm-research-api -n llm-research --replicas=3
```

### Database Connection Pool Scaling

```bash
# Increase connection pool size dynamically via ConfigMap
kubectl patch configmap llm-research-config -n llm-research \
  --type merge -p '{"data":{"CONNECTION_POOL_SIZE":"100"}}'

# Restart pods to apply new config
kubectl rollout restart deployment/llm-research-api -n llm-research
```

---

## Database Operations

### Pre-Deployment Migration

```bash
#!/bin/bash
# run-migrations.sh

set -e

echo "Running database migrations..."

# Create backup before migration
BACKUP_NAME="pre-migration-$(date +%Y%m%d-%H%M%S)"
pg_dump $DATABASE_URL > "/backups/${BACKUP_NAME}.sql"
aws s3 cp "/backups/${BACKUP_NAME}.sql" "s3://${BACKUP_BUCKET}/migrations/"

echo "Backup created: ${BACKUP_NAME}"

# Run migrations
sqlx migrate run --database-url $DATABASE_URL

# Verify migration
sqlx migrate info --database-url $DATABASE_URL

echo "Migrations completed successfully"
```

### Database Health Check

```bash
# Check database health
psql $DATABASE_URL <<EOF
-- Check connection count
SELECT count(*) as active_connections
FROM pg_stat_activity
WHERE state = 'active';

-- Check long-running queries
SELECT pid, now() - pg_stat_activity.query_start AS duration, query
FROM pg_stat_activity
WHERE state = 'active'
  AND (now() - pg_stat_activity.query_start) > interval '5 minutes';

-- Check table sizes
SELECT relname, pg_size_pretty(pg_total_relation_size(relid))
FROM pg_catalog.pg_statio_user_tables
ORDER BY pg_total_relation_size(relid) DESC
LIMIT 10;

-- Check index usage
SELECT schemaname, tablename, indexname, idx_scan, idx_tup_read, idx_tup_fetch
FROM pg_stat_user_indexes
ORDER BY idx_scan DESC
LIMIT 10;
EOF
```

### Connection Pool Monitoring

```bash
# Monitor connection pool via metrics
curl -s http://localhost:9090/metrics | grep -E "^db_pool"
```

---

## Monitoring Setup

### Prometheus Alerting Rules

```yaml
# alerts/llm-research-alerts.yaml
groups:
- name: llm-research-api
  rules:
  - alert: HighErrorRate
    expr: rate(http_requests_total{status=~"5.."}[5m]) / rate(http_requests_total[5m]) > 0.01
    for: 5m
    labels:
      severity: critical
    annotations:
      summary: "High error rate detected"
      description: "Error rate is {{ $value | printf \"%.2f\" }}%"

  - alert: HighLatency
    expr: histogram_quantile(0.95, rate(http_request_duration_seconds_bucket[5m])) > 2
    for: 5m
    labels:
      severity: warning
    annotations:
      summary: "High API latency"
      description: "95th percentile latency is {{ $value }}s"

  - alert: PodNotReady
    expr: kube_pod_status_ready{namespace="llm-research",condition="true"} == 0
    for: 2m
    labels:
      severity: critical
    annotations:
      summary: "Pod not ready"
      description: "Pod {{ $labels.pod }} is not ready"

  - alert: DatabaseConnectionPoolExhausted
    expr: db_pool_connections_active / db_pool_connections_max > 0.9
    for: 5m
    labels:
      severity: warning
    annotations:
      summary: "Database connection pool near exhaustion"
      description: "Pool usage is at {{ $value | printf \"%.0f\" }}%"
```

### Grafana Dashboard Setup

```bash
# Import pre-built dashboard
kubectl apply -f dashboards/llm-research-overview.json

# Dashboard includes:
# - Request rate and latency
# - Error rates by endpoint
# - Pod resource utilization
# - Database connection pool metrics
# - Cache hit rates
# - Circuit breaker status
```

---

## Maintenance Windows

### Scheduled Maintenance Procedure

```bash
#!/bin/bash
# maintenance-window.sh

set -e

MAINTENANCE_DURATION_MINUTES=${1:-30}
MAINTENANCE_MESSAGE="Scheduled maintenance in progress"

echo "Starting maintenance window..."

# 1. Enable maintenance mode
kubectl patch configmap llm-research-config -n llm-research \
  --type merge -p '{"data":{"MAINTENANCE_MODE":"true"}}'

# 2. Drain traffic gradually
for replica in $(seq $REPLICAS -1 1); do
  kubectl scale deployment llm-research-api -n llm-research --replicas=$replica
  sleep 30
done

# 3. Perform maintenance tasks
echo "Maintenance window active for ${MAINTENANCE_DURATION_MINUTES} minutes"
echo "Perform required maintenance tasks now..."

# Wait for maintenance duration
sleep $((MAINTENANCE_DURATION_MINUTES * 60))

# 4. Restore service
kubectl scale deployment llm-research-api -n llm-research --replicas=$REPLICAS

# 5. Disable maintenance mode
kubectl patch configmap llm-research-config -n llm-research \
  --type merge -p '{"data":{"MAINTENANCE_MODE":"false"}}'

# 6. Verify health
./health-check.sh

echo "Maintenance window completed"
```

---

## Emergency Procedures

### Emergency Rollback

```bash
#!/bin/bash
# emergency-rollback.sh

set -e

echo "EMERGENCY ROLLBACK INITIATED"
echo "Timestamp: $(date -u +%Y-%m-%dT%H:%M:%SZ)"

# 1. Immediate rollback
kubectl rollout undo deployment/llm-research-api -n llm-research

# 2. Wait for rollback
kubectl rollout status deployment/llm-research-api -n llm-research --timeout=120s

# 3. Verify health
./health-check.sh

# 4. Alert team
curl -X POST "${SLACK_WEBHOOK}" \
  -H "Content-Type: application/json" \
  -d '{
    "text": "EMERGENCY ROLLBACK COMPLETED for llm-research-api",
    "attachments": [{
      "color": "warning",
      "fields": [
        {"title": "Environment", "value": "'"${ENVIRONMENT}"'", "short": true},
        {"title": "Timestamp", "value": "'"$(date -u +%Y-%m-%dT%H:%M:%SZ)"'", "short": true}
      ]
    }]
  }'

echo "Rollback completed"
```

### Circuit Breaker Activation

```bash
# Force circuit breaker open for external dependency
kubectl exec -it deployment/llm-research-api -n llm-research -- \
  curl -X POST http://localhost:8080/admin/circuit-breaker/open \
    -H "X-Admin-Key: ${ADMIN_KEY}" \
    -d '{"service": "openai-api"}'

# Check circuit breaker status
kubectl exec -it deployment/llm-research-api -n llm-research -- \
  curl http://localhost:8080/admin/circuit-breaker/status
```

### Emergency Contact List

| Role | Name | Contact | Escalation Order |
|------|------|---------|------------------|
| On-Call Engineer | Rotation | PagerDuty | 1 |
| Platform Lead | TBD | TBD | 2 |
| Engineering Manager | TBD | TBD | 3 |
| VP Engineering | TBD | TBD | 4 |

---

## Appendix

### Useful Commands

```bash
# Get deployment status
kubectl describe deployment llm-research-api -n llm-research

# View recent logs
kubectl logs -n llm-research -l app=llm-research-api --tail=100 -f

# Get pod events
kubectl get events -n llm-research --sort-by=.lastTimestamp

# Check resource quotas
kubectl describe resourcequota -n llm-research

# Port forward for debugging
kubectl port-forward -n llm-research svc/llm-research-api 8080:8080

# Execute into pod
kubectl exec -it deployment/llm-research-api -n llm-research -- /bin/sh
```

### Version History

| Version | Date | Author | Changes |
|---------|------|--------|---------|
| 1.0.0 | 2025-01-15 | Platform Team | Initial runbook |
