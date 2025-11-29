# LLM Research Lab - Rollback Procedures

## Overview

This document provides comprehensive rollback procedures for the LLM Research Lab platform. Rollbacks should be executed when a deployment causes issues that cannot be quickly resolved through other means.

## Table of Contents
- [Decision Matrix](#decision-matrix)
- [Application Rollback](#application-rollback)
- [Database Rollback](#database-rollback)
- [Configuration Rollback](#configuration-rollback)
- [Infrastructure Rollback](#infrastructure-rollback)
- [Verification Procedures](#verification-procedures)
- [Post-Rollback Actions](#post-rollback-actions)

---

## Decision Matrix

### When to Rollback

| Condition | Action | Time to Decision |
|-----------|--------|------------------|
| Error rate > 10% for > 2 minutes | Immediate rollback | 2 minutes |
| Error rate > 5% for > 5 minutes | Consider rollback | 5 minutes |
| P95 latency > 5s for > 5 minutes | Investigate, prepare rollback | 5 minutes |
| Critical security vulnerability | Immediate rollback | Immediate |
| Data corruption detected | Immediate rollback + restore | Immediate |
| Memory leak causing OOM | Rollback if no quick fix | 15 minutes |
| Single endpoint broken | Partial rollback/feature flag | 30 minutes |

### Rollback vs. Hotfix Decision

```
Was the issue introduced in the last deployment?
├── Yes
│   ├── Can it be fixed in < 15 minutes?
│   │   ├── Yes → Deploy hotfix
│   │   └── No → Rollback
│   └── Is it critical (SEV-1/SEV-2)?
│       ├── Yes → Rollback immediately
│       └── No → Attempt hotfix, rollback if fails
└── No
    └── Investigate other causes
```

---

## Application Rollback

### Kubernetes Deployment Rollback

#### Quick Rollback (Most Common)

```bash
#!/bin/bash
# rollback-application.sh

set -e

NAMESPACE="${NAMESPACE:-llm-research}"
DEPLOYMENT="${DEPLOYMENT:-llm-research-api}"
REVISION="${1:-}" # Optional: specific revision number

echo "=== APPLICATION ROLLBACK ==="
echo "Timestamp: $(date -u +%Y-%m-%dT%H:%M:%SZ)"
echo "Namespace: $NAMESPACE"
echo "Deployment: $DEPLOYMENT"
echo ""

# Step 1: Get current version
CURRENT_IMAGE=$(kubectl get deployment $DEPLOYMENT -n $NAMESPACE \
  -o jsonpath='{.spec.template.spec.containers[0].image}')
echo "Current Image: $CURRENT_IMAGE"

# Step 2: Get revision history
echo ""
echo "Available Revisions:"
kubectl rollout history deployment/$DEPLOYMENT -n $NAMESPACE

# Step 3: Execute rollback
if [ -z "$REVISION" ]; then
  echo ""
  echo "Rolling back to previous revision..."
  kubectl rollout undo deployment/$DEPLOYMENT -n $NAMESPACE
else
  echo ""
  echo "Rolling back to revision $REVISION..."
  kubectl rollout undo deployment/$DEPLOYMENT -n $NAMESPACE --to-revision=$REVISION
fi

# Step 4: Wait for rollback to complete
echo ""
echo "Waiting for rollback to complete..."
kubectl rollout status deployment/$DEPLOYMENT -n $NAMESPACE --timeout=300s

# Step 5: Verify new version
NEW_IMAGE=$(kubectl get deployment $DEPLOYMENT -n $NAMESPACE \
  -o jsonpath='{.spec.template.spec.containers[0].image}')
echo ""
echo "Rolled back to: $NEW_IMAGE"

# Step 6: Verify health
echo ""
echo "Verifying health..."
sleep 10

HEALTH_STATUS=$(kubectl exec deployment/$DEPLOYMENT -n $NAMESPACE -- \
  curl -s http://localhost:8080/health)
echo "Health Status: $HEALTH_STATUS"

echo ""
echo "=== ROLLBACK COMPLETE ==="
```

#### Rollback to Specific Version

```bash
# List all revisions with details
kubectl rollout history deployment/llm-research-api -n llm-research

# View specific revision details
kubectl rollout history deployment/llm-research-api -n llm-research --revision=5

# Rollback to specific revision
kubectl rollout undo deployment/llm-research-api -n llm-research --to-revision=5
```

#### Rollback with Image Tag

```bash
# Rollback by setting specific image version
kubectl set image deployment/llm-research-api \
  api=ghcr.io/llm-research-lab/api:v1.2.2 \
  -n llm-research

# Wait for rollout
kubectl rollout status deployment/llm-research-api -n llm-research
```

### Blue-Green Rollback

```bash
#!/bin/bash
# rollback-blue-green.sh

set -e

NAMESPACE="llm-research"
SERVICE="llm-research-api"

# Get current active version
CURRENT_VERSION=$(kubectl get service $SERVICE -n $NAMESPACE \
  -o jsonpath='{.spec.selector.version}')

echo "Current active version: $CURRENT_VERSION"

# Determine rollback target
if [ "$CURRENT_VERSION" = "green" ]; then
  ROLLBACK_VERSION="blue"
else
  ROLLBACK_VERSION="green"
fi

echo "Rolling back to: $ROLLBACK_VERSION"

# Verify target deployment exists and is healthy
TARGET_READY=$(kubectl get deployment ${SERVICE}-${ROLLBACK_VERSION} -n $NAMESPACE \
  -o jsonpath='{.status.readyReplicas}')

if [ -z "$TARGET_READY" ] || [ "$TARGET_READY" -eq 0 ]; then
  echo "ERROR: Target deployment ${ROLLBACK_VERSION} is not ready"
  exit 1
fi

# Switch service selector
kubectl patch service $SERVICE -n $NAMESPACE \
  -p "{\"spec\":{\"selector\":{\"version\":\"${ROLLBACK_VERSION}\"}}}"

echo "Traffic switched to $ROLLBACK_VERSION"

# Verify switch
NEW_VERSION=$(kubectl get service $SERVICE -n $NAMESPACE \
  -o jsonpath='{.spec.selector.version}')
echo "Service now pointing to: $NEW_VERSION"
```

### Canary Rollback

```bash
#!/bin/bash
# rollback-canary.sh

# Remove canary from traffic immediately
kubectl patch virtualservice llm-research-api -n llm-research \
  --type=json -p='[
    {"op": "replace", "path": "/spec/http/0/route/0/weight", "value": 100},
    {"op": "replace", "path": "/spec/http/0/route/1/weight", "value": 0}
  ]'

# Scale down canary deployment
kubectl scale deployment llm-research-api-canary -n llm-research --replicas=0

echo "Canary traffic removed and deployment scaled down"
```

---

## Database Rollback

### Schema Migration Rollback

#### Automated Migration Rollback

```bash
#!/bin/bash
# rollback-migration.sh

set -e

MIGRATION_VERSION="${1:-}"
DATABASE_URL="${DATABASE_URL}"

if [ -z "$MIGRATION_VERSION" ]; then
  echo "Usage: rollback-migration.sh <version>"
  echo "Example: rollback-migration.sh 20250115120000"
  exit 1
fi

echo "=== DATABASE MIGRATION ROLLBACK ==="
echo "Target version: $MIGRATION_VERSION"
echo ""

# Step 1: Create backup
BACKUP_NAME="pre-rollback-$(date +%Y%m%d-%H%M%S)"
echo "Creating backup: $BACKUP_NAME"
pg_dump $DATABASE_URL > "/tmp/${BACKUP_NAME}.sql"

# Upload to S3
aws s3 cp "/tmp/${BACKUP_NAME}.sql" "s3://${BACKUP_BUCKET}/migrations/${BACKUP_NAME}.sql"
echo "Backup uploaded to S3"

# Step 2: Run migration rollback
echo ""
echo "Rolling back to version $MIGRATION_VERSION..."
sqlx migrate revert --database-url $DATABASE_URL --target-version $MIGRATION_VERSION

# Step 3: Verify migration state
echo ""
echo "Current migration state:"
sqlx migrate info --database-url $DATABASE_URL

echo ""
echo "=== MIGRATION ROLLBACK COMPLETE ==="
```

#### Manual SQL Rollback

```sql
-- Example: Rollback column addition
-- rollback-20250115-add-column.sql

BEGIN;

-- Verify we're in the expected state
DO $$
BEGIN
  IF NOT EXISTS (
    SELECT 1 FROM information_schema.columns
    WHERE table_name = 'experiments' AND column_name = 'new_column'
  ) THEN
    RAISE EXCEPTION 'Column does not exist - nothing to rollback';
  END IF;
END $$;

-- Perform rollback
ALTER TABLE experiments DROP COLUMN IF EXISTS new_column;

-- Update migration tracking
DELETE FROM _sqlx_migrations WHERE version = 20250115120000;

COMMIT;
```

### Data Restore from Backup

```bash
#!/bin/bash
# restore-from-backup.sh

set -e

BACKUP_FILE="${1}"
RESTORE_DATABASE="${2:-llm_research_restore}"

if [ -z "$BACKUP_FILE" ]; then
  echo "Usage: restore-from-backup.sh <backup-file> [restore-database]"
  exit 1
fi

echo "=== DATABASE RESTORE ==="
echo "Backup file: $BACKUP_FILE"
echo "Restore to: $RESTORE_DATABASE"
echo ""

# Step 1: Download backup if S3 path
if [[ "$BACKUP_FILE" == s3://* ]]; then
  LOCAL_FILE="/tmp/$(basename $BACKUP_FILE)"
  aws s3 cp "$BACKUP_FILE" "$LOCAL_FILE"
  BACKUP_FILE="$LOCAL_FILE"
fi

# Step 2: Create restore database
psql $DATABASE_URL -c "DROP DATABASE IF EXISTS $RESTORE_DATABASE;"
psql $DATABASE_URL -c "CREATE DATABASE $RESTORE_DATABASE;"

# Step 3: Restore backup
RESTORE_URL=$(echo $DATABASE_URL | sed "s|/[^/]*$|/${RESTORE_DATABASE}|")
pg_restore -d "$RESTORE_URL" "$BACKUP_FILE" || psql "$RESTORE_URL" < "$BACKUP_FILE"

# Step 4: Verify restore
echo ""
echo "Verifying restore..."
psql "$RESTORE_URL" -c "SELECT count(*) FROM experiments;"
psql "$RESTORE_URL" -c "SELECT count(*) FROM models;"
psql "$RESTORE_URL" -c "SELECT count(*) FROM datasets;"

echo ""
echo "=== RESTORE COMPLETE ==="
echo ""
echo "To switch to restored database:"
echo "1. Update application DATABASE_URL to point to $RESTORE_DATABASE"
echo "2. Restart application pods"
echo "3. Verify application health"
```

### Point-in-Time Recovery (RDS)

```bash
#!/bin/bash
# rds-pitr-restore.sh

INSTANCE_ID="llm-research-db"
RESTORE_TIME="${1}" # Format: 2025-01-15T10:30:00Z
NEW_INSTANCE_ID="${INSTANCE_ID}-restored-$(date +%Y%m%d%H%M%S)"

echo "Initiating point-in-time recovery..."
echo "Source: $INSTANCE_ID"
echo "Restore time: $RESTORE_TIME"
echo "New instance: $NEW_INSTANCE_ID"

# Initiate restore
aws rds restore-db-instance-to-point-in-time \
  --source-db-instance-identifier $INSTANCE_ID \
  --target-db-instance-identifier $NEW_INSTANCE_ID \
  --restore-time $RESTORE_TIME \
  --db-instance-class db.r6g.xlarge \
  --no-multi-az \
  --tags Key=Environment,Value=production Key=RestoredFrom,Value=$INSTANCE_ID

# Wait for instance to be available
echo "Waiting for instance to be available..."
aws rds wait db-instance-available --db-instance-identifier $NEW_INSTANCE_ID

# Get new endpoint
NEW_ENDPOINT=$(aws rds describe-db-instances \
  --db-instance-identifier $NEW_INSTANCE_ID \
  --query 'DBInstances[0].Endpoint.Address' \
  --output text)

echo ""
echo "Restore complete!"
echo "New endpoint: $NEW_ENDPOINT"
echo ""
echo "Next steps:"
echo "1. Verify data in restored instance"
echo "2. Update application configuration"
echo "3. Switch traffic to new instance"
```

---

## Configuration Rollback

### ConfigMap Rollback

```bash
#!/bin/bash
# rollback-configmap.sh

NAMESPACE="llm-research"
CONFIGMAP="llm-research-config"
BACKUP_DIR="/tmp/configmap-backups"

# List available backups
ls -la $BACKUP_DIR/${CONFIGMAP}*.yaml

# Restore from backup
BACKUP_FILE="${1}"
if [ -z "$BACKUP_FILE" ]; then
  echo "Usage: rollback-configmap.sh <backup-file>"
  exit 1
fi

# Apply backup
kubectl apply -f "$BACKUP_FILE" -n $NAMESPACE

# Restart pods to pick up new config
kubectl rollout restart deployment/llm-research-api -n $NAMESPACE
kubectl rollout status deployment/llm-research-api -n $NAMESPACE
```

### Secret Rollback

```bash
#!/bin/bash
# rollback-secret.sh

NAMESPACE="llm-research"
SECRET_NAME="llm-research-secrets"
SECRET_VERSION="${1}" # AWS Secrets Manager version

# Restore from AWS Secrets Manager version
aws secretsmanager get-secret-value \
  --secret-id llm-research/production \
  --version-id "$SECRET_VERSION" \
  --query SecretString \
  --output text > /tmp/secret-restore.json

# Update Kubernetes secret
kubectl create secret generic $SECRET_NAME \
  --from-file=config=/tmp/secret-restore.json \
  --dry-run=client -o yaml | kubectl apply -f - -n $NAMESPACE

# Cleanup
rm /tmp/secret-restore.json

# Restart pods
kubectl rollout restart deployment/llm-research-api -n $NAMESPACE
```

### Feature Flag Rollback

```bash
# Disable problematic feature flag immediately
curl -X PATCH https://api.featureflags.io/flags/new-feature \
  -H "Authorization: Bearer $FF_TOKEN" \
  -d '{"enabled": false}'

# Or via ConfigMap
kubectl patch configmap llm-research-config -n llm-research \
  --type merge -p '{"data":{"FEATURE_NEW_ALGORITHM":"false"}}'
```

---

## Infrastructure Rollback

### Terraform State Rollback

```bash
#!/bin/bash
# rollback-terraform.sh

WORKSPACE="${1:-production}"
STATE_VERSION="${2}"

cd terraform/environments/$WORKSPACE

# List state versions
terraform state list

# If using S3 backend with versioning
aws s3api list-object-versions \
  --bucket terraform-state-bucket \
  --prefix "llm-research/$WORKSPACE/terraform.tfstate"

# Restore specific version
aws s3api get-object \
  --bucket terraform-state-bucket \
  --key "llm-research/$WORKSPACE/terraform.tfstate" \
  --version-id "$STATE_VERSION" \
  terraform.tfstate.backup

# Apply previous state
terraform plan -target=module.api -out=rollback.plan
terraform apply rollback.plan
```

### Helm Release Rollback

```bash
# List release history
helm history llm-research-api -n llm-research

# Rollback to specific revision
helm rollback llm-research-api 3 -n llm-research

# Verify rollback
helm status llm-research-api -n llm-research
```

### Load Balancer/DNS Rollback

```bash
#!/bin/bash
# rollback-traffic.sh

# Route 53 failover
aws route53 change-resource-record-sets \
  --hosted-zone-id $HOSTED_ZONE_ID \
  --change-batch '{
    "Changes": [{
      "Action": "UPSERT",
      "ResourceRecordSet": {
        "Name": "api.llm-research-lab.io",
        "Type": "A",
        "AliasTarget": {
          "HostedZoneId": "'$ALB_ZONE_ID'",
          "DNSName": "'$BACKUP_ALB_DNS'",
          "EvaluateTargetHealth": true
        }
      }
    }]
  }'
```

---

## Verification Procedures

### Post-Rollback Verification Checklist

```bash
#!/bin/bash
# verify-rollback.sh

set -e

API_URL="${API_URL:-https://api.llm-research-lab.io}"

echo "=== POST-ROLLBACK VERIFICATION ==="
echo "Timestamp: $(date -u +%Y-%m-%dT%H:%M:%SZ)"
echo ""

# 1. Pod Health
echo "1. Checking pod health..."
kubectl get pods -n llm-research -l app=llm-research-api
READY_PODS=$(kubectl get pods -n llm-research -l app=llm-research-api \
  -o jsonpath='{.items[*].status.conditions[?(@.type=="Ready")].status}' | grep -c True)
TOTAL_PODS=$(kubectl get pods -n llm-research -l app=llm-research-api --no-headers | wc -l)
echo "Ready: $READY_PODS / $TOTAL_PODS"
[ "$READY_PODS" -eq "$TOTAL_PODS" ] || { echo "FAILED: Not all pods ready"; exit 1; }
echo "OK"

# 2. API Health
echo ""
echo "2. Checking API health..."
HEALTH_STATUS=$(curl -sf "$API_URL/health" || echo "FAILED")
echo "Health endpoint: $HEALTH_STATUS"
[ "$HEALTH_STATUS" = "OK" ] || { echo "FAILED: Health check failed"; exit 1; }
echo "OK"

# 3. Readiness Check
echo ""
echo "3. Checking readiness..."
READY_STATUS=$(curl -sf "$API_URL/health/ready" | jq -r '.status')
echo "Readiness status: $READY_STATUS"
[ "$READY_STATUS" = "healthy" ] || { echo "FAILED: Readiness check failed"; exit 1; }
echo "OK"

# 4. Error Rate
echo ""
echo "4. Checking error rate..."
ERROR_RATE=$(curl -s "http://prometheus:9090/api/v1/query?query=rate(http_requests_total{status=~\"5..\"}[1m])/rate(http_requests_total[1m])" \
  | jq -r '.data.result[0].value[1] // "0"')
echo "Current error rate: $ERROR_RATE"
(( $(echo "$ERROR_RATE < 0.01" | bc -l) )) || { echo "WARNING: Error rate elevated"; }
echo "OK"

# 5. Latency
echo ""
echo "5. Checking latency..."
P95_LATENCY=$(curl -s "http://prometheus:9090/api/v1/query?query=histogram_quantile(0.95,rate(http_request_duration_seconds_bucket[1m]))" \
  | jq -r '.data.result[0].value[1] // "0"')
echo "P95 latency: ${P95_LATENCY}s"
(( $(echo "$P95_LATENCY < 2.0" | bc -l) )) || { echo "WARNING: Latency elevated"; }
echo "OK"

# 6. Database Connectivity
echo ""
echo "6. Checking database connectivity..."
DB_CHECK=$(curl -sf "$API_URL/experiments?limit=1" -H "Authorization: Bearer $TEST_TOKEN" || echo "FAILED")
[ "$DB_CHECK" != "FAILED" ] || { echo "FAILED: Database connectivity"; exit 1; }
echo "OK"

# 7. Version Check
echo ""
echo "7. Checking deployed version..."
DEPLOYED_VERSION=$(kubectl get deployment llm-research-api -n llm-research \
  -o jsonpath='{.spec.template.spec.containers[0].image}')
echo "Deployed version: $DEPLOYED_VERSION"
echo "OK"

echo ""
echo "=== VERIFICATION COMPLETE ==="
echo "All checks passed"
```

### Smoke Tests

```bash
#!/bin/bash
# smoke-tests.sh

API_URL="${API_URL:-https://api.llm-research-lab.io}"
TOKEN="${TEST_TOKEN}"

echo "Running smoke tests..."

# Test 1: List models
echo -n "GET /models... "
curl -sf "$API_URL/models" -H "Authorization: Bearer $TOKEN" > /dev/null && echo "OK" || echo "FAILED"

# Test 2: List providers
echo -n "GET /models/providers... "
curl -sf "$API_URL/models/providers" -H "Authorization: Bearer $TOKEN" > /dev/null && echo "OK" || echo "FAILED"

# Test 3: List experiments
echo -n "GET /experiments... "
curl -sf "$API_URL/experiments" -H "Authorization: Bearer $TOKEN" > /dev/null && echo "OK" || echo "FAILED"

# Test 4: List datasets
echo -n "GET /datasets... "
curl -sf "$API_URL/datasets" -H "Authorization: Bearer $TOKEN" > /dev/null && echo "OK" || echo "FAILED"

# Test 5: List prompts
echo -n "GET /prompts... "
curl -sf "$API_URL/prompts" -H "Authorization: Bearer $TOKEN" > /dev/null && echo "OK" || echo "FAILED"

# Test 6: Metrics endpoint
echo -n "GET /metrics... "
curl -sf "$API_URL/metrics" > /dev/null && echo "OK" || echo "FAILED"

echo "Smoke tests complete"
```

---

## Post-Rollback Actions

### Communication

```markdown
# Internal Communication Template

**Rollback Completed**

**Time:** [HH:MM UTC]
**Component:** [Component name]
**Rolled back from:** [Version]
**Rolled back to:** [Version]
**Reason:** [Brief reason]

**Current Status:** Service restored, monitoring for stability

**Next Steps:**
1. Post-incident review scheduled for [date/time]
2. Root cause analysis in progress
3. Fix ETA: [estimate or TBD]

Please report any issues to #[channel]
```

### Documentation

```bash
# Log rollback event
cat >> /var/log/rollback-history.log <<EOF
$(date -u +%Y-%m-%dT%H:%M:%SZ) | ROLLBACK | $DEPLOYMENT | $OLD_VERSION -> $NEW_VERSION | $REASON
EOF

# Update runbook if needed
# Create JIRA ticket for RCA
```

### Monitoring Adjustment

```bash
# Increase alerting sensitivity temporarily
kubectl patch configmap alertmanager-config -n monitoring \
  --type merge -p '{
    "data": {
      "alerting_rules.yml": "..."
    }
  }'

# Set up additional metrics collection
kubectl annotate deployment llm-research-api -n llm-research \
  "rollback.monitoring/enhanced=true"
```

---

## Appendix

### Rollback Checklist

**Pre-Rollback:**
- [ ] Confirmed issue is caused by recent change
- [ ] Documented current state and symptoms
- [ ] Notified team in incident channel
- [ ] Identified target rollback version

**During Rollback:**
- [ ] Executed rollback procedure
- [ ] Monitored rollout progress
- [ ] No new errors introduced

**Post-Rollback:**
- [ ] Verified service health
- [ ] Ran smoke tests
- [ ] Checked error rates and latency
- [ ] Updated status page
- [ ] Documented rollback details
- [ ] Scheduled post-incident review

### Quick Reference

| Action | Command |
|--------|---------|
| Quick rollback | `kubectl rollout undo deployment/llm-research-api -n llm-research` |
| Rollback to revision | `kubectl rollout undo deployment/llm-research-api -n llm-research --to-revision=N` |
| Check rollout status | `kubectl rollout status deployment/llm-research-api -n llm-research` |
| View history | `kubectl rollout history deployment/llm-research-api -n llm-research` |
| Scale down | `kubectl scale deployment/llm-research-api -n llm-research --replicas=0` |
| Helm rollback | `helm rollback llm-research-api N -n llm-research` |
