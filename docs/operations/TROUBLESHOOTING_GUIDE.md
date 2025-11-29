# LLM Research Lab - Troubleshooting Guide

## Overview

This guide provides diagnostic procedures and solutions for common issues encountered in the LLM Research Lab platform.

## Table of Contents
- [Quick Diagnostics](#quick-diagnostics)
- [Application Issues](#application-issues)
- [Database Issues](#database-issues)
- [Storage Issues](#storage-issues)
- [Authentication Issues](#authentication-issues)
- [Performance Issues](#performance-issues)
- [Infrastructure Issues](#infrastructure-issues)
- [Logging and Monitoring](#logging-and-monitoring)

---

## Quick Diagnostics

### System Health Check

```bash
#!/bin/bash
# quick-health-check.sh

echo "=== LLM Research Lab Quick Health Check ==="
echo "Timestamp: $(date -u +%Y-%m-%dT%H:%M:%SZ)"
echo ""

# Check pods
echo "--- Pod Status ---"
kubectl get pods -n llm-research -l app=llm-research-api

# Check services
echo ""
echo "--- Service Endpoints ---"
kubectl get endpoints llm-research-api -n llm-research

# Check recent events
echo ""
echo "--- Recent Events (last 10) ---"
kubectl get events -n llm-research --sort-by=.lastTimestamp | tail -10

# Check resource usage
echo ""
echo "--- Resource Usage ---"
kubectl top pods -n llm-research -l app=llm-research-api

# Check logs for errors
echo ""
echo "--- Recent Errors ---"
kubectl logs -n llm-research -l app=llm-research-api --since=5m 2>/dev/null | grep -i "error\|panic\|fatal" | tail -10 || echo "No errors found"
```

### Component Status Matrix

| Component | Check Command | Expected Result |
|-----------|--------------|-----------------|
| API Pods | `kubectl get pods -n llm-research` | All Running |
| Database | `psql $DATABASE_URL -c "SELECT 1"` | Returns 1 |
| ClickHouse | `curl http://clickhouse:8123/ping` | Ok |
| S3 | `aws s3 ls s3://$BUCKET/ --max-items 1` | No error |
| Redis | `redis-cli -u $REDIS_URL ping` | PONG |

---

## Application Issues

### Issue: Pods Not Starting

**Symptoms:**
- Pods in `Pending`, `CrashLoopBackOff`, or `ImagePullBackOff` state
- Events showing scheduling or container failures

**Diagnosis:**
```bash
# Get pod status
kubectl get pods -n llm-research -l app=llm-research-api

# Describe failing pod
kubectl describe pod <pod-name> -n llm-research

# Check events
kubectl get events -n llm-research --field-selector involvedObject.name=<pod-name>
```

**Common Causes and Solutions:**

| Cause | Symptom | Solution |
|-------|---------|----------|
| Image pull failure | `ImagePullBackOff` | Check image name, registry credentials |
| Resource constraints | `Pending` with scheduling events | Increase node resources or reduce requests |
| Failed probes | `CrashLoopBackOff` | Check probe configuration, application startup |
| Missing secrets | Container creation error | Verify secrets exist and are correctly named |
| Volume mount issues | `ContainerCreating` stuck | Check PVC/PV, volume permissions |

```bash
# Fix: Image pull issues
kubectl create secret docker-registry regcred \
  --docker-server=ghcr.io \
  --docker-username=$GITHUB_USER \
  --docker-password=$GITHUB_TOKEN \
  -n llm-research

# Fix: Resource constraints
kubectl patch deployment llm-research-api -n llm-research --type json \
  -p='[{"op": "replace", "path": "/spec/template/spec/containers/0/resources/requests/memory", "value": "256Mi"}]'

# Fix: Missing secrets
kubectl create secret generic llm-research-secrets \
  --from-literal=DATABASE_URL=$DATABASE_URL \
  --from-literal=JWT_SECRET=$JWT_SECRET \
  -n llm-research
```

### Issue: OOMKilled Pods

**Symptoms:**
- Pods restarting with `OOMKilled` reason
- Application becoming unresponsive before restart

**Diagnosis:**
```bash
# Check for OOM events
kubectl get pods -n llm-research -l app=llm-research-api -o wide
kubectl describe pod <pod-name> -n llm-research | grep -A5 "Last State"

# Check memory usage
kubectl top pods -n llm-research -l app=llm-research-api

# Check container memory limits
kubectl get deployment llm-research-api -n llm-research -o jsonpath='{.spec.template.spec.containers[0].resources}'
```

**Solutions:**
```bash
# Increase memory limit
kubectl patch deployment llm-research-api -n llm-research --type json \
  -p='[
    {"op": "replace", "path": "/spec/template/spec/containers/0/resources/limits/memory", "value": "4Gi"},
    {"op": "replace", "path": "/spec/template/spec/containers/0/resources/requests/memory", "value": "2Gi"}
  ]'

# If memory leak, enable profiling
kubectl set env deployment/llm-research-api -n llm-research \
  RUST_BACKTRACE=1 \
  ENABLE_MEMORY_PROFILING=true
```

### Issue: High CPU Usage

**Symptoms:**
- Slow response times
- CPU throttling
- Pods using 100% CPU

**Diagnosis:**
```bash
# Check CPU usage
kubectl top pods -n llm-research -l app=llm-research-api

# Check for CPU throttling
kubectl describe pod <pod-name> -n llm-research | grep -A5 "Limits"

# Profile application
kubectl exec -it <pod-name> -n llm-research -- \
  curl http://localhost:9090/debug/pprof/profile?seconds=30 > cpu-profile.pb.gz
```

**Solutions:**
```bash
# Scale horizontally
kubectl scale deployment llm-research-api -n llm-research --replicas=5

# Increase CPU limits
kubectl patch deployment llm-research-api -n llm-research --type json \
  -p='[{"op": "replace", "path": "/spec/template/spec/containers/0/resources/limits/cpu", "value": "4000m"}]'
```

### Issue: Application Panics

**Symptoms:**
- Pods crashing immediately
- Panic messages in logs
- `CrashLoopBackOff` with exit code 101

**Diagnosis:**
```bash
# Get panic logs
kubectl logs <pod-name> -n llm-research --previous

# Get stack trace
kubectl logs -n llm-research -l app=llm-research-api --previous | grep -A 50 "panic"
```

**Solutions:**
```bash
# Enable full backtraces
kubectl set env deployment/llm-research-api -n llm-research RUST_BACKTRACE=full

# Rollback if recent deployment caused issue
kubectl rollout undo deployment/llm-research-api -n llm-research

# Check for configuration issues
kubectl get configmap llm-research-config -n llm-research -o yaml
kubectl get secret llm-research-secrets -n llm-research -o yaml
```

---

## Database Issues

### Issue: Connection Pool Exhaustion

**Symptoms:**
- "too many connections" errors
- Slow or failed API requests
- Connection timeout errors

**Diagnosis:**
```bash
# Check PostgreSQL connections
psql $DATABASE_URL <<EOF
SELECT count(*), state FROM pg_stat_activity GROUP BY state;
SELECT client_addr, count(*) FROM pg_stat_activity GROUP BY client_addr ORDER BY count DESC;
SELECT wait_event_type, wait_event, count(*) FROM pg_stat_activity WHERE state = 'active' GROUP BY 1, 2;
EOF

# Check application pool metrics
curl -s http://localhost:9090/metrics | grep -E "^db_pool"
```

**Solutions:**
```bash
# Terminate idle connections
psql $DATABASE_URL -c "SELECT pg_terminate_backend(pid) FROM pg_stat_activity WHERE state = 'idle' AND query_start < now() - interval '10 minutes';"

# Increase pool size
kubectl patch configmap llm-research-config -n llm-research \
  --type merge -p '{"data":{"DATABASE_MAX_CONNECTIONS":"100"}}'
kubectl rollout restart deployment/llm-research-api -n llm-research

# Check for connection leaks in application
kubectl logs -n llm-research -l app=llm-research-api | grep -i "connection\|pool"
```

### Issue: Slow Queries

**Symptoms:**
- High API latency
- Database CPU spiking
- Lock wait timeouts

**Diagnosis:**
```bash
# Find slow queries
psql $DATABASE_URL <<EOF
-- Currently running queries > 1 second
SELECT pid, now() - query_start AS duration, query, state
FROM pg_stat_activity
WHERE state = 'active' AND (now() - query_start) > interval '1 second'
ORDER BY duration DESC;

-- Most time-consuming queries (requires pg_stat_statements)
SELECT query, calls, mean_time, total_time
FROM pg_stat_statements
ORDER BY total_time DESC
LIMIT 10;

-- Check for lock waits
SELECT blocked_locks.pid AS blocked_pid,
       blocking_locks.pid AS blocking_pid,
       blocked_activity.query AS blocked_query
FROM pg_catalog.pg_locks blocked_locks
JOIN pg_catalog.pg_locks blocking_locks ON blocking_locks.locktype = blocked_locks.locktype
JOIN pg_catalog.pg_stat_activity blocked_activity ON blocked_activity.pid = blocked_locks.pid
WHERE NOT blocked_locks.granted;
EOF
```

**Solutions:**
```bash
# Kill long-running query
psql $DATABASE_URL -c "SELECT pg_terminate_backend(<pid>);"

# Add missing index (example)
psql $DATABASE_URL -c "CREATE INDEX CONCURRENTLY idx_experiments_status ON experiments(status);"

# Analyze tables
psql $DATABASE_URL -c "ANALYZE experiments; ANALYZE models; ANALYZE datasets;"

# Increase work_mem for complex queries
psql $DATABASE_URL -c "ALTER DATABASE llm_research SET work_mem = '256MB';"
```

### Issue: Database Replication Lag

**Symptoms:**
- Stale reads from replicas
- Read replica falling behind

**Diagnosis:**
```bash
# Check replication status (on primary)
psql $DATABASE_URL <<EOF
SELECT client_addr, state, sent_lsn, write_lsn, flush_lsn, replay_lsn,
       pg_wal_lsn_diff(sent_lsn, replay_lsn) AS replication_lag_bytes
FROM pg_stat_replication;
EOF

# Check RDS replication lag
aws cloudwatch get-metric-statistics \
  --namespace AWS/RDS \
  --metric-name ReplicaLag \
  --dimensions Name=DBInstanceIdentifier,Value=llm-research-db-replica \
  --start-time $(date -d '1 hour ago' --iso-8601=seconds) \
  --end-time $(date --iso-8601=seconds) \
  --period 60 \
  --statistics Average
```

**Solutions:**
```bash
# Route writes to primary only
kubectl patch configmap llm-research-config -n llm-research \
  --type merge -p '{"data":{"DATABASE_WRITE_URL":"postgresql://primary...", "DATABASE_READ_URL":"postgresql://replica..."}}'

# Increase replica instance size if CPU-bound
aws rds modify-db-instance \
  --db-instance-identifier llm-research-db-replica \
  --db-instance-class db.r6g.2xlarge \
  --apply-immediately
```

---

## Storage Issues

### Issue: S3 Upload Failures

**Symptoms:**
- Dataset uploads timing out
- "Access Denied" errors
- Presigned URL failures

**Diagnosis:**
```bash
# Test S3 connectivity
aws s3 ls s3://$S3_BUCKET/ --max-items 1

# Check IAM permissions
aws sts get-caller-identity
aws iam simulate-principal-policy \
  --policy-source-arn $ROLE_ARN \
  --action-names s3:PutObject s3:GetObject \
  --resource-arns "arn:aws:s3:::$S3_BUCKET/*"

# Check VPC endpoint (if using)
aws ec2 describe-vpc-endpoints --filters "Name=service-name,Values=com.amazonaws.$REGION.s3"

# Check application logs
kubectl logs -n llm-research -l app=llm-research-api | grep -i "s3\|upload\|aws"
```

**Solutions:**
```bash
# Fix IAM permissions
aws iam attach-role-policy \
  --role-name llm-research-api-role \
  --policy-arn arn:aws:iam::aws:policy/AmazonS3FullAccess

# Check CORS configuration
aws s3api get-bucket-cors --bucket $S3_BUCKET

# Set correct CORS
aws s3api put-bucket-cors --bucket $S3_BUCKET --cors-configuration '{
  "CORSRules": [{
    "AllowedOrigins": ["https://app.llm-research-lab.io"],
    "AllowedMethods": ["GET", "PUT", "POST"],
    "AllowedHeaders": ["*"],
    "ExposeHeaders": ["ETag"],
    "MaxAgeSeconds": 3600
  }]
}'

# Increase presigned URL expiration
kubectl patch configmap llm-research-config -n llm-research \
  --type merge -p '{"data":{"S3_PRESIGNED_URL_EXPIRY":"3600"}}'
```

### Issue: S3 Download Failures

**Symptoms:**
- Dataset downloads timing out
- "NoSuchKey" errors
- Slow downloads

**Diagnosis:**
```bash
# Check if object exists
aws s3 ls "s3://$S3_BUCKET/datasets/$DATASET_ID/"

# Check object metadata
aws s3api head-object --bucket $S3_BUCKET --key "datasets/$DATASET_ID/data.jsonl"

# Check transfer acceleration status
aws s3api get-bucket-accelerate-configuration --bucket $S3_BUCKET
```

**Solutions:**
```bash
# Enable transfer acceleration
aws s3api put-bucket-accelerate-configuration --bucket $S3_BUCKET \
  --accelerate-configuration Status=Enabled

# Use multipart download for large files
aws s3 cp "s3://$S3_BUCKET/datasets/$DATASET_ID/data.jsonl" ./data.jsonl \
  --expected-size 1073741824
```

---

## Authentication Issues

### Issue: JWT Token Validation Failures

**Symptoms:**
- 401 Unauthorized errors
- "Invalid token" messages
- Token expiration issues

**Diagnosis:**
```bash
# Decode JWT (without validation)
echo "$TOKEN" | cut -d. -f2 | base64 -d 2>/dev/null | jq .

# Check token expiration
echo "$TOKEN" | cut -d. -f2 | base64 -d 2>/dev/null | jq -r '.exp | todate'

# Verify secret is available
kubectl get secret llm-research-secrets -n llm-research -o jsonpath='{.data.JWT_SECRET}' | base64 -d | head -c 10; echo "..."

# Check application logs
kubectl logs -n llm-research -l app=llm-research-api | grep -i "jwt\|auth\|token"
```

**Solutions:**
```bash
# Rotate JWT secret
NEW_SECRET=$(openssl rand -hex 32)
kubectl patch secret llm-research-secrets -n llm-research \
  --type merge -p "{\"data\":{\"JWT_SECRET\":\"$(echo -n $NEW_SECRET | base64)\"}}"
kubectl rollout restart deployment/llm-research-api -n llm-research

# Check clock sync on pods
kubectl exec -it deployment/llm-research-api -n llm-research -- date

# Increase token expiry
kubectl patch configmap llm-research-config -n llm-research \
  --type merge -p '{"data":{"JWT_EXPIRY_SECONDS":"7200"}}'
```

### Issue: API Key Authentication Failures

**Symptoms:**
- API key rejected
- Rate limit errors
- Scope permission denied

**Diagnosis:**
```bash
# Check API key format
echo "$API_KEY" | cut -d_ -f1  # Should be "llm-research"

# Check API key in database
psql $DATABASE_URL -c "SELECT id, name, scopes, rate_limit_tier, expires_at, is_active FROM api_keys WHERE key_prefix = '<prefix>';"

# Check rate limit status
curl -v -H "X-API-Key: $API_KEY" https://api.llm-research-lab.io/experiments
# Look for X-RateLimit-* headers
```

**Solutions:**
```bash
# Regenerate API key
curl -X POST https://api.llm-research-lab.io/admin/api-keys \
  -H "Authorization: Bearer $ADMIN_TOKEN" \
  -d '{"name": "new-key", "scopes": ["experiments:read", "experiments:write"]}'

# Update rate limit tier
psql $DATABASE_URL -c "UPDATE api_keys SET rate_limit_tier = 'professional' WHERE id = '<key-id>';"

# Extend key expiration
psql $DATABASE_URL -c "UPDATE api_keys SET expires_at = now() + interval '1 year' WHERE id = '<key-id>';"
```

---

## Performance Issues

### Issue: High API Latency

**Symptoms:**
- P95 latency > 2 seconds
- Timeout errors
- User complaints

**Diagnosis:**
```bash
# Check latency by endpoint
curl -s "http://prometheus:9090/api/v1/query?query=histogram_quantile(0.95,rate(http_request_duration_seconds_bucket[5m]))" | jq '.data.result[] | {path: .metric.path, p95: .value[1]}'

# Check for slow endpoints
curl -s http://localhost:9090/metrics | grep http_request_duration | sort -t'=' -k2 -rn | head -20

# Check database query times
kubectl logs -n llm-research -l app=llm-research-api | grep "slow query\|query_time_ms"

# Check external dependency latency
kubectl logs -n llm-research -l app=llm-research-api | grep -E "external_call|http_client"
```

**Solutions:**
```bash
# Enable caching
kubectl patch configmap llm-research-config -n llm-research \
  --type merge -p '{"data":{"CACHE_ENABLED":"true", "CACHE_TTL_SECONDS":"300"}}'

# Scale horizontally
kubectl scale deployment llm-research-api -n llm-research --replicas=6

# Enable response compression
kubectl patch configmap llm-research-config -n llm-research \
  --type merge -p '{"data":{"COMPRESSION_ENABLED":"true", "COMPRESSION_MIN_SIZE":"1024"}}'

# Add database indexes
psql $DATABASE_URL -c "CREATE INDEX CONCURRENTLY idx_experiments_owner_status ON experiments(owner_id, status);"
```

### Issue: Cache Miss Rate High

**Symptoms:**
- Low cache hit ratio
- Increased database load
- Higher latency

**Diagnosis:**
```bash
# Check cache metrics
curl -s http://localhost:9090/metrics | grep -E "^cache_(hits|misses|size)"

# Check Redis connectivity
redis-cli -u $REDIS_URL INFO

# Check Redis memory
redis-cli -u $REDIS_URL INFO memory
```

**Solutions:**
```bash
# Increase cache TTL
kubectl patch configmap llm-research-config -n llm-research \
  --type merge -p '{"data":{"CACHE_TTL_SECONDS":"600"}}'

# Increase cache size
kubectl patch configmap llm-research-config -n llm-research \
  --type merge -p '{"data":{"CACHE_MAX_SIZE":"10000"}}'

# Warm cache on startup
kubectl exec -it deployment/llm-research-api -n llm-research -- \
  curl http://localhost:8080/admin/cache/warm -X POST -H "X-Admin-Key: $ADMIN_KEY"
```

---

## Infrastructure Issues

### Issue: Node Not Ready

**Symptoms:**
- Node in `NotReady` state
- Pods stuck in `Pending`
- Scheduling failures

**Diagnosis:**
```bash
# Check node status
kubectl get nodes
kubectl describe node <node-name>

# Check kubelet logs
ssh <node> "journalctl -u kubelet -n 100"

# Check node conditions
kubectl get node <node-name> -o jsonpath='{.status.conditions[*]}' | jq .
```

**Solutions:**
```bash
# Drain unhealthy node
kubectl drain <node-name> --ignore-daemonsets --delete-emptydir-data

# Cordon node for maintenance
kubectl cordon <node-name>

# Restart kubelet
ssh <node> "sudo systemctl restart kubelet"

# Replace node (EKS)
aws autoscaling terminate-instance-in-auto-scaling-group \
  --instance-id <instance-id> \
  --should-decrement-desired-capacity false
```

### Issue: Network Connectivity Problems

**Symptoms:**
- DNS resolution failures
- Service-to-service communication issues
- External API calls failing

**Diagnosis:**
```bash
# Test DNS resolution
kubectl run -it --rm debug --image=busybox --restart=Never -- nslookup llm-research-api.llm-research.svc.cluster.local

# Test service connectivity
kubectl run -it --rm debug --image=curlimages/curl --restart=Never -- curl http://llm-research-api.llm-research.svc.cluster.local:8080/health

# Check network policies
kubectl get networkpolicies -n llm-research

# Check service endpoints
kubectl get endpoints -n llm-research
```

**Solutions:**
```bash
# Restart CoreDNS
kubectl rollout restart deployment/coredns -n kube-system

# Check and fix network policy
kubectl delete networkpolicy <policy-name> -n llm-research

# Verify security groups (AWS)
aws ec2 describe-security-groups --group-ids <sg-id>
```

---

## Logging and Monitoring

### Accessing Logs

```bash
# Real-time logs
kubectl logs -f -n llm-research -l app=llm-research-api

# Logs with timestamps
kubectl logs -n llm-research -l app=llm-research-api --timestamps

# Previous container logs (after restart)
kubectl logs <pod-name> -n llm-research --previous

# Logs from specific time range
kubectl logs -n llm-research -l app=llm-research-api --since=30m

# Search logs for pattern
kubectl logs -n llm-research -l app=llm-research-api | grep -E "error|panic|fatal"

# Export logs to file
kubectl logs -n llm-research -l app=llm-research-api > /tmp/api-logs.txt
```

### Prometheus Queries

```promql
# Error rate
rate(http_requests_total{status=~"5.."}[5m]) / rate(http_requests_total[5m])

# P95 latency
histogram_quantile(0.95, rate(http_request_duration_seconds_bucket[5m]))

# Request rate by endpoint
sum(rate(http_requests_total[5m])) by (path)

# Database connection pool usage
db_pool_connections_active / db_pool_connections_max

# Cache hit ratio
rate(cache_hits_total[5m]) / (rate(cache_hits_total[5m]) + rate(cache_misses_total[5m]))

# Pod memory usage
container_memory_usage_bytes{container="api", namespace="llm-research"}
```

### Debugging Commands

```bash
# Port forward for local debugging
kubectl port-forward -n llm-research svc/llm-research-api 8080:8080

# Execute into running pod
kubectl exec -it deployment/llm-research-api -n llm-research -- /bin/sh

# Get pod resource usage
kubectl top pods -n llm-research -l app=llm-research-api

# Get detailed pod info
kubectl describe pod <pod-name> -n llm-research

# Watch pod status
watch -n 5 'kubectl get pods -n llm-research -l app=llm-research-api'
```

---

## Appendix

### Common Error Codes

| Error Code | Meaning | Typical Cause |
|------------|---------|---------------|
| 400 | Bad Request | Invalid request body/parameters |
| 401 | Unauthorized | Missing/invalid authentication |
| 403 | Forbidden | Insufficient permissions |
| 404 | Not Found | Resource doesn't exist |
| 409 | Conflict | Duplicate resource |
| 422 | Unprocessable Entity | Validation failed |
| 429 | Too Many Requests | Rate limit exceeded |
| 500 | Internal Server Error | Application error |
| 502 | Bad Gateway | Upstream service unavailable |
| 503 | Service Unavailable | Service overloaded |
| 504 | Gateway Timeout | Upstream timeout |

### Environment Variables Reference

| Variable | Description | Default |
|----------|-------------|---------|
| `DATABASE_URL` | PostgreSQL connection string | Required |
| `CLICKHOUSE_URL` | ClickHouse connection string | Required |
| `S3_BUCKET` | Dataset storage bucket | Required |
| `JWT_SECRET` | JWT signing secret | Required |
| `LOG_LEVEL` | Logging verbosity | `info` |
| `SERVER_PORT` | API server port | `8080` |
| `METRICS_PORT` | Metrics server port | `9090` |
| `CONNECTION_POOL_SIZE` | DB connection pool | `50` |
| `CACHE_TTL_SECONDS` | Cache entry TTL | `300` |

### Useful Scripts

All troubleshooting scripts are available at:
```
/workspaces/llm-research-lab/scripts/troubleshooting/
├── quick-health-check.sh
├── diagnose-database.sh
├── diagnose-network.sh
├── collect-debug-info.sh
└── analyze-logs.sh
```
