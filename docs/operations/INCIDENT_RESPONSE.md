# LLM Research Lab - Incident Response Playbook

## Overview

This playbook provides structured procedures for responding to production incidents affecting the LLM Research Lab platform. All team members should be familiar with these procedures.

## Table of Contents
- [Incident Classification](#incident-classification)
- [Incident Response Process](#incident-response-process)
- [Common Incident Scenarios](#common-incident-scenarios)
- [Communication Templates](#communication-templates)
- [Post-Incident Review](#post-incident-review)
- [Escalation Procedures](#escalation-procedures)

---

## Incident Classification

### Severity Levels

| Severity | Definition | Response Time | Examples |
|----------|------------|---------------|----------|
| **SEV-1** | Critical - Service completely unavailable | 15 minutes | Total API outage, data breach, data loss |
| **SEV-2** | Major - Significant functionality impaired | 30 minutes | >10% error rate, major feature broken, severe performance degradation |
| **SEV-3** | Minor - Limited impact on functionality | 2 hours | Single endpoint issues, minor performance degradation |
| **SEV-4** | Low - Minimal impact | 24 hours | Cosmetic issues, non-critical bugs |

### Impact Assessment Matrix

| Component | Users Affected | Business Impact | Severity |
|-----------|---------------|-----------------|----------|
| API Gateway | All | Critical | SEV-1 |
| Database (Primary) | All | Critical | SEV-1 |
| Authentication | All | Critical | SEV-1 |
| Experiment Execution | Active experiments | High | SEV-2 |
| Dataset Storage | Upload/Download users | Medium | SEV-2/3 |
| Metrics Collection | Monitoring | Low | SEV-3 |
| Documentation | None | None | SEV-4 |

---

## Incident Response Process

### Phase 1: Detection & Triage (0-15 minutes)

#### Step 1: Acknowledge Alert
```bash
# Acknowledge in PagerDuty/Opsgenie
# This prevents escalation and notifies team

# Quick status check
kubectl get pods -n llm-research -l app=llm-research-api
kubectl top pods -n llm-research
```

#### Step 2: Initial Assessment
```bash
#!/bin/bash
# incident-assessment.sh

echo "=== INCIDENT ASSESSMENT ==="
echo "Timestamp: $(date -u +%Y-%m-%dT%H:%M:%SZ)"
echo ""

# Check API health
echo "--- API Health ---"
curl -s -w "\nHTTP Status: %{http_code}\nResponse Time: %{time_total}s\n" \
  https://api.llm-research-lab.io/health

# Check error rates
echo "--- Error Rate (last 5m) ---"
curl -s "http://prometheus:9090/api/v1/query?query=rate(http_requests_total{status=~\"5..\"}[5m])" \
  | jq '.data.result[] | {endpoint: .metric.path, rate: .value[1]}'

# Check latency
echo "--- P95 Latency (last 5m) ---"
curl -s "http://prometheus:9090/api/v1/query?query=histogram_quantile(0.95,rate(http_request_duration_seconds_bucket[5m]))" \
  | jq '.data.result[] | {endpoint: .metric.path, latency: .value[1]}'

# Check pod status
echo "--- Pod Status ---"
kubectl get pods -n llm-research -l app=llm-research-api -o wide

# Check recent events
echo "--- Recent Events ---"
kubectl get events -n llm-research --sort-by=.lastTimestamp | tail -20

# Check database connections
echo "--- Database Connections ---"
curl -s http://localhost:9090/metrics | grep -E "^db_pool_connections"
```

#### Step 3: Declare Incident
```bash
# Create incident channel (Slack)
# Naming: #inc-YYYYMMDD-short-description
# Example: #inc-20250115-api-high-latency

# Post initial incident message (use template below)
```

### Phase 2: Investigation & Mitigation (15-60 minutes)

#### Step 4: Assign Roles

| Role | Responsibilities |
|------|------------------|
| **Incident Commander (IC)** | Coordinates response, makes decisions, manages communication |
| **Technical Lead** | Investigates root cause, implements fixes |
| **Communications Lead** | Updates stakeholders, manages status page |
| **Scribe** | Documents timeline, actions, and decisions |

#### Step 5: Gather Evidence
```bash
# Collect logs
kubectl logs -n llm-research -l app=llm-research-api --since=30m > /tmp/incident-logs.txt

# Collect metrics snapshots
curl -s "http://prometheus:9090/api/v1/query_range?query=http_requests_total&start=$(date -d '30 minutes ago' +%s)&end=$(date +%s)&step=60" \
  > /tmp/incident-metrics.json

# Save pod descriptions
kubectl describe pods -n llm-research -l app=llm-research-api > /tmp/incident-pods.txt

# Check recent deployments
kubectl rollout history deployment/llm-research-api -n llm-research
```

#### Step 6: Implement Mitigation
```bash
# Common mitigation actions:

# 1. Rollback if recent deployment
kubectl rollout undo deployment/llm-research-api -n llm-research

# 2. Scale up if capacity issue
kubectl scale deployment llm-research-api -n llm-research --replicas=10

# 3. Enable circuit breaker if external dependency
curl -X POST http://localhost:8080/admin/circuit-breaker/open \
  -H "X-Admin-Key: ${ADMIN_KEY}" \
  -d '{"service": "affected-service"}'

# 4. Restart pods if memory/state issues
kubectl rollout restart deployment/llm-research-api -n llm-research

# 5. Failover database if DB issue
# (Requires DBA involvement for RDS failover)
```

### Phase 3: Resolution & Recovery (60+ minutes)

#### Step 7: Verify Resolution
```bash
# Run full health check
./health-check.sh

# Monitor error rates for 15 minutes
watch -n 30 'curl -s "http://prometheus:9090/api/v1/query?query=rate(http_requests_total{status=~\"5..\"}[1m])"'

# Verify customer-facing functionality
./smoke-tests.sh
```

#### Step 8: Stand Down
```bash
# Update status page to "Operational"
# Post resolution message to incident channel
# Begin post-incident documentation
```

---

## Common Incident Scenarios

### Scenario 1: High Error Rate (>5% 5xx errors)

**Symptoms:**
- Spike in 5xx error alerts
- Users reporting failed requests
- Error rate dashboard shows elevated levels

**Investigation:**
```bash
# Check which endpoints are failing
curl -s "http://prometheus:9090/api/v1/query?query=topk(10,rate(http_requests_total{status=~\"5..\"}[5m]))" \
  | jq '.data.result[] | {endpoint: .metric.path, rate: .value[1]}'

# Check error logs
kubectl logs -n llm-research -l app=llm-research-api --since=10m | grep -i "error\|panic\|fatal"

# Check if recent deployment
kubectl rollout history deployment/llm-research-api -n llm-research

# Check database health
psql $DATABASE_URL -c "SELECT state, count(*) FROM pg_stat_activity GROUP BY state;"
```

**Mitigation:**
1. If recent deployment → Rollback
2. If database overload → Scale connection pool, investigate slow queries
3. If external service failure → Enable circuit breaker
4. If code bug → Deploy hotfix or rollback

---

### Scenario 2: High Latency (P95 > 2s)

**Symptoms:**
- Slow API responses
- Timeout errors
- User complaints about performance

**Investigation:**
```bash
# Identify slow endpoints
curl -s "http://prometheus:9090/api/v1/query?query=topk(10,histogram_quantile(0.95,rate(http_request_duration_seconds_bucket[5m])))" \
  | jq '.data.result[] | {endpoint: .metric.path, latency_p95: .value[1]}'

# Check database query performance
psql $DATABASE_URL <<EOF
SELECT pid, now() - pg_stat_activity.query_start AS duration, query
FROM pg_stat_activity
WHERE state = 'active'
ORDER BY duration DESC
LIMIT 10;
EOF

# Check pod resource usage
kubectl top pods -n llm-research -l app=llm-research-api

# Check connection pool saturation
curl -s http://localhost:9090/metrics | grep db_pool
```

**Mitigation:**
1. Scale horizontally if CPU bound
2. Increase connection pool if DB bound
3. Kill long-running queries if blocking
4. Enable request queuing/load shedding
5. Cache frequently accessed data

---

### Scenario 3: Database Connection Exhaustion

**Symptoms:**
- "too many connections" errors
- Connection timeout errors
- Slow or failed API requests

**Investigation:**
```bash
# Check current connections
psql $DATABASE_URL -c "SELECT count(*), state FROM pg_stat_activity GROUP BY state;"

# Check connection pool metrics
curl -s http://localhost:9090/metrics | grep -E "db_pool_(connections|waiters)"

# Check for connection leaks
psql $DATABASE_URL <<EOF
SELECT client_addr, count(*)
FROM pg_stat_activity
WHERE state != 'idle'
GROUP BY client_addr
ORDER BY count DESC;
EOF
```

**Mitigation:**
1. Terminate idle connections
```sql
SELECT pg_terminate_backend(pid)
FROM pg_stat_activity
WHERE state = 'idle'
  AND query_start < now() - interval '10 minutes';
```

2. Scale down pod count temporarily to reduce connection pressure
3. Increase RDS max_connections if available headroom
4. Deploy fix for connection leak if identified

---

### Scenario 4: Pod Crash Loop

**Symptoms:**
- CrashLoopBackOff status in kubectl
- Frequent pod restarts
- Service degradation or outage

**Investigation:**
```bash
# Check pod status and restart count
kubectl get pods -n llm-research -l app=llm-research-api

# Get crash reason
kubectl describe pod <pod-name> -n llm-research | grep -A 10 "Last State"

# Check recent logs before crash
kubectl logs <pod-name> -n llm-research --previous

# Check resource limits
kubectl describe pod <pod-name> -n llm-research | grep -A 5 "Limits"

# Check node health
kubectl describe node <node-name>
```

**Mitigation:**
1. If OOMKilled → Increase memory limits
2. If startup failure → Check configs, secrets, dependencies
3. If panic/crash → Rollback to previous version
4. If node issue → Drain and replace node

---

### Scenario 5: S3/Storage Issues

**Symptoms:**
- Dataset upload/download failures
- S3 timeout errors
- Storage quota exceeded

**Investigation:**
```bash
# Check S3 bucket status
aws s3 ls s3://${S3_BUCKET}/ --summarize

# Check S3 metrics
aws cloudwatch get-metric-statistics \
  --namespace AWS/S3 \
  --metric-name 4xxErrors \
  --dimensions Name=BucketName,Value=${S3_BUCKET} \
  --start-time $(date -d '1 hour ago' --iso-8601=seconds) \
  --end-time $(date --iso-8601=seconds) \
  --period 300 \
  --statistics Sum

# Check application S3 errors
kubectl logs -n llm-research -l app=llm-research-api --since=30m | grep -i "s3\|aws"
```

**Mitigation:**
1. If quota exceeded → Cleanup old data or increase quota
2. If network issue → Check VPC endpoints, security groups
3. If permissions → Verify IAM roles
4. If AWS outage → Enable degraded mode, notify users

---

### Scenario 6: Authentication/Authorization Failures

**Symptoms:**
- 401/403 errors spike
- Users unable to login
- API key validation failures

**Investigation:**
```bash
# Check auth error patterns
kubectl logs -n llm-research -l app=llm-research-api --since=30m | grep -E "401|403|auth|jwt"

# Verify JWT secret availability
kubectl get secret llm-research-secrets -n llm-research -o jsonpath='{.data.JWT_SECRET}' | base64 -d | head -c 10

# Check token validation
curl -v -H "Authorization: Bearer ${TEST_TOKEN}" https://api.llm-research-lab.io/experiments

# Check API key service
curl -s https://api.llm-research-lab.io/health/ready | jq '.components.auth'
```

**Mitigation:**
1. If secret rotation issue → Verify and fix secrets
2. If clock skew → Check NTP sync on pods
3. If code bug → Rollback
4. If DDoS/brute force → Enable additional rate limiting

---

## Communication Templates

### Initial Incident Declaration

```markdown
**INCIDENT DECLARED**

**Severity:** SEV-[1/2/3]
**Time Detected:** [HH:MM UTC]
**Impact:** [Brief description of user impact]

**Current Status:**
- [What we know so far]
- [What we're investigating]

**Roles Assigned:**
- IC: @[name]
- Technical Lead: @[name]
- Comms: @[name]

**Next Update:** [Time - typically 15-30 min]

**Incident Channel:** #inc-[YYYYMMDD]-[description]
```

### Status Update Template

```markdown
**INCIDENT UPDATE** - [HH:MM UTC]

**Status:** [Investigating/Identified/Mitigating/Monitoring/Resolved]

**What we know:**
- [Finding 1]
- [Finding 2]

**Current actions:**
- [Action being taken]

**Estimated resolution:** [Time estimate or "Unknown"]

**Next update:** [Time]
```

### Resolution Communication

```markdown
**INCIDENT RESOLVED**

**Duration:** [Start time] - [End time] ([X hours Y minutes])
**Impact:** [Description of what was affected]

**Root Cause:** [Brief explanation]

**Resolution:** [What fixed it]

**Follow-up Actions:**
- [ ] Post-incident review scheduled for [date]
- [ ] [Other action items]

Thank you for your patience during this incident.
```

### Customer Communication (Status Page)

```markdown
**[INVESTIGATING/IDENTIFIED/MONITORING/RESOLVED]**

**Posted:** [Timestamp]

We are currently experiencing [elevated error rates/degraded performance/partial outage]
affecting [description of affected services].

Our engineering team is actively investigating and working to resolve this issue.

We will provide updates every [15/30] minutes until resolved.

**Affected Services:**
- [Service 1]
- [Service 2]

**Current Workarounds:**
- [If any]
```

---

## Post-Incident Review

### Timeline Template

| Time (UTC) | Event | Actor |
|------------|-------|-------|
| HH:MM | Alert triggered | Monitoring |
| HH:MM | IC assigned | @name |
| HH:MM | Investigation started | @name |
| HH:MM | Root cause identified | @name |
| HH:MM | Mitigation deployed | @name |
| HH:MM | Service restored | System |
| HH:MM | Incident resolved | @name |

### Post-Incident Review Questions

1. **Detection**
   - How was the incident detected?
   - Could we have detected it earlier?
   - Were the right people alerted?

2. **Response**
   - Was the response time acceptable?
   - Were runbooks helpful and accurate?
   - What blocked faster resolution?

3. **Root Cause**
   - What was the root cause?
   - How did this slip through our processes?
   - Were there warning signs we missed?

4. **Prevention**
   - How can we prevent recurrence?
   - What monitoring improvements are needed?
   - What process changes should we make?

### Action Items Template

| Priority | Action | Owner | Due Date | Status |
|----------|--------|-------|----------|--------|
| P0 | [Critical fix] | @name | [date] | [ ] |
| P1 | [Important improvement] | @name | [date] | [ ] |
| P2 | [Nice to have] | @name | [date] | [ ] |

---

## Escalation Procedures

### Escalation Matrix

| Condition | Escalate To | Contact Method |
|-----------|-------------|----------------|
| SEV-1 not mitigated in 15 min | Engineering Manager | Phone |
| SEV-1 not resolved in 1 hour | VP Engineering | Phone |
| Data breach suspected | Security Team + Legal | Phone + Email |
| Customer SLA breach | Customer Success | Slack |
| Infrastructure failure | Cloud Provider Support | Support Ticket |

### On-Call Rotation

```
Week 1: Primary - @engineer1, Secondary - @engineer2
Week 2: Primary - @engineer2, Secondary - @engineer3
Week 3: Primary - @engineer3, Secondary - @engineer1
```

### External Vendor Contacts

| Vendor | Service | Support Contact | SLA |
|--------|---------|-----------------|-----|
| AWS | Infrastructure | AWS Support Portal | Business/Enterprise |
| Datadog | Monitoring | support@datadog.com | Pro SLA |
| PagerDuty | Alerting | support@pagerduty.com | Enterprise |

---

## Appendix

### Incident Response Checklist

**Detection:**
- [ ] Alert acknowledged
- [ ] Initial assessment completed
- [ ] Severity determined

**Response:**
- [ ] Incident channel created
- [ ] Roles assigned
- [ ] Status page updated
- [ ] Stakeholders notified

**Investigation:**
- [ ] Logs collected
- [ ] Metrics captured
- [ ] Root cause identified

**Mitigation:**
- [ ] Fix implemented
- [ ] Health verified
- [ ] Users notified of resolution

**Follow-up:**
- [ ] Timeline documented
- [ ] Post-incident review scheduled
- [ ] Action items created

### Quick Reference Commands

```bash
# Get pod status
kubectl get pods -n llm-research

# View logs
kubectl logs -f -n llm-research -l app=llm-research-api

# Rollback
kubectl rollout undo deployment/llm-research-api -n llm-research

# Scale
kubectl scale deployment/llm-research-api -n llm-research --replicas=N

# Port forward
kubectl port-forward -n llm-research svc/llm-research-api 8080:8080

# Exec into pod
kubectl exec -it <pod-name> -n llm-research -- /bin/sh
```
