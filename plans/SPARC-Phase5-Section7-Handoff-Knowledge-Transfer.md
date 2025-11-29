# SPARC Phase 5: Completion - Section 7: Handoff & Knowledge Transfer

> **LLM-Research-Lab**
> Operations Handoff & Knowledge Transfer Specification
> Target: Smooth transition to operational teams with comprehensive documentation

---

## Table of Contents

1. [Overview](#overview)
2. [7.1 Operations Training Program](#71-operations-training-program)
3. [7.2 Documentation Package](#72-documentation-package)
4. [7.3 Knowledge Transfer Sessions](#73-knowledge-transfer-sessions)
5. [7.4 Handoff Checklist](#74-handoff-checklist)
6. [7.5 Support Transition](#75-support-transition)
7. [Appendix A: Training Templates](#appendix-a-training-templates)
8. [Appendix B: Certification Criteria](#appendix-b-certification-criteria)

---

## Overview

This section defines the comprehensive handoff and knowledge transfer process for transitioning LLM-Research-Lab from development to operations. The goal is to ensure operations teams have complete understanding, documentation, and capabilities to maintain 99.9% SLA.

### Transfer Objectives

1. **Complete Knowledge Transfer**: Operations team fully understands system architecture, dependencies, and operational procedures
2. **Documentation Completeness**: All runbooks, architecture docs, and troubleshooting guides are production-ready
3. **Operational Readiness**: Support team certified and equipped to handle incidents independently
4. **Continuous Support**: Seamless transition with shadow period and escalation paths established

### Success Criteria

| Metric | Target | Validation Method |
|--------|--------|------------------|
| **Training Completion Rate** | 100% | All operations staff complete certification |
| **Documentation Coverage** | 100% | All components have runbooks and architecture docs |
| **Incident Response Time** | <15 min | Shadow on-call successfully handles test incidents |
| **Handoff Approval** | Sign-off | Operations manager approves readiness |

---

## 7.1 Operations Training Program

### 7.1.1 Training Curriculum Outline

#### Module 1: System Architecture Overview (4 hours)

**Learning Objectives:**
- Understand high-level architecture of LLM-Research-Lab
- Identify core components and their interactions
- Map data flows between services
- Recognize integration points with LLM DevOps ecosystem

**Topics:**
1. **System Architecture** (60 min)
   - Component overview: API, Experiment Runner, Workers, Storage
   - Database architecture: PostgreSQL, ClickHouse, Redis
   - Message queue architecture: Kafka topics and consumer groups
   - Integration: LLM-Registry, LLM-Data-Vault, LLM-Analytics-Hub, LLM-Test-Bench

2. **Infrastructure Components** (60 min)
   - Kubernetes cluster architecture and namespaces
   - Load balancing and ingress configuration
   - Storage systems: S3-compatible object storage, persistent volumes
   - Network policies and security boundaries

3. **Data Flow Patterns** (60 min)
   - Experiment submission workflow
   - Dataset versioning and retrieval
   - Metrics collection and aggregation
   - Result storage and retrieval

4. **Hands-On Lab** (60 min)
   - Navigate Kubernetes cluster
   - Inspect running services
   - Query databases
   - Review Kafka topics

**Deliverables:**
- Architecture diagram walkthrough
- Component interaction mapping exercise
- Quiz: 20 questions (80% pass rate required)

---

#### Module 2: Operational Procedures (6 hours)

**Learning Objectives:**
- Execute standard operational tasks
- Follow runbook procedures accurately
- Use monitoring and observability tools
- Perform basic troubleshooting

**Topics:**
1. **Monitoring and Observability** (90 min)
   - Grafana dashboard navigation
   - Prometheus query basics (PromQL)
   - Log aggregation with Loki
   - Distributed tracing with Jaeger
   - Alert interpretation and triage

2. **Routine Operations** (90 min)
   - Service health checks
   - Database maintenance tasks
   - Cache management (Redis)
   - Kafka topic management
   - Backup verification

3. **Deployment Procedures** (90 min)
   - Rolling updates and canary deployments
   - Database migration execution
   - Configuration changes
   - Rollback procedures

4. **Hands-On Lab** (90 min)
   - Execute health check runbook
   - Perform simulated deployment
   - Investigate mock alert in Grafana
   - Execute rollback procedure

**Deliverables:**
- Runbook execution checklist
- Monitoring dashboard customization
- Practical exam: Complete 5 operational tasks

---

#### Module 3: Incident Response (4 hours)

**Learning Objectives:**
- Respond to production incidents effectively
- Use troubleshooting guides to diagnose issues
- Execute escalation procedures appropriately
- Document incidents for post-mortem analysis

**Topics:**
1. **Incident Response Framework** (60 min)
   - Incident severity classification (P0-P4)
   - Alert triage and prioritization
   - Communication protocols (Slack, PagerDuty, status page)
   - Incident command structure

2. **Common Failure Scenarios** (90 min)
   - Database connection pool exhaustion
   - Kafka consumer lag buildup
   - API latency spikes
   - Out of memory errors
   - Disk space exhaustion
   - Network connectivity issues

3. **Troubleshooting Methodology** (60 min)
   - Structured problem-solving approach
   - Log analysis techniques
   - Metrics correlation
   - Root cause analysis

4. **Tabletop Exercise** (30 min)
   - Simulated P1 incident: API latency spike
   - Team response and communication
   - Runbook execution
   - Post-incident review

**Deliverables:**
- Incident response checklist
- Troubleshooting decision tree
- Tabletop exercise report

---

#### Module 4: Advanced Topics (4 hours)

**Learning Objectives:**
- Perform capacity planning analysis
- Execute disaster recovery procedures
- Implement security best practices
- Optimize system performance

**Topics:**
1. **Capacity Management** (60 min)
   - Resource utilization monitoring
   - Forecasting and trending
   - Scaling strategies (horizontal vs vertical)
   - Cost optimization techniques

2. **Disaster Recovery** (90 min)
   - Backup and restore procedures
   - Failover mechanisms
   - RTO/RPO requirements
   - DR testing protocols

3. **Security Operations** (60 min)
   - Access control and RBAC
   - Secret management (Kubernetes secrets, external secret stores)
   - Security monitoring and audit logs
   - Vulnerability management

4. **Performance Tuning** (30 min)
   - Database query optimization
   - Cache tuning
   - Connection pool configuration
   - Resource allocation best practices

**Deliverables:**
- Capacity planning worksheet
- DR drill execution report
- Security audit checklist

---

### 7.1.2 Hands-On Exercises

#### Exercise 1: Service Health Check

**Objective:** Verify all LLM-Research-Lab services are healthy

**Steps:**
1. Access Kubernetes cluster: `kubectl config use-context llm-research-lab-prod`
2. Check pod status:
   ```bash
   kubectl get pods -n llm-research-lab-prod
   kubectl get pods -n llm-research-lab-data
   ```
3. Verify API health endpoint:
   ```bash
   curl https://api.llm-research-lab.company.com/health/ready
   ```
4. Check database connectivity:
   ```bash
   kubectl exec -n llm-research-lab-data -it postgres-0 -- psql -U postgres -c "SELECT 1"
   ```
5. Verify Kafka broker health:
   ```bash
   kubectl exec -n llm-research-lab-prod kafka-0 -- kafka-broker-api-versions.sh --bootstrap-server localhost:9092
   ```

**Expected Results:**
- All pods in `Running` state
- API returns `200 OK` with `{"status": "healthy"}`
- Database query returns `1`
- Kafka brokers respond with version information

**Validation:** Instructor reviews executed commands and verifies outputs

---

#### Exercise 2: Investigate API Latency Alert

**Objective:** Diagnose and resolve simulated API latency spike

**Scenario:** PagerDuty alert: `API p99 latency > 500ms for 5 minutes`

**Steps:**
1. Access Grafana dashboard: "LLM Research Lab - API Performance"
2. Identify latency spike timestamp
3. Check concurrent requests:
   ```promql
   api_request_in_flight{method="POST", path="/api/v1/experiments"}
   ```
4. Review error rates:
   ```promql
   rate(api_request_total{status=~"5.."}[5m])
   ```
5. Analyze database query performance:
   ```promql
   db:query_latency:p99:5m{query_type="INSERT"}
   ```
6. Check logs for errors:
   ```bash
   kubectl logs -n llm-research-lab-prod -l app=llm-research-api --tail=100 | grep ERROR
   ```
7. Correlate with distributed trace in Jaeger

**Expected Actions:**
- Identify root cause (e.g., database slow query)
- Execute remediation (e.g., kill long-running query, scale up pods)
- Document findings in incident report template

**Validation:** Trainee completes incident report with correct root cause analysis

---

#### Exercise 3: Execute Database Backup Restore

**Objective:** Restore PostgreSQL database from backup

**Steps:**
1. List available backups:
   ```bash
   kubectl exec -n llm-research-lab-data postgres-0 -- barman list-backup llm-research-postgres
   ```
2. Create test database for restore:
   ```bash
   kubectl exec -n llm-research-lab-data postgres-0 -- psql -U postgres -c "CREATE DATABASE restore_test"
   ```
3. Execute restore:
   ```bash
   kubectl exec -n llm-research-lab-data postgres-0 -- barman recover \
     --target-time "2025-11-28 10:00:00" \
     llm-research-postgres latest restore_test
   ```
4. Verify restore:
   ```bash
   kubectl exec -n llm-research-lab-data postgres-0 -- psql -U postgres -d restore_test -c "\dt"
   ```
5. Document restore completion time and verification steps

**Expected Results:**
- Restore completes successfully within 30 minutes
- All tables present in restored database
- Sample data queries return expected results

**Validation:** Instructor verifies restored database schema and data integrity

---

#### Exercise 4: Scale Experiment Workers

**Objective:** Horizontally scale experiment workers based on Kafka lag

**Steps:**
1. Check current worker count:
   ```bash
   kubectl get deployment experiment-worker -n llm-research-lab-prod
   ```
2. Monitor Kafka consumer lag:
   ```promql
   kafka_consumergroup_lag{topic="experiments",group="experiment-processor-v1"}
   ```
3. Calculate required worker count (target: 100 messages lag per worker)
4. Scale deployment:
   ```bash
   kubectl scale deployment experiment-worker -n llm-research-lab-prod --replicas=10
   ```
5. Verify scaling:
   ```bash
   kubectl rollout status deployment/experiment-worker -n llm-research-lab-prod
   ```
6. Monitor lag reduction in Grafana

**Expected Results:**
- Workers scale to target count within 2 minutes
- Kafka consumer lag decreases to target threshold
- No errors during scaling operation

**Validation:** Lag metric returns to healthy levels (< 1000 messages total)

---

### 7.1.3 Certification Requirements

#### Certification Tracks

**Track 1: L1 Support Operator**

**Prerequisites:**
- Basic Linux command-line proficiency
- Understanding of HTTP/REST APIs
- Familiarity with Kubernetes concepts

**Requirements:**
- Complete Modules 1-2 (10 hours)
- Pass written exam: 30 questions, 85% score required
- Complete 3 hands-on exercises successfully
- Shadow L2 engineer for 5 incident responses

**Certification Grants:**
- Read-only access to production cluster
- Execute pre-approved runbooks
- Escalate to L2 with context
- Update status page and incident tickets

---

**Track 2: L2 Support Engineer**

**Prerequisites:**
- L1 certification OR 2+ years operations experience
- Proficiency in Kubernetes, PostgreSQL, Kafka
- Programming knowledge (Rust/Python preferred)

**Requirements:**
- Complete all modules (18 hours)
- Pass written exam: 50 questions, 90% score required
- Complete all 4 hands-on exercises successfully
- Lead 3 incident responses during shadow period
- Execute 1 production deployment under supervision
- Complete DR drill successfully

**Certification Grants:**
- Full read access to production cluster
- Execute all runbooks
- Approve emergency changes
- Modify non-critical configurations
- Lead incident response

---

**Track 3: L3 SRE Specialist**

**Prerequisites:**
- L2 certification
- 3+ years SRE/DevOps experience
- Deep Rust and infrastructure-as-code knowledge

**Requirements:**
- Complete advanced training (additional 8 hours on architecture deep-dive)
- Review all source code in llm-research-lab repository
- Design and document 2 new runbooks
- Lead post-mortem for 3 incidents
- Mentor 2 L1/L2 trainees
- Pass architecture review interview with development team

**Certification Grants:**
- Full read-write access to production cluster
- Approve architectural changes
- Modify critical system configurations
- Create new deployment procedures
- Incident commander for P0/P1 incidents

---

### 7.1.4 Training Schedule

#### Week 1-2: Initial Training

| Day | Time | Module | Audience | Instructor |
|-----|------|--------|----------|-----------|
| Mon W1 | 9:00-13:00 | Module 1: Architecture | All | Lead Architect |
| Tue W1 | 9:00-12:00 | Module 2: Operations (Part 1) | All | Senior SRE |
| Wed W1 | 9:00-12:00 | Module 2: Operations (Part 2) | All | Senior SRE |
| Thu W1 | 9:00-13:00 | Module 3: Incident Response | All | Incident Commander |
| Fri W1 | 9:00-13:00 | Hands-On Lab Day | All | Ops Team Lead |
| Mon W2 | 9:00-13:00 | Module 4: Advanced Topics | L2/L3 | Principal Engineer |
| Tue W2 | 9:00-17:00 | Certification Exams | All | Training Coordinator |
| Wed W2 | 9:00-17:00 | Exam Review & Remediation | Failed Exams | Instructors |
| Thu W2 | 9:00-17:00 | Shadow Period Begins | All | Current On-Call |
| Fri W2 | 14:00-16:00 | Training Retrospective | All | Training Lead |

#### Week 3-4: Shadow On-Call Period

- **Weeks 3-4**: All certified operators shadow current on-call rotation
- **Daily**: 30-minute debrief with shadow mentor
- **Week 4 Friday**: Shadow period completion assessment

---

## 7.2 Documentation Package

### 7.2.1 System Architecture Documentation

#### High-Level Architecture Document

**File:** `/docs/architecture/overview.md`

**Contents:**
```markdown
# LLM-Research-Lab System Architecture

## Executive Summary
LLM-Research-Lab is a distributed system for LLM experimentation and evaluation,
comprising API services, asynchronous experiment workers, and data storage layers.

## Component Diagram
[Include C4 Level 1 Context Diagram]

## Core Components

### API Layer
- **Service:** llm-research-api
- **Technology:** Rust (Axum framework)
- **Responsibilities:**
  - Experiment submission and management
  - Dataset versioning API
  - Metrics query endpoints
  - Authentication and authorization
- **Scaling:** Horizontal (3-20 replicas, HPA-managed)
- **Dependencies:** PostgreSQL (metadata), Redis (caching), Kafka (async processing)

### Experiment Runner
- **Service:** experiment-worker
- **Technology:** Rust (Tokio async runtime)
- **Responsibilities:**
  - Consume experiment jobs from Kafka
  - Execute model inference against LLM providers
  - Collect evaluation metrics
  - Store results in ClickHouse and PostgreSQL
- **Scaling:** Horizontal (2-50 replicas based on Kafka lag)
- **Dependencies:** Kafka, ClickHouse, S3-compatible storage, LLM-Registry

### Data Layer
- **PostgreSQL:** Relational metadata (experiments, datasets, users)
- **ClickHouse:** Time-series metrics and analytics
- **Redis:** Session cache, rate limiting, distributed locks
- **Kafka:** Asynchronous job queue (experiments, results, metrics)
- **S3-Compatible Storage:** Model artifacts, datasets, experiment outputs

### Integration Points
- **LLM-Registry:** Fetch model metadata and versioned artifacts
- **LLM-Data-Vault:** Access controlled datasets with lineage tracking
- **LLM-Analytics-Hub:** Stream metrics for visualization
- **LLM-Test-Bench:** Submit models for standardized benchmarking

## Network Architecture
[Include network diagram showing ingress, service mesh, egress]

## Data Flow
[Include sequence diagrams for key workflows: experiment submission, result retrieval]

## Security Boundaries
[Include trust boundaries and authentication/authorization flows]
```

---

#### Component Deep-Dive Documents

**File:** `/docs/architecture/components/api-service.md`

**Template:**
```markdown
# API Service Architecture

## Overview
The LLM-Research-Lab API provides RESTful endpoints for experiment management.

## Technology Stack
- **Framework:** Axum (Rust async web framework)
- **Database ORM:** SQLx with compile-time query verification
- **Authentication:** JWT with RS256 signing
- **Serialization:** serde_json
- **Metrics:** Prometheus client library

## API Endpoints

### Experiments API
- `POST /api/v1/experiments` - Submit new experiment
- `GET /api/v1/experiments/{id}` - Retrieve experiment details
- `GET /api/v1/experiments` - List experiments (paginated)
- `DELETE /api/v1/experiments/{id}` - Cancel running experiment

### Datasets API
- `POST /api/v1/datasets` - Upload and version dataset
- `GET /api/v1/datasets/{name}/{version}` - Retrieve dataset
- `GET /api/v1/datasets` - List all dataset versions

### Metrics API
- `GET /api/v1/metrics/experiments/{id}` - Fetch experiment metrics
- `POST /api/v1/metrics/query` - Execute PromQL-style metric queries

## Configuration
Environment variables and config file locations documented in `/docs/configuration/api.md`

## Observability
- **Metrics:** Exposed on `:9090/metrics`
- **Logs:** Structured JSON to stdout (collected by Vector)
- **Traces:** OpenTelemetry exporter to Jaeger

## Scaling Characteristics
- Stateless design enables horizontal scaling
- Connection pooling: 25 connections per replica
- Target: 1000 req/s per replica at <50ms p99 latency

## Failure Modes
| Failure | Detection | Impact | Mitigation |
|---------|-----------|--------|------------|
| Database unavailable | Health check fails | API returns 503 | Circuit breaker, retry with backoff |
| Redis cache miss | Log warning | Slight latency increase | Database fallback |
| Kafka producer timeout | Metric alert | Experiment submission fails | Client retry, DLQ |
```

Repeat for:
- `/docs/architecture/components/experiment-worker.md`
- `/docs/architecture/components/database-layer.md`
- `/docs/architecture/components/messaging-layer.md`

---

### 7.2.2 API Documentation (Rustdoc + OpenAPI)

#### Rustdoc Generation

**Build Command:**
```bash
cargo doc --no-deps --document-private-items --open
```

**CI Pipeline Integration:**
```yaml
# .github/workflows/docs.yml
name: Generate Documentation
on:
  push:
    branches: [main]
jobs:
  docs:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: actions-rust-lang/setup-rust-toolchain@v1
      - name: Generate Rustdoc
        run: cargo doc --no-deps --workspace
      - name: Deploy to GitHub Pages
        uses: peaceiris/actions-gh-pages@v3
        with:
          github_token: ${{ secrets.GITHUB_TOKEN }}
          publish_dir: ./target/doc
```

**Access:** https://company.github.io/llm-research-lab/

---

#### OpenAPI Specification

**File:** `/docs/api/openapi.yaml`

**Generation:** Use `utoipa` crate for Rust code annotation

```rust
// Example from src/api/experiments.rs
use utoipa::{OpenApi, ToSchema};

#[derive(OpenApi)]
#[openapi(
    paths(
        submit_experiment,
        get_experiment,
        list_experiments
    ),
    components(
        schemas(ExperimentRequest, ExperimentResponse, ExperimentStatus)
    ),
    tags(
        (name = "experiments", description = "Experiment management endpoints")
    )
)]
pub struct ExperimentsApi;

#[utoipa::path(
    post,
    path = "/api/v1/experiments",
    request_body = ExperimentRequest,
    responses(
        (status = 201, description = "Experiment created", body = ExperimentResponse),
        (status = 400, description = "Invalid request"),
        (status = 401, description = "Unauthorized")
    ),
    tag = "experiments"
)]
async fn submit_experiment(
    Json(req): Json<ExperimentRequest>
) -> Result<Json<ExperimentResponse>, ApiError> {
    // Implementation
}
```

**Interactive Documentation:** Deploy Swagger UI at `https://api.llm-research-lab.company.com/docs`

---

### 7.2.3 Operational Runbooks Index

All runbooks follow standardized template in `/docs/runbooks/template.md`

#### Runbook: Service Health Check

**File:** `/docs/runbooks/health-check.md`

**Metadata:**
```yaml
title: LLM-Research-Lab Service Health Check
category: Routine Maintenance
frequency: Daily
duration: 15 minutes
required_role: L1 Operator
last_updated: 2025-11-28
```

**Purpose:** Verify all LLM-Research-Lab services are operational

**Prerequisites:**
- Access to Kubernetes cluster
- Read access to Grafana dashboards

**Procedure:**
1. **Check Pod Health**
   ```bash
   kubectl get pods -n llm-research-lab-prod -o wide
   kubectl get pods -n llm-research-lab-data -o wide
   ```
   **Expected:** All pods in `Running` state, no restarts in last 24h

2. **Verify API Endpoints**
   ```bash
   curl -f https://api.llm-research-lab.company.com/health/live
   curl -f https://api.llm-research-lab.company.com/health/ready
   ```
   **Expected:** Both return `200 OK`

3. **Check Database Connectivity**
   ```bash
   kubectl exec -n llm-research-lab-data postgres-0 -- \
     psql -U postgres -d llm_research -c "SELECT COUNT(*) FROM experiments;"
   ```
   **Expected:** Query returns count without errors

4. **Verify Kafka Topics**
   ```bash
   kubectl exec -n llm-research-lab-prod kafka-0 -- \
     kafka-topics.sh --bootstrap-server localhost:9092 --list
   ```
   **Expected:** Topics `experiments`, `experiment-results`, `metrics-events` present

5. **Review Grafana Dashboard**
   - Navigate to "LLM Research Lab - System Overview"
   - Confirm no critical alerts firing
   - Verify request rate > 0 (indicates active usage)

**Success Criteria:**
- All checks pass
- No critical alerts
- Response times within SLO

**Failure Actions:**
- If any check fails, escalate to L2 with failure details
- Create incident ticket with severity based on number of failures

**Automation Opportunity:** Convert to automated health check script

---

#### Runbook: Database Connection Pool Exhaustion

**File:** `/docs/runbooks/db-connection-pool-exhaustion.md`

**Metadata:**
```yaml
title: Resolve Database Connection Pool Exhaustion
category: Incident Response
severity: P2
mttr_target: 30 minutes
required_role: L2 Engineer
```

**Symptoms:**
- Alert: `db_connection_pool_utilization > 0.95`
- API errors: `"database connection timeout"`
- Logs: `ERROR: connection pool exhausted`

**Investigation:**
1. **Confirm Pool Exhaustion**
   ```promql
   db_connection_pool_size - db_connection_pool_idle
   ```
   **Expected:** Value near `db_connection_pool_size` (default 25)

2. **Identify Long-Running Queries**
   ```sql
   SELECT pid, now() - query_start AS duration, query
   FROM pg_stat_activity
   WHERE state = 'active' AND now() - query_start > interval '5 minutes'
   ORDER BY duration DESC;
   ```

3. **Check for Connection Leaks**
   ```sql
   SELECT application_name, COUNT(*)
   FROM pg_stat_activity
   GROUP BY application_name
   ORDER BY COUNT(*) DESC;
   ```
   **Look for:** Unexpected high connection counts from specific apps

**Resolution:**
1. **Immediate Mitigation (choose one):**
   - **Option A:** Kill long-running queries (if identified as root cause)
     ```sql
     SELECT pg_terminate_backend(pid) FROM pg_stat_activity
     WHERE pid = <problematic_pid>;
     ```
   - **Option B:** Restart API pods to reset connections
     ```bash
     kubectl rollout restart deployment/llm-research-api -n llm-research-lab-prod
     ```
   - **Option C:** Temporarily increase pool size (emergency only)
     ```bash
     kubectl set env deployment/llm-research-api -n llm-research-lab-prod \
       DATABASE_POOL_SIZE=50
     ```

2. **Verify Resolution**
   ```promql
   db_connection_pool:utilization < 0.80
   ```

3. **Root Cause Analysis**
   - Review slow query logs
   - Check for recent code deployments
   - Analyze API request patterns (unexpected load spike?)

**Prevention:**
- Implement connection timeout configurations
- Add alerts for slow queries (> 10s)
- Review and optimize identified slow queries
- Consider increasing default pool size if sustained high usage

**Escalation:**
- If resolution fails after 30 minutes, escalate to L3 SRE
- If database performance degraded, engage DBA team

---

#### Additional Runbooks (Index)

| Runbook | File | Category | Severity | Role |
|---------|------|----------|----------|------|
| API Latency Spike | `api-latency-spike.md` | Incident Response | P2 | L2 |
| Kafka Consumer Lag | `kafka-consumer-lag.md` | Incident Response | P3 | L2 |
| Disk Space Exhaustion | `disk-space-exhaustion.md` | Incident Response | P1 | L2 |
| Database Backup/Restore | `database-backup-restore.md` | Maintenance | N/A | L2 |
| Redis Failover | `redis-failover.md` | Incident Response | P2 | L2 |
| Certificate Renewal | `certificate-renewal.md` | Maintenance | N/A | L1 |
| Deployment Rollback | `deployment-rollback.md` | Change Management | P1 | L2 |
| Scale Experiment Workers | `scale-workers.md` | Capacity | N/A | L1 |
| ClickHouse Query Optimization | `clickhouse-optimization.md` | Performance | P3 | L3 |
| Disaster Recovery Drill | `disaster-recovery-drill.md` | DR Testing | N/A | L3 |

**Access:** All runbooks in `/docs/runbooks/` with index at `/docs/runbooks/README.md`

---

### 7.2.4 Troubleshooting Guides

#### Troubleshooting Decision Tree

**File:** `/docs/troubleshooting/decision-tree.md`

```
Problem: User reports "Experiment submission failed"
├─ Is API returning 5xx errors?
│  ├─ Yes → Check API pod health
│  │  ├─ Pods CrashLooping → Review logs for panic/OOM
│  │  ├─ Pods Running but slow → Check database connection pool
│  │  └─ No pods available → Check HPA configuration
│  └─ No → Check API returning 4xx errors?
│     ├─ 401 Unauthorized → Verify JWT token validity
│     ├─ 400 Bad Request → Review request payload schema
│     └─ 429 Too Many Requests → Rate limit exceeded
│
├─ Is Kafka accepting messages?
│  ├─ No → Check Kafka broker health
│  │  └─ Follow runbook: kafka-broker-recovery.md
│  └─ Yes → Check consumer lag
│     └─ High lag → Scale experiment workers
│
└─ Is experiment stuck in "pending" state?
   ├─ Check worker pod count
   ├─ Review worker logs for errors
   └─ Verify dataset availability in LLM-Data-Vault
```

---

#### Common Error Messages Reference

**File:** `/docs/troubleshooting/error-messages.md`

| Error Message | Cause | Resolution |
|--------------|-------|------------|
| `database connection timeout` | Connection pool exhausted | Runbook: `db-connection-pool-exhaustion.md` |
| `Kafka producer timeout` | Kafka broker unavailable or slow | Check Kafka broker health, verify network connectivity |
| `Dataset version not found: {name}:{version}` | Dataset not in LLM-Data-Vault | Verify dataset upload, check Data-Vault availability |
| `Model not found in registry: {model_id}` | Model not registered in LLM-Registry | Register model or use existing model ID |
| `OOM killed` | Container memory exceeded | Increase pod memory limits, investigate memory leak |
| `too many open files` | File descriptor limit reached | Increase ulimit or investigate file descriptor leak |
| `certificate has expired` | TLS certificate expired | Runbook: `certificate-renewal.md` |
| `RBAC: access denied` | Insufficient Kubernetes permissions | Review ServiceAccount permissions, update RBAC |

---

## 7.3 Knowledge Transfer Sessions

### 7.3.1 Session Schedule and Topics

#### Week 1: Architecture and Design

**Session 1.1: System Overview and Design Principles**
- **Date:** Monday, Week 1 | 10:00-12:00
- **Presenter:** Lead Architect
- **Audience:** All operations staff (L1/L2/L3)
- **Agenda:**
  1. LLM-Research-Lab purpose and scope (30 min)
  2. High-level architecture walkthrough (45 min)
  3. Design principles and trade-offs (30 min)
  4. Q&A (15 min)
- **Materials:** Architecture diagrams, design decision log
- **Recording:** Required, post to internal wiki

**Session 1.2: Database Architecture**
- **Date:** Tuesday, Week 1 | 14:00-16:00
- **Presenter:** Database Engineer
- **Audience:** L2/L3 engineers
- **Agenda:**
  1. PostgreSQL schema design and indexing strategy (30 min)
  2. ClickHouse sharding and replication (30 min)
  3. Redis caching patterns (20 min)
  4. Connection pooling and performance tuning (25 min)
  5. Q&A (15 min)
- **Materials:** ER diagrams, sample queries, performance benchmarks
- **Recording:** Required

**Session 1.3: Messaging Architecture**
- **Date:** Wednesday, Week 1 | 10:00-12:00
- **Presenter:** Senior Backend Engineer
- **Audience:** L2/L3 engineers
- **Agenda:**
  1. Kafka topic design and partitioning (30 min)
  2. Producer/consumer patterns (30 min)
  3. Message schemas and versioning (20 min)
  4. Error handling and dead-letter queues (25 min)
  5. Q&A (15 min)
- **Materials:** Topic configurations, message schemas, consumer group configs
- **Recording:** Required

---

#### Week 2: Operations and Monitoring

**Session 2.1: Observability Stack Deep-Dive**
- **Date:** Monday, Week 2 | 10:00-12:00
- **Presenter:** SRE Lead
- **Audience:** All operations staff
- **Agenda:**
  1. Prometheus metrics and recording rules (30 min)
  2. Grafana dashboards and alerting (30 min)
  3. Loki log aggregation and querying (20 min)
  4. Jaeger distributed tracing (25 min)
  5. Hands-on: Create custom dashboard (15 min)
- **Materials:** Prometheus queries, dashboard JSON templates, log query examples
- **Recording:** Required

**Session 2.2: Incident Response Procedures**
- **Date:** Tuesday, Week 2 | 14:00-16:00
- **Presenter:** Incident Commander
- **Audience:** All operations staff
- **Agenda:**
  1. Incident severity classification (20 min)
  2. Communication protocols and escalation (25 min)
  3. Runbook walkthrough: Top 5 incidents (45 min)
  4. Post-mortem process (15 min)
  5. Q&A (15 min)
- **Materials:** Incident response playbook, sample post-mortems, escalation matrix
- **Recording:** Required

**Session 2.3: Deployment and Rollback Procedures**
- **Date:** Wednesday, Week 2 | 10:00-12:00
- **Presenter:** DevOps Engineer
- **Audience:** L2/L3 engineers
- **Agenda:**
  1. CI/CD pipeline overview (20 min)
  2. Deployment strategies (rolling, canary, blue-green) (30 min)
  3. Database migration workflow (25 min)
  4. Rollback procedures and automation (30 min)
  5. Live demo: Execute deployment (15 min)
- **Materials:** CI/CD configs, deployment checklists, rollback scripts
- **Recording:** Required

---

#### Week 3: Advanced Topics

**Session 3.1: Capacity Planning and Scaling**
- **Date:** Monday, Week 3 | 10:00-12:00
- **Presenter:** Principal SRE
- **Audience:** L2/L3 engineers
- **Agenda:**
  1. Resource utilization analysis (30 min)
  2. Forecasting and trend analysis (30 min)
  3. Autoscaling configurations (HPA, VPA, Cluster Autoscaler) (30 min)
  4. Cost optimization techniques (15 min)
  5. Q&A (15 min)
- **Materials:** Capacity planning spreadsheets, historical usage data, cost reports
- **Recording:** Required

**Session 3.2: Security and Compliance**
- **Date:** Tuesday, Week 3 | 14:00-16:00
- **Presenter:** Security Engineer
- **Audience:** L2/L3 engineers
- **Agenda:**
  1. RBAC and access control (25 min)
  2. Secret management best practices (25 min)
  3. Security monitoring and audit logs (30 min)
  4. Vulnerability management workflow (25 min)
  5. Q&A (15 min)
- **Materials:** RBAC policies, secret rotation procedures, security audit checklist
- **Recording:** Required

**Session 3.3: Disaster Recovery and Business Continuity**
- **Date:** Wednesday, Week 3 | 10:00-12:00
- **Presenter:** DR Lead
- **Audience:** L2/L3 engineers
- **Agenda:**
  1. RTO/RPO requirements and SLAs (20 min)
  2. Backup strategies and automation (30 min)
  3. Failover procedures (30 min)
  4. DR testing and validation (25 min)
  5. Q&A (15 min)
- **Materials:** DR plan, backup schedules, failover runbooks, test results
- **Recording:** Required

---

### 7.3.2 Recorded Session Requirements

#### Recording Standards

**Technical Requirements:**
- **Platform:** Zoom with cloud recording
- **Video Quality:** 1080p minimum
- **Audio Quality:** 48kHz, noise cancellation enabled
- **Screen Sharing:** Record presenter screen + webcam
- **Duration:** Full session including Q&A
- **Closed Captions:** Auto-generated, manually reviewed for accuracy

**Content Requirements:**
- **Intro Slide:** Session title, presenter, date, objectives
- **Agenda Slide:** Topics and time allocation
- **Demo/Hands-On:** Screen recording of all live demonstrations
- **Q&A:** All questions and answers captured
- **Resources Slide:** Links to documentation, runbooks, source code

**Post-Processing:**
- **Editing:** Remove long pauses, technical difficulties
- **Chapters:** Add video chapters for each agenda item
- **Transcript:** Generate full text transcript
- **Slides:** Export slides as PDF and attach to recording

**Publishing:**
- **Platform:** Internal video library (e.g., company LMS or wiki)
- **Access Control:** Restrict to operations and engineering teams
- **Metadata:** Tag with keywords, session number, presenter, date
- **Searchability:** Ensure transcript is indexed for full-text search

---

### 7.3.3 Q&A Documentation

#### Q&A Session Format

Each knowledge transfer session includes dedicated Q&A time. All questions and answers are documented in shared tracker.

**Q&A Tracker:** Google Sheet or Confluence page

**Template:**

| ID | Session | Question | Asked By | Answer | Answered By | Follow-Up Required | Status |
|----|---------|----------|----------|--------|-------------|-------------------|--------|
| 001 | 1.1 | What is the expected query load on PostgreSQL? | Jane (L2) | ~500 queries/sec during peak hours. See capacity plan doc. | Lead Architect | No | Closed |
| 002 | 2.1 | How do we customize Grafana dashboards? | Mike (L1) | Follow runbook: grafana-dashboard-customization.md | SRE Lead | Yes - Create runbook | Open |
| 003 | 2.2 | What is escalation path for P0 incidents? | Sarah (L2) | L1 → L2 → L3 → Dev Team Oncall. See escalation matrix. | Incident Commander | No | Closed |

**Process:**
1. **During Session:** Moderator captures questions in real-time
2. **After Session:** Presenter provides detailed written answers within 48 hours
3. **Follow-Up:** If question requires runbook creation or documentation update, create Jira ticket
4. **Review:** All Q&A reviewed in weekly handoff sync meeting

---

### 7.3.4 Shadow On-Call Period

#### Shadow On-Call Program Structure

**Duration:** 2 weeks (minimum) per operator

**Objectives:**
1. Observe real-world incident response
2. Practice runbook execution in live environment
3. Build confidence in escalation and communication
4. Validate training effectiveness

**Roles:**
- **Primary On-Call:** Experienced engineer (current rotation)
- **Shadow:** Operations trainee (L1 or L2 candidate)
- **Mentor:** Designated coach (usually Shadow's manager)

---

#### Shadow On-Call Schedule

**Week 1: Observation**

| Day | Activity | Primary Responsibility | Shadow Responsibility |
|-----|----------|----------------------|----------------------|
| Mon | Daily Health Check | Execute | Observe and document |
| Mon | Alert Triage | Lead response | Observe, take notes |
| Tue | Incident Response (if any) | Lead | Observe, assist with documentation |
| Wed | Deployment Review | Approve | Observe approval process |
| Thu | Capacity Review | Analyze metrics | Observe and ask questions |
| Fri | Weekly Debrief | Present findings | Share observations |

**Week 2: Active Participation**

| Day | Activity | Primary Responsibility | Shadow Responsibility |
|-----|----------|----------------------|----------------------|
| Mon | Daily Health Check | Supervise | Execute with guidance |
| Mon | Alert Triage | Supervise | Lead response with oversight |
| Tue | Incident Response (if any) | Backup | Lead with primary as backup |
| Wed | Deployment Review | Supervise | Execute approval checklist |
| Thu | Capacity Review | Supervise | Analyze and present findings |
| Fri | Certification Assessment | Evaluate | Demonstrate competency |

---

#### Shadow On-Call Guidelines

**For Shadows:**
1. **Be Present:** Available during primary's on-call hours
2. **Ask Questions:** No question is too basic during learning period
3. **Document Everything:** Take detailed notes on all activities
4. **Propose Solutions:** Suggest approaches before primary acts (learning exercise)
5. **Hands-On (Week 2):** Execute runbooks with primary supervision

**For Primary On-Call:**
1. **Narrate Actions:** Explain reasoning behind each decision
2. **Encourage Participation:** Let shadow attempt tasks (with supervision)
3. **Provide Context:** Share historical context and patterns
4. **Give Feedback:** Daily 15-min debrief on shadow's performance
5. **Safety First:** Override shadow if incorrect action could cause impact

**For Mentors:**
1. **Daily Check-In:** 15-min 1:1 with shadow to discuss learnings
2. **Review Notes:** Read shadow's daily documentation
3. **Answer Questions:** Provide clarification on complex topics
4. **Assess Progress:** Determine if additional shadow time needed
5. **Certification Decision:** Approve or defer certification based on performance

---

#### Shadow On-Call Evaluation Criteria

**Competency Checklist:**

| Skill | Week 1 | Week 2 | Required for Certification |
|-------|--------|--------|---------------------------|
| Execute health check runbook | Observed | Performed | Yes (L1) |
| Triage and classify alerts | Observed | Performed | Yes (L1) |
| Navigate Grafana dashboards | Observed | Performed | Yes (L1) |
| Query logs in Loki | Observed | Performed | Yes (L2) |
| Execute deployment runbook | Observed | Performed | Yes (L2) |
| Lead incident response | Observed | Performed | Yes (L2) |
| Perform root cause analysis | Observed | Performed | Yes (L2) |
| Execute rollback procedure | Observed | Performed | Yes (L2) |
| Make architectural decisions | Observed | Observed | Yes (L3) |

**Assessment Rubric:**

**Level 1 (Novice):** Can execute runbook with significant guidance
**Level 2 (Competent):** Can execute runbook independently with occasional questions
**Level 3 (Proficient):** Can execute runbook independently and troubleshoot deviations
**Level 4 (Expert):** Can execute runbook, handle edge cases, and mentor others

**Certification Threshold:**
- **L1:** Achieve Level 2 on all L1-required skills
- **L2:** Achieve Level 3 on all L2-required skills
- **L3:** Achieve Level 4 on all L3-required skills

---

## 7.4 Handoff Checklist

### 7.4.1 Access Provisioning

#### Kubernetes Access

**Production Cluster Access:**

| Role | Namespace Access | Permissions | Approval Required |
|------|-----------------|-------------|------------------|
| L1 Operator | llm-research-lab-prod (read-only) | get, list, describe pods/services/deployments | Manager |
| L2 Engineer | llm-research-lab-prod (read-write) | get, list, describe, logs, exec (non-destructive) | Manager + Lead SRE |
| L3 SRE | llm-research-lab-prod (admin) | Full access including delete, edit | Manager + Engineering Director |

**Provisioning Steps:**
1. Create ServiceAccount:
   ```bash
   kubectl create serviceaccount <username>-sa -n llm-research-lab-prod
   ```
2. Apply RoleBinding:
   ```bash
   kubectl apply -f rbac/<role>-rolebinding.yaml
   ```
3. Generate kubeconfig:
   ```bash
   ./scripts/generate-kubeconfig.sh <username> <role>
   ```
4. Verify access:
   ```bash
   kubectl auth can-i --as=system:serviceaccount:llm-research-lab-prod:<username>-sa list pods
   ```

**Checklist:**
- [ ] ServiceAccount created
- [ ] RoleBinding applied
- [ ] Kubeconfig delivered securely (1Password vault)
- [ ] Access verified by user
- [ ] Access documented in access log

---

#### Database Access

**PostgreSQL Access:**

| Role | Database | Access Level | Approval Required |
|------|----------|--------------|------------------|
| L1 Operator | llm_research | Read-only (SELECT on specific tables) | Manager |
| L2 Engineer | llm_research | Read-only (SELECT all tables) | Manager + DBA |
| L3 SRE | llm_research | Read-write (SELECT, INSERT, UPDATE on specific tables) | Manager + DBA + Security |

**Provisioning Steps:**
1. Create database user:
   ```sql
   CREATE USER <username> WITH PASSWORD '<secure-password>';
   ```
2. Grant permissions:
   ```sql
   -- L1: Read-only on experiments and datasets tables
   GRANT SELECT ON experiments, datasets TO <username>;

   -- L2: Read-only on all tables
   GRANT SELECT ON ALL TABLES IN SCHEMA public TO <username>;

   -- L3: Read-write on specific tables
   GRANT SELECT, INSERT, UPDATE ON experiments, datasets TO <username>;
   ```
3. Store credentials in secret manager:
   ```bash
   kubectl create secret generic <username>-db-creds \
     --from-literal=username=<username> \
     --from-literal=password=<password> \
     -n llm-research-lab-prod
   ```

**Checklist:**
- [ ] Database user created
- [ ] Permissions granted per role
- [ ] Credentials stored in 1Password/Vault
- [ ] Connection string provided
- [ ] Access verified by user

---

#### Monitoring and Observability Access

**Grafana Access:**

| Role | Access Level | Dashboards | Edit Permissions |
|------|--------------|-----------|-----------------|
| L1 Operator | Viewer | All LLM-Research-Lab dashboards | No |
| L2 Engineer | Editor | All dashboards | Create/Edit personal dashboards |
| L3 SRE | Admin | All dashboards | Create/Edit/Delete all dashboards |

**Provisioning Steps:**
1. Create Grafana user via UI or API
2. Assign to appropriate organization and team
3. Set dashboard permissions
4. Configure data source access

**Checklist:**
- [ ] Grafana user created
- [ ] Added to "LLM-Research-Lab Operations" team
- [ ] Dashboard permissions verified
- [ ] Login credentials delivered

**PagerDuty Access:**

- **L1 Operator:** Add to escalation policy as responder
- **L2 Engineer:** Add as primary on-call rotation
- **L3 SRE:** Add as escalation point

**Checklist:**
- [ ] PagerDuty user created/updated
- [ ] Added to escalation policy
- [ ] Notification preferences configured
- [ ] Test page sent and acknowledged

---

### 7.4.2 Tool Access Verification

#### Access Verification Matrix

| Tool | L1 Operator | L2 Engineer | L3 SRE | Verification Method |
|------|-------------|-------------|---------|-------------------|
| Kubernetes Cluster | Read pods, logs | Read + exec | Full admin | `kubectl auth can-i` |
| PostgreSQL | SELECT on key tables | SELECT all tables | SELECT/INSERT/UPDATE | Test query |
| ClickHouse | Read-only | Read-only | Read-write | Test query |
| Redis | No direct access | redis-cli read | redis-cli read-write | Test command |
| Kafka | No direct access | kafka-console-consumer | kafka-topics admin | Test command |
| Grafana | Viewer | Editor | Admin | Login and verify |
| Prometheus | Query access | Query access | Admin API | Test query |
| Jaeger | Read traces | Read traces | Read traces | Search trace |
| PagerDuty | Incident responder | Primary on-call | Escalation point | Test page |
| GitHub Repo | Read | Read + Comment | Read + Write | Clone repo |
| 1Password Vault | Read shared secrets | Read shared secrets | Manage secrets | Retrieve test secret |

**Verification Procedure:**

1. **Schedule Verification Session:** 1 hour per operator
2. **Execute Verification:**
   - Operator attempts access to each tool
   - Operator performs test action (read/write based on role)
   - Verifier observes and confirms
3. **Document Results:** Record pass/fail for each tool in checklist
4. **Remediate Failures:** If access fails, re-provision and retest
5. **Sign-Off:** Both operator and verifier sign access verification form

---

### 7.4.3 Emergency Contact List

#### Primary Contacts

| Role | Name | Phone | Email | Slack Handle | Availability |
|------|------|-------|-------|--------------|-------------|
| **L3 SRE Lead** | [Name] | +1-XXX-XXX-XXXX | sre-lead@company.com | @sre-lead | 24/7 |
| **Engineering Manager** | [Name] | +1-XXX-XXX-XXXX | eng-manager@company.com | @eng-manager | 24/7 |
| **Lead Architect** | [Name] | +1-XXX-XXX-XXXX | architect@company.com | @architect | Business hours + on-call |
| **DBA On-Call** | Rotation | +1-XXX-XXX-XXXX | dba-oncall@company.com | @dba-oncall | 24/7 |
| **Security On-Call** | Rotation | +1-XXX-XXX-XXXX | security-oncall@company.com | @security-oncall | 24/7 |
| **Infrastructure Lead** | [Name] | +1-XXX-XXX-XXXX | infra-lead@company.com | @infra-lead | 24/7 |

#### Vendor Support Contacts

| Vendor | Service | Contact Method | SLA | Support Level |
|--------|---------|---------------|-----|--------------|
| **Cloud Provider** | Kubernetes, VMs | Support portal + phone | 1 hour response (P1) | Premium |
| **Database Vendor** | PostgreSQL support | Email + Slack | 4 hour response | Standard |
| **Kafka Support** | Confluent Cloud | Support portal | 2 hour response (P1) | Enterprise |
| **Monitoring Vendor** | Grafana Cloud | Support portal + email | 8 hour response | Pro |

#### Escalation Matrix

**Incident Escalation Path:**

```
P0 (Critical Outage):
L1 Operator → L2 Engineer (immediate) → L3 SRE (if unresolved in 15 min) → Engineering Manager (if unresolved in 30 min)

P1 (Severe Degradation):
L1 Operator → L2 Engineer (immediate) → L3 SRE (if unresolved in 30 min) → Engineering Manager (if unresolved in 1 hour)

P2 (Moderate Impact):
L1 Operator → L2 Engineer (within 30 min) → L3 SRE (if unresolved in 2 hours)

P3 (Low Impact):
L1 Operator → L2 Engineer (within 2 hours)

P4 (No Impact):
L1 Operator creates ticket, no immediate escalation
```

**Emergency Contact Procedures:**
1. **Primary Contact:** Always attempt primary contact first (Slack, then phone)
2. **Escalation Delay:** Follow time-based escalation per severity
3. **After-Hours:** Use PagerDuty to auto-escalate if primary does not respond in 10 minutes
4. **All-Hands:** For P0 incidents, Engineering Manager may invoke "all-hands" to pull in additional engineers

---

### 7.4.4 Escalation Procedures Confirmed

#### Escalation Procedure Validation

**Pre-Handoff Test:**
1. **Simulate P1 Incident:** Create realistic test incident (e.g., API latency spike in staging)
2. **L1 Operator Response:** L1 triages and follows escalation procedure
3. **L2 Engineer Response:** L2 responds and attempts resolution
4. **L3 SRE Escalation (if needed):** L3 is notified and provides guidance
5. **Debrief:** Team reviews escalation flow, communication, and resolution

**Validation Checklist:**
- [ ] L1 correctly identified incident severity
- [ ] L1 escalated to L2 within target time
- [ ] L2 acknowledged page within 5 minutes
- [ ] Communication in Slack incident channel was clear and timely
- [ ] Status page updated appropriately
- [ ] Incident documented in ticketing system
- [ ] Post-incident review scheduled

**Sign-Off:**
- [ ] L1 Operator confirms understanding of escalation procedures
- [ ] L2 Engineer confirms receipt of escalations
- [ ] L3 SRE confirms backup support availability
- [ ] Engineering Manager signs off on escalation readiness

---

## 7.5 Support Transition

### 7.5.1 Support Tier Definitions

#### L1 Support Operator

**Responsibilities:**
- Monitor dashboards and alert queues
- Execute standard health check runbooks
- Triage incoming alerts and incidents
- Escalate to L2 with detailed context
- Update status page and incident tickets
- Perform routine maintenance tasks (certificate checks, log rotation verification)

**Skills Required:**
- Basic Kubernetes knowledge (kubectl get, describe, logs)
- Ability to read and interpret Grafana dashboards
- Understanding of HTTP status codes and API concepts
- Familiarity with ticketing systems (Jira, ServiceNow)
- Clear written and verbal communication

**Scope of Changes:**
- **Allowed:** No configuration changes, read-only access
- **Escalation Required:** Any changes beyond health checks

**Typical Incidents:**
- P3/P4 alerts (low priority)
- Routine monitoring and status checks
- Customer support ticket triage

---

#### L2 Support Engineer

**Responsibilities:**
- Respond to P1-P3 incidents
- Execute diagnostic and remediation runbooks
- Perform deployments and rollbacks
- Conduct root cause analysis
- Mentor L1 operators
- Create and update runbooks
- Execute database and infrastructure maintenance

**Skills Required:**
- Advanced Kubernetes operations (exec, port-forward, edit resources)
- PostgreSQL query analysis and optimization
- Kafka consumer group management
- Prometheus/PromQL proficiency
- Distributed tracing analysis
- Scripting (bash, Python)

**Scope of Changes:**
- **Allowed:** Restart pods, scale deployments, execute approved runbooks, modify non-critical configs
- **Escalation Required:** Schema changes, architectural modifications, critical config changes

**Typical Incidents:**
- P1-P3 incidents
- Performance degradation
- Deployment failures
- Database optimization
- Capacity adjustments

---

#### L3 SRE Specialist

**Responsibilities:**
- Respond to P0/P1 critical incidents as escalation point
- Architect-level troubleshooting and decision-making
- Emergency architectural changes
- Capacity planning and long-term optimization
- Disaster recovery execution
- Review and approve runbooks created by L2
- Mentor L2 engineers
- Participate in design reviews for new features

**Skills Required:**
- Expert Rust and systems programming knowledge
- Deep understanding of LLM-Research-Lab architecture
- Database administration (PostgreSQL, ClickHouse)
- Infrastructure-as-code (Terraform, Helm)
- Performance profiling and optimization
- Security hardening and compliance

**Scope of Changes:**
- **Allowed:** All changes including architectural modifications, critical config changes, emergency schema migrations
- **Escalation Required:** Business-impacting decisions (e.g., data deletion) require Engineering Manager approval

**Typical Incidents:**
- P0/P1 critical outages
- Complex multi-component failures
- Disaster recovery scenarios
- Major performance optimization
- Security incidents

---

### 7.5.2 SLA Requirements

#### Service Level Objectives (SLOs)

| Metric | Target | Measurement Window | Consequences of Miss |
|--------|--------|--------------------|---------------------|
| **API Availability** | 99.9% | 30 days | Executive review, RCA required |
| **API Latency (p99)** | < 100ms | 7 days | Performance optimization sprint |
| **Experiment Success Rate** | > 99% | 30 days | Investigate error patterns |
| **Data Durability** | 99.999% | Annual | Immediate DR review |

#### Support Response SLAs

| Severity | Response Time | Resolution Time | Support Tier | Business Hours | After-Hours |
|----------|--------------|-----------------|--------------|---------------|-------------|
| **P0** (Critical Outage) | 15 minutes | 4 hours | L3 SRE | Yes | Yes |
| **P1** (Severe Degradation) | 30 minutes | 8 hours | L2 → L3 | Yes | Yes |
| **P2** (Moderate Impact) | 2 hours | 24 hours | L2 | Yes | Best effort |
| **P3** (Low Impact) | 8 hours | 3 business days | L1 → L2 | Yes | No |
| **P4** (No Impact) | 24 hours | 5 business days | L1 | Yes | No |

**SLA Tracking:**
- Incidents tracked in Jira with timestamps for detection, response, and resolution
- Weekly SLA compliance reports generated automatically
- Monthly SLA review meeting with stakeholders

---

### 7.5.3 Ticket Routing Rules

#### Automated Ticket Routing

**Integration:** PagerDuty → Jira Service Desk

**Routing Logic:**

```yaml
# ticket-routing-rules.yaml
rules:
  - name: "Critical Alerts"
    condition: "severity == 'P0' OR severity == 'P1'"
    action:
      - create_incident: true
      - assign_to: "L2-oncall-rotation"
      - notify: ["slack://incidents", "pagerduty://sre-team"]
      - priority: "Critical"

  - name: "Performance Degradation"
    condition: "alert_name contains 'latency' OR alert_name contains 'throughput'"
    action:
      - create_ticket: true
      - assign_to: "L2-performance-team"
      - labels: ["performance", "auto-routed"]
      - priority: "High"

  - name: "Database Issues"
    condition: "component == 'postgresql' OR component == 'clickhouse'"
    action:
      - create_ticket: true
      - assign_to: "L2-database-team"
      - cc: ["dba-team"]
      - labels: ["database", "auto-routed"]

  - name: "Kafka/Messaging"
    condition: "component == 'kafka' OR alert_name contains 'consumer_lag'"
    action:
      - create_ticket: true
      - assign_to: "L2-platform-team"
      - labels: ["messaging", "auto-routed"]

  - name: "Security Alerts"
    condition: "alert_name contains 'security' OR alert_name contains 'unauthorized'"
    action:
      - create_incident: true
      - assign_to: "security-team"
      - notify: ["pagerduty://security-oncall"]
      - priority: "Critical"

  - name: "Default Routing"
    condition: "default"
    action:
      - create_ticket: true
      - assign_to: "L1-triage-queue"
      - priority: "Medium"
```

**Manual Routing (via Jira):**

- **Component Field:** Set to "llm-research-lab"
- **Labels:** Performance, Database, Kafka, Security, Deployment, etc.
- **Assign Based on Expertise:** L1 triages and manually assigns to appropriate L2 team if auto-routing fails

---

### 7.5.4 Knowledge Base Setup

#### Knowledge Base Structure

**Platform:** Confluence or Internal Wiki

**Hierarchy:**
```
LLM-Research-Lab Operations Knowledge Base
├── Architecture
│   ├── System Overview
│   ├── Component Deep-Dives
│   ├── Data Flow Diagrams
│   └── Integration Points
├── Runbooks
│   ├── Health Checks
│   ├── Incident Response
│   ├── Maintenance Procedures
│   └── Deployment Guides
├── Troubleshooting
│   ├── Common Issues
│   ├── Error Messages Reference
│   ├── Decision Trees
│   └── Root Cause Analysis Examples
├── How-To Guides
│   ├── Access Provisioning
│   ├── Monitoring Setup
│   ├── Custom Dashboard Creation
│   └── Deployment Procedures
├── Reference
│   ├── API Documentation (link to Rustdoc)
│   ├── Database Schema
│   ├── Kafka Topics Reference
│   ├── Metrics Glossary
│   └── Configuration Reference
└── Training Materials
    ├── Video Recordings
    ├── Training Slides
    ├── Certification Exams
    └── Q&A Archive
```

#### Knowledge Base Content Requirements

**Each Article Must Include:**
1. **Title:** Clear, descriptive, follows naming convention
2. **Metadata:** Last updated date, owner, review cycle
3. **Summary:** 2-3 sentence overview
4. **Prerequisites:** Required knowledge, access, tools
5. **Content:** Step-by-step instructions with screenshots/code samples
6. **Validation:** How to verify successful completion
7. **Troubleshooting:** Common issues and resolutions
8. **Related Articles:** Links to related documentation
9. **Feedback:** Comment section for questions and improvements

**Quality Standards:**
- **Accuracy:** Technically accurate and tested
- **Clarity:** Written for target audience (L1/L2/L3)
- **Completeness:** All steps included, no assumed knowledge
- **Currency:** Reviewed and updated quarterly
- **Searchability:** Proper tagging and keywords

#### Knowledge Base Maintenance

**Ownership:**
- Each article assigned to a primary owner (engineer responsible for updates)
- Quarterly review cycle to ensure accuracy
- Automated reminders for articles >6 months old

**Update Triggers:**
- New feature deployment
- Architecture change
- Incident reveals gap in documentation
- Feedback from operations team

**Approval Workflow:**
1. Author creates/updates article
2. Peer review by L2/L3 engineer
3. Technical review by subject matter expert
4. Final approval by SRE Lead
5. Published to knowledge base
6. Announced in team Slack channel

---

## Appendix A: Training Templates

### Template: Training Module

```markdown
# Training Module: [Module Name]

## Module Information
- **Module ID:** [Unique identifier]
- **Duration:** [Hours]
- **Target Audience:** [L1/L2/L3]
- **Prerequisites:** [Required knowledge]
- **Instructor:** [Name]

## Learning Objectives
By the end of this module, participants will be able to:
1. [Objective 1]
2. [Objective 2]
3. [Objective 3]

## Agenda
| Time | Topic | Format |
|------|-------|--------|
| 0:00-0:30 | [Topic 1] | Lecture |
| 0:30-1:00 | [Topic 2] | Demo |
| 1:00-1:30 | [Hands-On Lab] | Practical |
| 1:30-1:45 | Q&A | Discussion |

## Materials
- Slides: [Link]
- Lab Instructions: [Link]
- Reference Documentation: [Link]

## Assessment
- Quiz: [Number of questions], [Pass percentage]%
- Practical Exam: [Description]

## Resources
- [Resource 1]
- [Resource 2]
```

---

### Template: Hands-On Exercise

```markdown
# Exercise: [Exercise Name]

## Objective
[Clear statement of what participant will accomplish]

## Prerequisites
- Access to: [List required access]
- Tools installed: [List required tools]
- Knowledge of: [Required concepts]

## Scenario
[Realistic scenario description]

## Steps
### Step 1: [Action]
```bash
[Command or action]
```
**Expected Result:** [What should happen]

### Step 2: [Action]
[Instructions]

**Expected Result:** [What should happen]

## Validation
[How to confirm successful completion]

## Troubleshooting
| Issue | Cause | Resolution |
|-------|-------|------------|
| [Common issue 1] | [Root cause] | [Fix] |

## Estimated Time
[Minutes to complete]

## Instructor Notes
[Tips for instructors]
```

---

## Appendix B: Certification Criteria

### L1 Support Operator Certification

**Written Exam (30 questions, 85% pass required):**

Sample Questions:
1. What is the first step when responding to an alert? (Multiple choice)
2. Which Grafana dashboard shows API latency metrics? (Multiple choice)
3. Describe the escalation path for a P1 incident. (Short answer)
4. What command would you use to check pod status in Kubernetes? (Fill in blank)
5. Identify the error from this log message: `ERROR: connection pool exhausted` (Multiple choice)

**Practical Exam (3 exercises, all must pass):**

1. **Exercise 1:** Execute service health check runbook
   - Time limit: 20 minutes
   - Pass criteria: All health checks completed, results documented

2. **Exercise 2:** Triage mock alert and escalate appropriately
   - Time limit: 15 minutes
   - Pass criteria: Correct severity classification, proper escalation, incident ticket created

3. **Exercise 3:** Navigate Grafana to identify latency spike
   - Time limit: 15 minutes
   - Pass criteria: Identify timestamp of spike, correlate with logs, document findings

**Shadow Period (5 incidents):**
- Observe and assist with 5 real or simulated incidents
- Mentor signs off on each incident response
- Demonstrate communication and documentation skills

---

### L2 Support Engineer Certification

**Written Exam (50 questions, 90% pass required):**

Includes L1 content plus:
- Advanced Kubernetes operations
- Database query optimization
- Kafka troubleshooting
- Deployment procedures
- Root cause analysis methodology

**Practical Exam (4 exercises, all must pass):**

1. **Exercise 1:** Diagnose and resolve API latency spike
2. **Exercise 2:** Execute database backup and restore
3. **Exercise 3:** Scale experiment workers based on Kafka lag
4. **Exercise 4:** Execute deployment rollback

**Shadow Period (3 incidents + 1 deployment):**
- Lead 3 incident responses under L3 supervision
- Execute 1 production deployment under supervision

**Deliverables:**
- 2 incident post-mortems written by candidate
- 1 new runbook or documentation contribution

---

### L3 SRE Specialist Certification

**Advanced Training (8 additional hours):**
- Architecture deep-dive with source code review
- Performance optimization techniques
- Disaster recovery planning and execution

**Practical Assessment:**
- Design 2 new runbooks for complex scenarios
- Lead post-mortem for 3 incidents (demonstrating root cause analysis)
- Mentor 2 L1/L2 engineers through certification

**Architecture Review Interview:**
- 90-minute technical interview with development team
- Questions on system design, trade-offs, and optimization
- Pass/fail decision by panel

**Sign-Off:**
- Engineering Manager approval
- Lead Architect approval
- Principal SRE approval

---

## Document Metadata

| Field | Value |
|-------|-------|
| **Version** | 1.0.0 |
| **Status** | Complete |
| **SPARC Phase** | Phase 5 - Completion |
| **Section** | 7 - Handoff & Knowledge Transfer |
| **Created** | 2025-11-28 |
| **Target Audience** | Operations Teams, Engineering Managers, SREs |
| **Handoff Timeline** | 4 weeks (2 weeks training + 2 weeks shadow) |

---

## Next Steps

1. **Week -2:** Finalize training materials and schedule sessions
2. **Week 1-2:** Execute training program with all operations staff
3. **Week 2:** Administer certification exams and practical assessments
4. **Week 3-4:** Shadow on-call period with experienced engineers
5. **Week 4 End:** Handoff approval and sign-off
6. **Week 5+:** Operations team assumes full responsibility with dev team backup

---

*This document is part of the SPARC Phase 5 (Completion) specification for LLM-Research-Lab, ensuring a comprehensive and successful transition from development to operations.*
