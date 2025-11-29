# LLM Research Lab - Operations Training Materials

## Overview

This document provides comprehensive training materials for operations engineers responsible for managing the LLM Research Lab platform in production.

## Table of Contents
- [System Overview](#system-overview)
- [Day-to-Day Operations](#day-to-day-operations)
- [Monitoring and Alerting](#monitoring-and-alerting)
- [Common Operational Tasks](#common-operational-tasks)
- [Emergency Procedures](#emergency-procedures)
- [Hands-On Exercises](#hands-on-exercises)
- [Certification Checklist](#certification-checklist)

---

## System Overview

### Architecture Summary

The LLM Research Lab platform consists of:

```
┌─────────────────────────────────────────────────────────────────────┐
│                        Production Environment                        │
├─────────────────────────────────────────────────────────────────────┤
│                                                                      │
│  ┌──────────────┐   ┌──────────────┐   ┌──────────────────────────┐ │
│  │   API Layer  │   │  Data Layer  │   │   External Services      │ │
│  │              │   │              │   │                          │ │
│  │ • 3-20 pods  │   │ • PostgreSQL │   │ • OpenAI API             │ │
│  │ • Rust/Axum  │   │ • ClickHouse │   │ • Anthropic API          │ │
│  │ • Stateless  │   │ • Amazon S3  │   │ • Other LLM providers    │ │
│  └──────────────┘   └──────────────┘   └──────────────────────────┘ │
│                                                                      │
└─────────────────────────────────────────────────────────────────────┘
```

### Key Components

| Component | Description | SLA Impact |
|-----------|-------------|------------|
| API Pods | Handle all HTTP requests | Critical |
| PostgreSQL | Stores experiments, models, datasets | Critical |
| ClickHouse | Stores time-series metrics | High |
| S3 | Stores dataset files | High |
| Load Balancer | Routes traffic | Critical |

### Access Requirements

| System | Access Method | Credentials Location |
|--------|---------------|---------------------|
| Kubernetes | kubectl + kubeconfig | AWS EKS |
| PostgreSQL | psql / DBeaver | AWS Secrets Manager |
| ClickHouse | clickhouse-client | Kubernetes Secret |
| S3 | aws-cli | IAM Role |
| Monitoring | Grafana UI | SSO |

---

## Day-to-Day Operations

### Morning Checklist

```bash
#!/bin/bash
# morning-checklist.sh

echo "=== LLM Research Lab Morning Checklist ==="
echo "Date: $(date)"
echo ""

# 1. Check overall system health
echo "1. System Health"
kubectl get pods -n llm-research
kubectl top pods -n llm-research

# 2. Check for alerts
echo ""
echo "2. Active Alerts"
curl -s http://alertmanager:9093/api/v2/alerts | jq '.[] | select(.status.state == "active") | {alertname: .labels.alertname, severity: .labels.severity}'

# 3. Check error rates (last 24h)
echo ""
echo "3. Error Rate (24h)"
curl -s "http://prometheus:9090/api/v1/query?query=sum(increase(http_requests_total{status=~\"5..\"}[24h]))/sum(increase(http_requests_total[24h]))" | jq '.data.result[0].value[1]'

# 4. Check database health
echo ""
echo "4. Database Health"
psql $DATABASE_URL -c "SELECT count(*) as connections FROM pg_stat_activity WHERE state = 'active';"

# 5. Check disk usage
echo ""
echo "5. Storage Status"
kubectl exec deployment/llm-research-api -n llm-research -- df -h /

# 6. Review recent deployments
echo ""
echo "6. Recent Deployments"
kubectl rollout history deployment/llm-research-api -n llm-research | tail -5

echo ""
echo "=== Checklist Complete ==="
```

### Key Metrics to Monitor

| Metric | Normal Range | Alert Threshold | Action |
|--------|--------------|-----------------|--------|
| Error Rate | < 0.1% | > 1% | Investigate |
| P95 Latency | < 500ms | > 2s | Scale or optimize |
| CPU Usage | < 70% | > 85% | Scale out |
| Memory Usage | < 80% | > 90% | Scale out or investigate leak |
| DB Connections | < 80 | > 90 | Check for leaks |
| Disk Usage | < 70% | > 80% | Add storage or cleanup |

### Regular Maintenance Tasks

#### Weekly Tasks

| Task | Procedure | Duration |
|------|-----------|----------|
| Review error logs | Search for recurring errors | 30 min |
| Check security alerts | Review vulnerability scan | 15 min |
| Validate backups | Test restore procedure | 1 hour |
| Review capacity | Check growth trends | 15 min |

#### Monthly Tasks

| Task | Procedure | Duration |
|------|-----------|----------|
| Security patching | Apply OS/container updates | 2 hours |
| Certificate renewal check | Verify expiry dates | 15 min |
| Cost optimization | Review resource usage | 1 hour |
| Runbook review | Update procedures | 2 hours |

---

## Monitoring and Alerting

### Grafana Dashboards

| Dashboard | Purpose | URL |
|-----------|---------|-----|
| System Overview | High-level health | /d/overview |
| API Performance | Request metrics | /d/api-performance |
| Database | PostgreSQL/ClickHouse | /d/database |
| Business KPIs | Experiments, models | /d/business |
| SLO Tracking | Error budget | /d/slo |

### Alert Response Guide

#### SEV-1: Critical

**Response Time:** 15 minutes

**Examples:**
- API completely unavailable
- Database primary down
- Data breach detected

**Actions:**
1. Acknowledge alert immediately
2. Join incident channel
3. Begin investigation
4. Escalate if not resolved in 30 minutes

#### SEV-2: Major

**Response Time:** 30 minutes

**Examples:**
- Error rate > 5%
- P95 latency > 5 seconds
- Single component failure

**Actions:**
1. Acknowledge alert
2. Investigate root cause
3. Implement mitigation
4. Document findings

#### SEV-3: Minor

**Response Time:** 2 hours

**Examples:**
- Error rate > 1%
- Non-critical feature degraded
- Single pod failure

**Actions:**
1. Acknowledge alert
2. Schedule investigation
3. Fix during business hours

### Alert Tuning

```yaml
# Example: Reduce alert noise for expected behavior
- alert: HighLatency
  expr: histogram_quantile(0.95, rate(http_request_duration_seconds_bucket[5m])) > 2
  for: 10m  # Increased from 5m to reduce flapping
  labels:
    severity: warning
```

---

## Common Operational Tasks

### Scaling Operations

#### Scale Up (Manual)

```bash
# Scale API pods
kubectl scale deployment llm-research-api -n llm-research --replicas=10

# Verify scaling
kubectl get pods -n llm-research -l app=llm-research-api -w
```

#### Scale Down

```bash
# Scale down after peak
kubectl scale deployment llm-research-api -n llm-research --replicas=3

# Verify graceful shutdown
kubectl get pods -n llm-research -l app=llm-research-api
```

#### Configure HPA

```bash
# View current HPA status
kubectl get hpa -n llm-research

# Describe HPA for details
kubectl describe hpa llm-research-api-hpa -n llm-research
```

### Log Investigation

#### View Real-time Logs

```bash
# All API pods
kubectl logs -f -n llm-research -l app=llm-research-api

# Specific pod
kubectl logs -f -n llm-research llm-research-api-xxxxx

# With timestamps
kubectl logs -n llm-research -l app=llm-research-api --timestamps
```

#### Search for Errors

```bash
# Recent errors
kubectl logs -n llm-research -l app=llm-research-api --since=1h | grep -i error

# Specific error pattern
kubectl logs -n llm-research -l app=llm-research-api | grep "database connection"
```

#### Export Logs

```bash
# Export to file
kubectl logs -n llm-research -l app=llm-research-api --since=24h > /tmp/api-logs-$(date +%Y%m%d).log

# Compress and archive
gzip /tmp/api-logs-*.log
aws s3 cp /tmp/api-logs-*.gz s3://llm-research-logs/
```

### Database Operations

#### Check Connection Status

```bash
psql $DATABASE_URL <<EOF
-- Active connections
SELECT count(*), state
FROM pg_stat_activity
GROUP BY state;

-- Long-running queries
SELECT pid, now() - query_start AS duration, query
FROM pg_stat_activity
WHERE state = 'active'
  AND (now() - query_start) > interval '1 minute'
ORDER BY duration DESC;
EOF
```

#### Kill Problematic Query

```bash
# Find the PID first
psql $DATABASE_URL -c "SELECT pid, query FROM pg_stat_activity WHERE state = 'active';"

# Kill specific query
psql $DATABASE_URL -c "SELECT pg_terminate_backend(<PID>);"
```

#### Vacuum and Analyze

```bash
# Run vacuum on specific table
psql $DATABASE_URL -c "VACUUM ANALYZE experiments;"

# Run vacuum on all tables
psql $DATABASE_URL -c "VACUUM ANALYZE;"
```

### Certificate Management

```bash
# Check certificate expiry
kubectl get certificate -n llm-research

# Check secret expiry
kubectl get secret llm-research-tls -n llm-research -o jsonpath='{.data.tls\.crt}' | base64 -d | openssl x509 -noout -enddate

# Renew certificate (cert-manager)
kubectl delete certificate llm-research-tls -n llm-research
# cert-manager will automatically recreate
```

---

## Emergency Procedures

### Service Outage

```bash
# 1. Quick assessment
kubectl get pods -n llm-research
kubectl get events -n llm-research --sort-by=.lastTimestamp | tail -20

# 2. Check recent changes
kubectl rollout history deployment/llm-research-api -n llm-research

# 3. Rollback if needed
kubectl rollout undo deployment/llm-research-api -n llm-research

# 4. Verify recovery
kubectl rollout status deployment/llm-research-api -n llm-research
```

### Database Failover

```bash
# Check RDS status
aws rds describe-db-instances --db-instance-identifier llm-research-db

# Initiate failover (if needed)
aws rds reboot-db-instance --db-instance-identifier llm-research-db --force-failover

# Monitor failover
watch -n 5 'aws rds describe-db-instances --db-instance-identifier llm-research-db --query "DBInstances[0].DBInstanceStatus"'
```

### Emergency Contacts

| Role | Contact | When to Escalate |
|------|---------|------------------|
| On-Call Primary | PagerDuty | First responder |
| On-Call Secondary | PagerDuty | Primary unavailable |
| Platform Lead | Direct | SEV-1 > 30 min |
| Engineering Manager | Direct | SEV-1 > 1 hour |
| VP Engineering | Direct | SEV-1 > 2 hours |

---

## Hands-On Exercises

### Exercise 1: Pod Restart

**Scenario:** A pod is consuming excessive memory and needs to be restarted.

```bash
# 1. Identify the problematic pod
kubectl top pods -n llm-research -l app=llm-research-api

# 2. Delete the pod (will be recreated by deployment)
kubectl delete pod <pod-name> -n llm-research

# 3. Verify new pod is healthy
kubectl get pods -n llm-research -l app=llm-research-api
kubectl logs <new-pod-name> -n llm-research | tail -20
```

### Exercise 2: Investigate High Latency

**Scenario:** P95 latency has spiked to 3 seconds.

```bash
# 1. Check which endpoints are slow
curl -s "http://prometheus:9090/api/v1/query?query=topk(5,histogram_quantile(0.95,rate(http_request_duration_seconds_bucket[5m])))"

# 2. Check database query times
kubectl logs -n llm-research -l app=llm-research-api | grep "query_time_ms" | tail -20

# 3. Check pod resource usage
kubectl top pods -n llm-research -l app=llm-research-api

# 4. Check for slow queries
psql $DATABASE_URL -c "SELECT query, mean_time FROM pg_stat_statements ORDER BY mean_time DESC LIMIT 10;"
```

### Exercise 3: Perform Rolling Update

**Scenario:** Deploy a new version with zero downtime.

```bash
# 1. Check current version
kubectl get deployment llm-research-api -n llm-research -o jsonpath='{.spec.template.spec.containers[0].image}'

# 2. Update to new version
kubectl set image deployment/llm-research-api api=ghcr.io/llm-research-lab/api:v1.2.4 -n llm-research

# 3. Monitor rollout
kubectl rollout status deployment/llm-research-api -n llm-research

# 4. Verify health
curl -s https://api.llm-research-lab.io/health

# 5. Rollback if needed
kubectl rollout undo deployment/llm-research-api -n llm-research
```

### Exercise 4: Database Backup Verification

**Scenario:** Verify that database backups are working correctly.

```bash
# 1. List recent backups
aws s3 ls s3://llm-research-backups/postgresql/ | tail -5

# 2. Download a backup
aws s3 cp s3://llm-research-backups/postgresql/backup-20250115.sql.gz /tmp/

# 3. Create test database
psql $DATABASE_URL -c "CREATE DATABASE backup_test;"

# 4. Restore backup to test database
gunzip -c /tmp/backup-20250115.sql.gz | psql postgresql://...backup_test

# 5. Verify data
psql postgresql://...backup_test -c "SELECT count(*) FROM experiments;"

# 6. Cleanup
psql $DATABASE_URL -c "DROP DATABASE backup_test;"
```

---

## Certification Checklist

### Level 1: Basic Operations

- [ ] Can access all monitoring dashboards
- [ ] Can view logs from all components
- [ ] Can restart a single pod
- [ ] Can check system health
- [ ] Can acknowledge alerts
- [ ] Understands escalation procedures

### Level 2: Intermediate Operations

- [ ] Can perform rolling deployments
- [ ] Can rollback a deployment
- [ ] Can scale pods manually
- [ ] Can investigate common issues
- [ ] Can perform database queries
- [ ] Can manage certificates

### Level 3: Advanced Operations

- [ ] Can handle SEV-1 incidents independently
- [ ] Can perform database failover
- [ ] Can restore from backups
- [ ] Can tune alerts and dashboards
- [ ] Can write runbook updates
- [ ] Can train Level 1 operators

### Certification Sign-off

```
Level 1 Certified: _________________ Date: _________
Level 2 Certified: _________________ Date: _________
Level 3 Certified: _________________ Date: _________

Trainer Signature: _________________
```

---

## Additional Resources

### Documentation

- [Deployment Runbook](./DEPLOYMENT_RUNBOOK.md)
- [Incident Response Playbook](./INCIDENT_RESPONSE.md)
- [Rollback Procedures](./ROLLBACK_PROCEDURES.md)
- [Troubleshooting Guide](./TROUBLESHOOTING_GUIDE.md)

### External Resources

- [Kubernetes Documentation](https://kubernetes.io/docs/)
- [PostgreSQL Administration](https://www.postgresql.org/docs/current/admin.html)
- [ClickHouse Operations](https://clickhouse.com/docs/en/operations)
- [AWS EKS Best Practices](https://aws.github.io/aws-eks-best-practices/)

### Support Channels

- **Slack:** #ops-help
- **PagerDuty:** For escalations
- **GitHub:** Issue tracking
- **Wiki:** Internal documentation
