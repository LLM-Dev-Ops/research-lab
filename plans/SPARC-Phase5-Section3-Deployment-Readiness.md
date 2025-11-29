# SPARC Phase 5: Section 3 - Deployment Readiness

**Project**: LLM-Research-Lab
**Stack**: Rust + Axum + PostgreSQL + ClickHouse + Kubernetes
**Version**: 1.0.0
**Status**: Specification
**Last Updated**: 2025-11-28

---

## Table of Contents

- [3.1 Pre-Deployment Checklist](#31-pre-deployment-checklist)
- [3.2 Environment Certification](#32-environment-certification)
- [3.3 Deployment Artifacts](#33-deployment-artifacts)

---

## 3.1 Pre-Deployment Checklist

### 3.1.1 Code Freeze Verification

**Objective**: Ensure the codebase is stable, tagged, and ready for production deployment with no active development branches.

```yaml
code_freeze:
  branch_verification:
    - action: "Verify main branch is stable"
      command: "git checkout main && git pull origin main"
      validation: "No uncommitted changes or conflicts"

    - action: "Tag release candidate"
      command: "git tag -a v1.0.0-rc.1 -m 'Release candidate for production'"
      validation: "Tag created and pushed to origin"

    - action: "Lock main branch"
      method: "GitHub branch protection rules"
      rules:
        - "Require pull request reviews (2 approvers)"
        - "Require status checks to pass"
        - "Restrict pushes to release managers only"

  dependency_lock:
    - action: "Verify Cargo.lock is committed"
      command: "git ls-files | grep Cargo.lock"
      validation: "Cargo.lock present and up-to-date"

    - action: "Pin all dependencies"
      file: "Cargo.toml"
      requirement: "All dependencies use exact versions (=x.y.z)"

    - action: "Audit dependency tree"
      command: "cargo tree --locked > dependency-tree.txt"
      validation: "No version conflicts or ambiguous dependencies"

  build_verification:
    - action: "Clean production build"
      command: "cargo clean && cargo build --release --locked"
      validation: "Build completes with zero warnings"

    - action: "Verify binary reproducibility"
      method: "Rebuild twice and compare checksums"
      validation: "SHA256 checksums match across builds"
```

**Checklist**:
- [ ] Main branch is stable and up-to-date
- [ ] Release tag created (e.g., v1.0.0-rc.1)
- [ ] Branch protection rules enforced
- [ ] Cargo.lock committed and verified
- [ ] All dependencies pinned to exact versions
- [ ] Production build succeeds with zero warnings
- [ ] Binary checksums verified for reproducibility

---

### 3.1.2 Quality Gates Passed

**Objective**: Validate all code quality, security, and performance thresholds are met before deployment.

```yaml
quality_gates:
  code_coverage:
    metric: "Line coverage ≥ 85%"
    command: "cargo tarpaulin --out Xml --output-dir ./coverage"
    validation: "coverage.xml shows ≥85% line coverage"
    blocker: true

  mutation_testing:
    metric: "Mutation score ≥ 70%"
    command: "cargo mutants --output mutants.json"
    validation: "Mutation score ≥70% in mutants.json"
    blocker: true

  static_analysis:
    - tool: "Clippy (pedantic mode)"
      command: "cargo clippy --all-targets --all-features -- -D warnings -W clippy::pedantic"
      validation: "Zero warnings or errors"
      blocker: true

    - tool: "Rustfmt"
      command: "cargo fmt -- --check"
      validation: "All code formatted consistently"
      blocker: true

  security_scanning:
    - tool: "cargo-audit"
      command: "cargo audit --deny warnings"
      validation: "Zero critical/high vulnerabilities"
      blocker: true

    - tool: "Trivy (container scan)"
      command: "trivy image llm-research-lab:latest --severity CRITICAL,HIGH"
      validation: "Zero critical/high vulnerabilities"
      blocker: true

    - tool: "Semgrep"
      command: "semgrep --config=auto --severity=ERROR ."
      validation: "Zero security-relevant errors"
      blocker: true

    - tool: "gitleaks"
      command: "gitleaks detect --no-git --source=."
      validation: "Zero secrets detected"
      blocker: true

  performance_validation:
    - metric: "API latency p99 < 100ms"
      tool: "k6 load tests"
      validation: "Load test report confirms p99 < 100ms"
      blocker: true

    - metric: "Memory stability"
      tool: "72-hour soak test"
      validation: "No memory leaks or degradation"
      blocker: true
```

**Quality Gate Summary**:

| Gate | Metric | Target | Status | Blocker |
|------|--------|--------|--------|---------|
| Code Coverage | Line coverage | ≥85% | ✅ Pass | Yes |
| Mutation Testing | Mutation score | ≥70% | ✅ Pass | Yes |
| Clippy | Warnings | 0 | ✅ Pass | Yes |
| Cargo Audit | Critical/High vulns | 0 | ✅ Pass | Yes |
| Trivy Scan | Container vulns | 0 | ✅ Pass | Yes |
| API Latency | p99 latency | <100ms | ✅ Pass | Yes |

---

### 3.1.3 Documentation Complete

**Objective**: Ensure all customer-facing and internal documentation is complete, accurate, and accessible.

```yaml
documentation_completeness:
  api_documentation:
    - artifact: "OpenAPI 3.0 specification"
      location: "docs/api/openapi.yaml"
      validation: "All endpoints documented with examples"
      coverage: "100% public API"

    - artifact: "rustdoc generated docs"
      command: "cargo doc --no-deps --document-private-items"
      location: "target/doc/llm_research_lab/index.html"
      validation: "All public APIs have doc comments"

  operational_documentation:
    - artifact: "Deployment Runbook"
      location: "docs/operations/deployment.md"
      sections:
        - "Prerequisites and dependencies"
        - "Step-by-step deployment procedure"
        - "Rollback procedure"
        - "Smoke testing checklist"

    - artifact: "Incident Response Playbook"
      location: "docs/operations/incident-response.md"
      sections:
        - "Alert triage and severity classification"
        - "Common failure modes and remediation"
        - "Escalation paths and contact information"

    - artifact: "Monitoring and Observability Guide"
      location: "docs/operations/monitoring.md"
      sections:
        - "Key metrics and dashboard locations"
        - "Alert definitions and thresholds"
        - "Log aggregation queries"

  customer_documentation:
    - artifact: "Getting Started Guide"
      location: "docs/user/getting-started.md"
      validation: "New user can onboard in <30 minutes"

    - artifact: "API Reference"
      location: "docs/user/api-reference.md"
      validation: "All endpoints documented with curl examples"

    - artifact: "Troubleshooting Guide"
      location: "docs/user/troubleshooting.md"
      validation: "Top 10 common issues documented"
```

**Documentation Checklist**:
- [ ] OpenAPI 3.0 specification complete
- [ ] Rustdoc generated with 100% public API coverage
- [ ] Deployment runbook reviewed and validated
- [ ] Incident response playbook approved by SRE
- [ ] Monitoring guide includes all critical dashboards
- [ ] Getting Started guide tested with new users
- [ ] API reference includes examples for all endpoints

---

### 3.1.4 Release Notes Finalized

**Objective**: Communicate changes, improvements, and migration requirements to stakeholders.

```markdown
# Release Notes v1.0.0

## Release Information
- **Version**: 1.0.0
- **Release Date**: 2025-12-15
- **Release Type**: General Availability (GA)
- **SPARC Phase**: Completion (Phase 5)

## What's New

### Experiment Tracking
- Full-lifecycle experiment management with versioning and provenance
- Integration with LLM-Registry for artifact storage
- Real-time metrics streaming to LLM-Analytics-Hub

### Metric Benchmarking
- Standardized framework for custom evaluation metrics
- Statistical significance testing with confidence intervals
- Comparative analysis dashboards

### Dataset Versioning
- Content-addressable storage with cryptographic verification
- Lineage tracking for dataset transformations
- Integration with LLM-Data-Vault for governance

## Breaking Changes
- None (initial GA release)

## Migration Guide
- **New Deployments**: Follow the Getting Started guide in docs/user/getting-started.md
- **Database Schema**: Initial schema deployment via migrations/001_initial_schema.sql

## Known Issues
- None at GA release

## Performance Improvements
- API latency p99 < 100ms under 5x expected load
- Experiment throughput: 200+ experiments/week

## Security Enhancements
- Zero critical/high vulnerabilities (cargo-audit, Trivy, Semgrep)
- TLS 1.3 enforcement for all API endpoints
- mTLS for inter-service communication

## Dependencies
- Rust: 1.75+
- PostgreSQL: 13+
- ClickHouse: 22+
- Kubernetes: 1.24+

## Upgrade Instructions
- Not applicable (initial GA release)

## Documentation
- API Reference: https://docs.llm-research-lab/api
- User Guide: https://docs.llm-research-lab/user-guide
- Operations Manual: https://docs.llm-research-lab/operations

## Support
- GitHub Issues: https://github.com/org/llm-research-lab/issues
- Slack: #llm-research-lab-support
- Email: support@llm-devops.example.com
```

---

### 3.1.5 Stakeholder Approvals

**Objective**: Obtain formal sign-off from all required stakeholders before production deployment.

```yaml
stakeholder_approvals:
  technical_approvals:
    - role: "VP Engineering / CTO"
      deliverable: "Technical Production Readiness Report"
      criteria:
        - "All quality gates passed"
        - "Architecture reviewed and approved"
        - "No critical technical debt"
      status: "Pending"
      sign_off_date: null

    - role: "Principal Security Engineer"
      deliverable: "Security Certification Report"
      criteria:
        - "Zero critical/high vulnerabilities"
        - "Penetration testing passed"
        - "Security compliance checklist complete"
      status: "Pending"
      sign_off_date: null

    - role: "Director of Platform Engineering"
      deliverable: "Infrastructure Certification"
      criteria:
        - "Production infrastructure deployed and tested"
        - "Monitoring and observability operational"
        - "Disaster recovery validated"
      status: "Pending"
      sign_off_date: null

  operational_approvals:
    - role: "VP Operations / Head of SRE"
      deliverable: "Operational Readiness Certification"
      criteria:
        - "Operations team trained"
        - "Runbooks complete and validated"
        - "On-call rotation staffed"
      status: "Pending"
      sign_off_date: null

  business_approvals:
    - role: "VP Product"
      deliverable: "Product Launch Readiness"
      criteria:
        - "Product meets requirements"
        - "Customer onboarding tested"
        - "Success metrics defined"
      status: "Pending"
      sign_off_date: null
```

**Approval Checklist**:
- [ ] VP Engineering sign-off on technical readiness
- [ ] Security Engineer sign-off on security certification
- [ ] Platform Engineering sign-off on infrastructure
- [ ] VP Operations sign-off on operational readiness
- [ ] VP Product sign-off on product launch readiness

---

## 3.2 Environment Certification

### 3.2.1 Production Environment Validation

**Objective**: Verify production infrastructure is provisioned, configured, and operational before deploying application code.

```yaml
production_environment:
  kubernetes_cluster:
    provider: "AWS EKS / GCP GKE / Azure AKS"
    version: "1.28+"
    configuration:
      node_pools:
        - name: "system-pool"
          machine_type: "n2-standard-4"
          min_nodes: 3
          max_nodes: 5
          labels:
            workload: "system"

        - name: "application-pool"
          machine_type: "n2-standard-8"
          min_nodes: 5
          max_nodes: 20
          labels:
            workload: "application"

        - name: "gpu-pool"
          machine_type: "n1-standard-8 (with T4 GPUs)"
          min_nodes: 0
          max_nodes: 10
          labels:
            workload: "gpu-inference"
          taints:
            - key: "nvidia.com/gpu"
              value: "true"
              effect: "NoSchedule"

      addons:
        - "metrics-server"
        - "cluster-autoscaler"
        - "vertical-pod-autoscaler"
        - "cert-manager"
        - "ingress-nginx"

  databases:
    postgresql:
      provider: "AWS RDS / Cloud SQL / Azure Database"
      version: "15.4"
      instance_class: "db.r6g.xlarge"
      storage: "500GB gp3"
      configuration:
        multi_az: true
        backup_retention: "30 days"
        point_in_time_recovery: true
        encryption_at_rest: true
        ssl_mode: "require"
      validation:
        - "pg_isready -h $POSTGRES_HOST -p 5432"
        - "psql -c 'SELECT version();'"

    clickhouse:
      deployment: "Kubernetes StatefulSet"
      version: "23.8"
      replicas: 3
      storage: "1TB per replica (SSD)"
      configuration:
        replication: "Replicated tables with Zookeeper coordination"
        sharding: "3 shards for distributed queries"
        compression: "LZ4"
      validation:
        - "clickhouse-client --query='SELECT version()'"
        - "clickhouse-client --query='SELECT count() FROM system.clusters'"

  message_queue:
    provider: "Redis Streams"
    deployment: "Redis Sentinel cluster"
    configuration:
      replicas: 3
      persistence: "AOF + RDB snapshots"
      max_memory: "16GB per instance"
    validation:
      - "redis-cli ping"
      - "redis-cli info replication"

  object_storage:
    provider: "AWS S3 / GCS / MinIO"
    buckets:
      - name: "llm-research-lab-artifacts"
        versioning: true
        lifecycle_policy: "Transition to Glacier after 90 days"
        encryption: "AES-256"

      - name: "llm-research-lab-datasets"
        versioning: true
        lifecycle_policy: "Retain indefinitely"
        encryption: "AES-256"
    validation:
      - "aws s3 ls s3://llm-research-lab-artifacts"
      - "aws s3api get-bucket-versioning --bucket llm-research-lab-artifacts"

  networking:
    load_balancer:
      type: "Application Load Balancer (Layer 7)"
      tls_termination: true
      certificate: "ACM / Let's Encrypt (cert-manager)"
      health_checks:
        path: "/health"
        interval: "10s"
        timeout: "5s"
        healthy_threshold: 2
        unhealthy_threshold: 3

    dns:
      domain: "api.llm-research-lab.example.com"
      provider: "Route53 / Cloud DNS"
      ttl: "60s"
      validation:
        - "nslookup api.llm-research-lab.example.com"
        - "curl -I https://api.llm-research-lab.example.com/health"

    tls_certificates:
      provider: "cert-manager with Let's Encrypt"
      renewal: "Automatic (30 days before expiry)"
      validation:
        - "kubectl get certificate -n llm-research-lab"
        - "openssl s_client -connect api.llm-research-lab.example.com:443 -servername api.llm-research-lab.example.com"
```

**Validation Checklist**:
- [ ] Kubernetes cluster operational with all node pools
- [ ] PostgreSQL database accessible with TLS
- [ ] ClickHouse cluster operational with replication
- [ ] Redis Sentinel cluster accessible
- [ ] S3 buckets created with versioning enabled
- [ ] Load balancer configured with TLS termination
- [ ] DNS records resolving correctly
- [ ] TLS certificates issued and valid

---

### 3.2.2 Infrastructure-as-Code Verification

**Objective**: Ensure all infrastructure is defined declaratively, version-controlled, and reproducible.

```yaml
iac_verification:
  terraform:
    version: "1.6+"
    modules:
      - name: "networking"
        path: "terraform/modules/networking"
        resources:
          - "VPC with public/private subnets"
          - "NAT gateways for outbound traffic"
          - "Security groups for database access"

      - name: "kubernetes"
        path: "terraform/modules/kubernetes"
        resources:
          - "EKS/GKE/AKS cluster"
          - "Node groups with autoscaling"
          - "IRSA/Workload Identity for service accounts"

      - name: "databases"
        path: "terraform/modules/databases"
        resources:
          - "PostgreSQL RDS instance"
          - "ClickHouse StatefulSet manifests"
          - "Database security groups"

      - name: "storage"
        path: "terraform/modules/storage"
        resources:
          - "S3 buckets with lifecycle policies"
          - "IAM roles for bucket access"

    validation:
      - command: "terraform fmt -check -recursive"
        description: "Verify Terraform code is formatted"

      - command: "terraform validate"
        description: "Validate Terraform syntax"

      - command: "terraform plan -out=tfplan"
        description: "Generate execution plan"

      - command: "terraform show -json tfplan | jq '.resource_changes | length'"
        description: "Verify no unexpected changes"

  helm_charts:
    version: "3.12+"
    charts:
      - name: "llm-research-lab"
        path: "helm/llm-research-lab"
        values_files:
          - "values.production.yaml"
        validation:
          - "helm lint helm/llm-research-lab"
          - "helm template llm-research-lab helm/llm-research-lab -f helm/values.production.yaml | kubeval --strict"

      - name: "monitoring-stack"
        path: "helm/monitoring"
        components:
          - "Prometheus"
          - "Grafana"
          - "Jaeger"
        validation:
          - "helm lint helm/monitoring"
          - "helm template monitoring helm/monitoring | kubeval --strict"

  kubernetes_manifests:
    location: "k8s/"
    structure:
      - "k8s/base/ (common manifests)"
      - "k8s/overlays/production/ (environment-specific)"
    tool: "Kustomize"
    validation:
      - "kustomize build k8s/overlays/production | kubeval --strict"
      - "kustomize build k8s/overlays/production | kubectl apply --dry-run=server -f -"
```

**IaC Validation Commands**:

```bash
# Terraform validation
cd terraform/environments/production
terraform init
terraform fmt -check -recursive
terraform validate
terraform plan -out=tfplan
terraform apply tfplan

# Helm validation
helm lint helm/llm-research-lab
helm template llm-research-lab helm/llm-research-lab \
  -f helm/values.production.yaml \
  --validate

# Kustomize validation
kustomize build k8s/overlays/production | kubeval --strict
kustomize build k8s/overlays/production | kubectl apply --dry-run=server -f -
```

---

### 3.2.3 Secrets Management Audit

**Objective**: Verify all secrets are securely managed, rotated, and accessed through approved mechanisms.

```yaml
secrets_management:
  secrets_provider:
    primary: "AWS Secrets Manager / GCP Secret Manager / Azure Key Vault"
    kubernetes_integration: "External Secrets Operator"
    configuration:
      rotation_policy: "Automatic rotation every 90 days"
      access_control: "IAM-based with least-privilege"
      audit_logging: "All secret access logged to CloudTrail/Cloud Audit Logs"

  secret_categories:
    database_credentials:
      - name: "postgresql-admin-password"
        rotation: "90 days"
        access: "application-service-account only"

      - name: "clickhouse-admin-password"
        rotation: "90 days"
        access: "application-service-account only"

    api_keys:
      - name: "llm-registry-api-key"
        rotation: "180 days"
        access: "application-service-account only"

      - name: "llm-data-vault-api-key"
        rotation: "180 days"
        access: "application-service-account only"

    tls_certificates:
      - name: "mtls-client-certificate"
        rotation: "365 days (automated via cert-manager)"
        access: "ingress-controller, application pods"

    encryption_keys:
      - name: "data-encryption-key (DEK)"
        rotation: "Annual (with re-encryption)"
        access: "application-service-account only"
        key_management: "AWS KMS / Cloud KMS envelope encryption"

  kubernetes_secrets:
    storage_class: "external-secrets (synced from cloud provider)"
    encryption_at_rest: true
    rbac:
      - "ServiceAccount 'llm-research-lab-app' can read secrets in namespace 'llm-research-lab'"
      - "No direct kubectl access to secrets in production"

  validation:
    - action: "Audit secret access logs"
      command: "aws cloudtrail lookup-events --lookup-attributes AttributeKey=ResourceName,AttributeValue=postgresql-admin-password"
      frequency: "Weekly"

    - action: "Verify no hardcoded secrets"
      command: "gitleaks detect --no-git --source=."
      frequency: "Every commit (CI/CD)"

    - action: "Rotate test secrets"
      description: "Verify rotation mechanism works without downtime"
      frequency: "Quarterly"

    - action: "Review secret access policies"
      description: "Ensure least-privilege is maintained"
      frequency: "Quarterly"
```

**Secrets Audit Checklist**:
- [ ] All secrets stored in managed secret service (no K8s native secrets for sensitive data)
- [ ] Automatic rotation enabled for database credentials
- [ ] External Secrets Operator configured and syncing
- [ ] No secrets detected in Git history (gitleaks scan passed)
- [ ] IAM policies enforce least-privilege access
- [ ] Audit logging enabled for all secret access
- [ ] mTLS certificates managed by cert-manager
- [ ] Encryption-at-rest enabled for Kubernetes secrets

---

### 3.2.4 Network Security Validation

**Objective**: Verify network segmentation, firewall rules, and traffic policies enforce zero-trust principles.

```yaml
network_security:
  network_segmentation:
    architecture: "VPC with public, private, and database subnets"
    subnets:
      - name: "public-subnet"
        purpose: "Load balancers, NAT gateways"
        cidr: "10.0.1.0/24"
        internet_access: "Inbound + Outbound"

      - name: "private-subnet"
        purpose: "Application pods"
        cidr: "10.0.10.0/24"
        internet_access: "Outbound only (via NAT)"

      - name: "database-subnet"
        purpose: "PostgreSQL, ClickHouse"
        cidr: "10.0.20.0/24"
        internet_access: "None (isolated)"

    validation:
      - "Verify application pods cannot directly access internet (must use NAT)"
      - "Verify database subnets have no internet gateway route"

  firewall_rules:
    ingress:
      - name: "allow-https"
        protocol: "TCP"
        port: 443
        source: "0.0.0.0/0"
        destination: "Load balancer"

      - name: "allow-internal-app-traffic"
        protocol: "TCP"
        port: "8080, 8081"
        source: "private-subnet"
        destination: "private-subnet"

      - name: "allow-db-access"
        protocol: "TCP"
        port: "5432 (Postgres), 9000 (ClickHouse)"
        source: "private-subnet"
        destination: "database-subnet"

    egress:
      - name: "allow-outbound-https"
        protocol: "TCP"
        port: 443
        source: "private-subnet"
        destination: "0.0.0.0/0 (via NAT)"

      - name: "deny-all-other-egress"
        protocol: "ALL"
        source: "database-subnet"
        destination: "0.0.0.0/0"
        action: "DENY"

    validation:
      - "Attempt database connection from public subnet (should fail)"
      - "Attempt outbound connection from database subnet (should fail)"
      - "Verify application can connect to databases"

  kubernetes_network_policies:
    default_policy: "Deny all ingress/egress"
    policies:
      - name: "allow-app-to-db"
        selector:
          app: "llm-research-lab"
        ingress: []
        egress:
          - to:
              - podSelector:
                  matchLabels:
                    component: "postgresql"
            ports:
              - protocol: "TCP"
                port: 5432

      - name: "allow-ingress-to-app"
        selector:
          app: "llm-research-lab"
        ingress:
          - from:
              - namespaceSelector:
                  matchLabels:
                    name: "ingress-nginx"
            ports:
              - protocol: "TCP"
                port: 8080

    validation:
      - "kubectl exec -it test-pod -- curl http://llm-research-lab-service:8080/health (from unauthorized namespace should fail)"
      - "kubectl exec -it test-pod -- nc -zv postgresql-service 5432 (from unauthorized namespace should fail)"

  tls_enforcement:
    ingress_tls:
      minimum_version: "TLS 1.3"
      cipher_suites:
        - "TLS_AES_128_GCM_SHA256"
        - "TLS_AES_256_GCM_SHA384"
        - "TLS_CHACHA20_POLY1305_SHA256"
      validation:
        - "nmap --script ssl-enum-ciphers -p 443 api.llm-research-lab.example.com"
        - "testssl.sh --severity MEDIUM api.llm-research-lab.example.com"

    service_mesh_mtls:
      provider: "Istio / Linkerd (optional)"
      mode: "STRICT (enforce mTLS for all inter-pod traffic)"
      validation:
        - "istioctl authn tls-check llm-research-lab-pod.llm-research-lab"
```

**Network Security Validation Commands**:

```bash
# Test network segmentation
kubectl run test-pod --image=busybox --restart=Never --rm -it -- \
  nc -zv postgresql-service.llm-research-lab.svc.cluster.local 5432
# Expected: Connection should only succeed from authorized namespaces

# Test TLS configuration
nmap --script ssl-enum-ciphers -p 443 api.llm-research-lab.example.com
# Expected: Only TLS 1.3 with strong ciphers

# Test network policies
kubectl exec -it unauthorized-pod -n default -- \
  curl http://llm-research-lab-service.llm-research-lab.svc.cluster.local:8080/health
# Expected: Connection refused (network policy blocks)
```

**Network Security Checklist**:
- [ ] VPC subnets properly segmented (public, private, database)
- [ ] Database subnets have no internet access
- [ ] Security groups enforce least-privilege access
- [ ] Kubernetes NetworkPolicies deny all by default
- [ ] TLS 1.3 enforced with strong cipher suites
- [ ] mTLS configured for inter-service communication (if using service mesh)
- [ ] Network policy tests passed (unauthorized access blocked)

---

## 3.3 Deployment Artifacts

### 3.3.1 Container Image Specifications

**Objective**: Build optimized, secure, multi-stage Docker images for production deployment.

**Multi-Stage Dockerfile**:

```dockerfile
# syntax=docker/dockerfile:1.4

#############################################
# Stage 1: Build environment
#############################################
FROM rust:1.75-slim-bookworm AS builder

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    libpq-dev \
    && rm -rf /var/lib/apt/lists/*

# Create non-root user for build
RUN useradd -m -u 10001 builder
USER builder
WORKDIR /app

# Copy dependency manifests and build dependencies first (caching layer)
COPY --chown=builder:builder Cargo.toml Cargo.lock ./
RUN mkdir src && echo "fn main() {}" > src/main.rs
RUN cargo build --release --locked && rm -rf src

# Copy source code and build application
COPY --chown=builder:builder . .
RUN touch src/main.rs && cargo build --release --locked

# Strip debug symbols to reduce binary size
RUN strip target/release/llm-research-lab

#############################################
# Stage 2: Runtime environment
#############################################
FROM debian:bookworm-slim

# Install runtime dependencies only
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    libpq5 \
    curl \
    && rm -rf /var/lib/apt/lists/*

# Create non-root user for runtime
RUN useradd -m -u 10001 -s /bin/bash appuser

# Copy application binary from builder
COPY --from=builder /app/target/release/llm-research-lab /usr/local/bin/llm-research-lab

# Set ownership and permissions
RUN chown appuser:appuser /usr/local/bin/llm-research-lab \
    && chmod 755 /usr/local/bin/llm-research-lab

# Create application directories
RUN mkdir -p /app/config /app/logs \
    && chown -R appuser:appuser /app

# Switch to non-root user
USER appuser
WORKDIR /app

# Health check
HEALTHCHECK --interval=30s --timeout=5s --start-period=10s --retries=3 \
  CMD curl -f http://localhost:8080/health || exit 1

# Expose application port
EXPOSE 8080

# Run application
ENTRYPOINT ["/usr/local/bin/llm-research-lab"]
CMD ["--config", "/app/config/config.yaml"]
```

**Image Build Commands**:

```bash
# Build image with BuildKit for optimal caching
DOCKER_BUILDKIT=1 docker build \
  --tag llm-research-lab:v1.0.0 \
  --tag llm-research-lab:latest \
  --build-arg RUST_VERSION=1.75 \
  --file Dockerfile \
  .

# Scan image for vulnerabilities
trivy image --severity CRITICAL,HIGH llm-research-lab:v1.0.0

# Inspect image layers and size
docker images llm-research-lab:v1.0.0
docker history llm-research-lab:v1.0.0

# Push to registry
docker tag llm-research-lab:v1.0.0 gcr.io/my-project/llm-research-lab:v1.0.0
docker push gcr.io/my-project/llm-research-lab:v1.0.0
```

**Image Optimization**:

| Optimization | Benefit | Implementation |
|--------------|---------|----------------|
| Multi-stage build | Reduced image size | Separate builder and runtime stages |
| Dependency caching | Faster builds | Copy Cargo.toml/Cargo.lock before source |
| Binary stripping | Smaller image | `strip target/release/llm-research-lab` |
| Minimal base image | Reduced attack surface | debian:bookworm-slim (no unnecessary tools) |
| Non-root user | Security hardening | Run as UID 10001 |
| Layer optimization | Better caching | Group RUN commands, clean apt cache |

**Expected Image Size**: ~150-200MB (vs ~2GB without optimization)

---

### 3.3.2 Helm Chart Requirements

**Objective**: Define declarative Kubernetes deployment manifests with environment-specific overrides.

**Helm Chart Structure**:

```
helm/llm-research-lab/
├── Chart.yaml                 # Chart metadata
├── values.yaml                # Default values
├── values.production.yaml     # Production overrides
├── values.staging.yaml        # Staging overrides
├── templates/
│   ├── _helpers.tpl           # Template helpers
│   ├── deployment.yaml        # Application Deployment
│   ├── service.yaml           # Service definition
│   ├── ingress.yaml           # Ingress configuration
│   ├── configmap.yaml         # Application configuration
│   ├── secret.yaml            # Secret references (External Secrets)
│   ├── hpa.yaml               # HorizontalPodAutoscaler
│   ├── pdb.yaml               # PodDisruptionBudget
│   ├── servicemonitor.yaml    # Prometheus ServiceMonitor
│   └── networkpolicy.yaml     # NetworkPolicy
└── README.md                  # Chart documentation
```

**Chart.yaml**:

```yaml
apiVersion: v2
name: llm-research-lab
description: Helm chart for LLM Research Lab application
type: application
version: 1.0.0
appVersion: "1.0.0"
keywords:
  - llm
  - research
  - experiment-tracking
maintainers:
  - name: Platform Team
    email: platform@example.com
dependencies: []
```

**values.production.yaml** (Production Overrides):

```yaml
# Production configuration for LLM Research Lab

replicaCount: 5

image:
  repository: gcr.io/my-project/llm-research-lab
  pullPolicy: IfNotPresent
  tag: "v1.0.0"

imagePullSecrets:
  - name: gcr-json-key

nameOverride: ""
fullnameOverride: "llm-research-lab"

serviceAccount:
  create: true
  annotations:
    iam.gke.io/gcp-service-account: "llm-research-lab@my-project.iam.gserviceaccount.com"
  name: "llm-research-lab"

podAnnotations:
  prometheus.io/scrape: "true"
  prometheus.io/port: "8081"
  prometheus.io/path: "/metrics"

podSecurityContext:
  runAsNonRoot: true
  runAsUser: 10001
  fsGroup: 10001
  seccompProfile:
    type: RuntimeDefault

securityContext:
  allowPrivilegeEscalation: false
  capabilities:
    drop:
      - ALL
  readOnlyRootFilesystem: true

service:
  type: ClusterIP
  port: 80
  targetPort: 8080
  annotations:
    cloud.google.com/neg: '{"ingress": true}'

ingress:
  enabled: true
  className: "nginx"
  annotations:
    cert-manager.io/cluster-issuer: "letsencrypt-prod"
    nginx.ingress.kubernetes.io/ssl-protocols: "TLSv1.3"
    nginx.ingress.kubernetes.io/ssl-ciphers: "TLS_AES_128_GCM_SHA256:TLS_AES_256_GCM_SHA384"
  hosts:
    - host: api.llm-research-lab.example.com
      paths:
        - path: /
          pathType: Prefix
  tls:
    - secretName: llm-research-lab-tls
      hosts:
        - api.llm-research-lab.example.com

resources:
  limits:
    cpu: 2000m
    memory: 4Gi
  requests:
    cpu: 1000m
    memory: 2Gi

autoscaling:
  enabled: true
  minReplicas: 5
  maxReplicas: 20
  targetCPUUtilizationPercentage: 70
  targetMemoryUtilizationPercentage: 80

nodeSelector:
  workload: application

tolerations: []

affinity:
  podAntiAffinity:
    preferredDuringSchedulingIgnoredDuringExecution:
      - weight: 100
        podAffinityTerm:
          labelSelector:
            matchExpressions:
              - key: app.kubernetes.io/name
                operator: In
                values:
                  - llm-research-lab
          topologyKey: kubernetes.io/hostname

livenessProbe:
  httpGet:
    path: /health/liveness
    port: 8080
  initialDelaySeconds: 30
  periodSeconds: 10
  timeoutSeconds: 5
  failureThreshold: 3

readinessProbe:
  httpGet:
    path: /health/readiness
    port: 8080
  initialDelaySeconds: 10
  periodSeconds: 5
  timeoutSeconds: 3
  failureThreshold: 3

# Environment-specific configuration
config:
  log_level: "info"
  database_url: "postgres://llm-research-lab:PASSWORD@postgres-service:5432/llm_research_lab?sslmode=require"
  clickhouse_url: "http://clickhouse-service:8123"
  redis_url: "redis://redis-sentinel:26379/0"
  s3_bucket: "llm-research-lab-artifacts"
  s3_region: "us-west-2"

# External Secrets Operator integration
externalSecrets:
  enabled: true
  secretStore:
    name: "aws-secrets-manager"
    kind: "ClusterSecretStore"
  secrets:
    - name: "postgresql-credentials"
      remoteRef:
        key: "production/llm-research-lab/postgresql"
    - name: "clickhouse-credentials"
      remoteRef:
        key: "production/llm-research-lab/clickhouse"

podDisruptionBudget:
  enabled: true
  minAvailable: 3

networkPolicy:
  enabled: true
  policyTypes:
    - Ingress
    - Egress
  ingress:
    - from:
        - namespaceSelector:
            matchLabels:
              name: ingress-nginx
      ports:
        - protocol: TCP
          port: 8080
  egress:
    - to:
        - namespaceSelector: {}
          podSelector:
            matchLabels:
              component: postgresql
      ports:
        - protocol: TCP
          port: 5432
    - to:
        - namespaceSelector: {}
          podSelector:
            matchLabels:
              component: clickhouse
      ports:
        - protocol: TCP
          port: 9000
```

**Helm Deployment Commands**:

```bash
# Validate chart
helm lint helm/llm-research-lab
helm template llm-research-lab helm/llm-research-lab \
  -f helm/values.production.yaml \
  --validate

# Dry-run deployment
helm upgrade --install llm-research-lab helm/llm-research-lab \
  -f helm/values.production.yaml \
  --namespace llm-research-lab \
  --create-namespace \
  --dry-run --debug

# Deploy to production
helm upgrade --install llm-research-lab helm/llm-research-lab \
  -f helm/values.production.yaml \
  --namespace llm-research-lab \
  --create-namespace \
  --wait --timeout 10m

# Verify deployment
kubectl get pods -n llm-research-lab
kubectl rollout status deployment/llm-research-lab -n llm-research-lab
```

---

### 3.3.3 ConfigMaps and Secrets Structure

**Objective**: Manage application configuration and sensitive data securely and declaratively.

**ConfigMap Example** (`templates/configmap.yaml`):

```yaml
apiVersion: v1
kind: ConfigMap
metadata:
  name: {{ include "llm-research-lab.fullname" . }}-config
  labels:
    {{- include "llm-research-lab.labels" . | nindent 4 }}
data:
  config.yaml: |
    server:
      host: "0.0.0.0"
      port: 8080
      metrics_port: 8081
      shutdown_timeout_secs: 30

    logging:
      level: {{ .Values.config.log_level | quote }}
      format: "json"
      output: "stdout"

    database:
      max_connections: 20
      min_connections: 5
      connection_timeout_secs: 30
      idle_timeout_secs: 600
      max_lifetime_secs: 1800

    clickhouse:
      max_connections: 10
      query_timeout_secs: 60
      compression: "lz4"

    redis:
      max_connections: 10
      connection_timeout_secs: 5
      command_timeout_secs: 10

    s3:
      region: {{ .Values.config.s3_region | quote }}
      bucket: {{ .Values.config.s3_bucket | quote }}
      endpoint: null  # Use default AWS endpoint

    observability:
      metrics_enabled: true
      tracing_enabled: true
      tracing_endpoint: "http://jaeger-collector:14268/api/traces"
      tracing_sample_rate: 0.1

    integrations:
      llm_registry_url: "https://registry.llm-devops.example.com"
      llm_data_vault_url: "https://data-vault.llm-devops.example.com"
      llm_analytics_hub_url: "https://analytics.llm-devops.example.com"
      llm_test_bench_url: "https://test-bench.llm-devops.example.com"
```

**External Secret Example** (`templates/secret.yaml`):

```yaml
{{- if .Values.externalSecrets.enabled }}
apiVersion: external-secrets.io/v1beta1
kind: ExternalSecret
metadata:
  name: {{ include "llm-research-lab.fullname" . }}-secrets
  labels:
    {{- include "llm-research-lab.labels" . | nindent 4 }}
spec:
  secretStoreRef:
    name: {{ .Values.externalSecrets.secretStore.name }}
    kind: {{ .Values.externalSecrets.secretStore.kind }}

  target:
    name: {{ include "llm-research-lab.fullname" . }}-secrets
    creationPolicy: Owner

  dataFrom:
    - extract:
        key: production/llm-research-lab/postgresql
    - extract:
        key: production/llm-research-lab/clickhouse
    - extract:
        key: production/llm-research-lab/api-keys

  refreshInterval: 1h
{{- end }}
```

**Secrets Consumption in Deployment** (`templates/deployment.yaml` excerpt):

```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: {{ include "llm-research-lab.fullname" . }}
spec:
  template:
    spec:
      containers:
        - name: {{ .Chart.Name }}
          image: "{{ .Values.image.repository }}:{{ .Values.image.tag | default .Chart.AppVersion }}"
          env:
            # Non-sensitive configuration from ConfigMap
            - name: CONFIG_FILE
              value: "/app/config/config.yaml"

            # Sensitive credentials from External Secrets
            - name: DATABASE_URL
              valueFrom:
                secretKeyRef:
                  name: {{ include "llm-research-lab.fullname" . }}-secrets
                  key: postgresql-url

            - name: CLICKHOUSE_PASSWORD
              valueFrom:
                secretKeyRef:
                  name: {{ include "llm-research-lab.fullname" . }}-secrets
                  key: clickhouse-password

            - name: LLM_REGISTRY_API_KEY
              valueFrom:
                secretKeyRef:
                  name: {{ include "llm-research-lab.fullname" . }}-secrets
                  key: registry-api-key

          volumeMounts:
            - name: config
              mountPath: /app/config
              readOnly: true

            - name: tmp
              mountPath: /tmp

      volumes:
        - name: config
          configMap:
            name: {{ include "llm-research-lab.fullname" . }}-config

        - name: tmp
          emptyDir: {}
```

**Configuration Management Best Practices**:

| Category | Best Practice | Implementation |
|----------|---------------|----------------|
| Secrets Storage | Never commit secrets to Git | Use External Secrets Operator |
| Secret Rotation | Automate rotation | AWS Secrets Manager auto-rotation |
| Configuration Versioning | Track config changes | ConfigMaps versioned with Helm releases |
| Environment Separation | Separate configs per environment | values.production.yaml vs values.staging.yaml |
| Least Privilege | Minimal secret access | RBAC policies limit secret access |
| Audit Logging | Log all secret access | Cloud provider audit logs enabled |

---

### 3.3.4 Database Migration Strategy

**Objective**: Manage database schema evolution safely with versioned, reversible migrations.

**Migration Framework**: `sqlx` (Rust-native SQL toolkit with compile-time verification)

**Migration Directory Structure**:

```
migrations/
├── 001_initial_schema.sql
├── 002_add_experiment_metadata.sql
├── 003_add_dataset_versioning.sql
└── 004_add_metrics_benchmarking.sql
```

**Migration Example** (`migrations/001_initial_schema.sql`):

```sql
-- Initial schema for LLM Research Lab
-- Version: 1.0.0
-- Applied: 2025-12-01

-- Enable UUID extension
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

-- Experiments table
CREATE TABLE experiments (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    name VARCHAR(255) NOT NULL,
    description TEXT,
    status VARCHAR(50) NOT NULL DEFAULT 'pending',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    created_by VARCHAR(255) NOT NULL,
    metadata JSONB NOT NULL DEFAULT '{}',

    CONSTRAINT valid_status CHECK (status IN ('pending', 'running', 'completed', 'failed', 'cancelled'))
);

CREATE INDEX idx_experiments_status ON experiments(status);
CREATE INDEX idx_experiments_created_at ON experiments(created_at DESC);
CREATE INDEX idx_experiments_created_by ON experiments(created_by);

-- Datasets table
CREATE TABLE datasets (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    name VARCHAR(255) NOT NULL,
    version VARCHAR(50) NOT NULL,
    content_hash VARCHAR(64) NOT NULL UNIQUE,  -- SHA-256 hash
    storage_uri TEXT NOT NULL,
    size_bytes BIGINT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    metadata JSONB NOT NULL DEFAULT '{}',

    UNIQUE(name, version)
);

CREATE INDEX idx_datasets_content_hash ON datasets(content_hash);
CREATE INDEX idx_datasets_name_version ON datasets(name, version);

-- Metrics table
CREATE TABLE metrics (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    experiment_id UUID NOT NULL REFERENCES experiments(id) ON DELETE CASCADE,
    metric_name VARCHAR(255) NOT NULL,
    metric_value DOUBLE PRECISION NOT NULL,
    timestamp TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    metadata JSONB NOT NULL DEFAULT '{}'
);

CREATE INDEX idx_metrics_experiment_id ON metrics(experiment_id);
CREATE INDEX idx_metrics_metric_name ON metrics(metric_name);
CREATE INDEX idx_metrics_timestamp ON metrics(timestamp DESC);

-- Experiment-Dataset associations
CREATE TABLE experiment_datasets (
    experiment_id UUID NOT NULL REFERENCES experiments(id) ON DELETE CASCADE,
    dataset_id UUID NOT NULL REFERENCES datasets(id) ON DELETE CASCADE,
    role VARCHAR(50) NOT NULL,  -- 'train', 'validation', 'test'
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    PRIMARY KEY (experiment_id, dataset_id, role)
);

-- Audit log table
CREATE TABLE audit_log (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    table_name VARCHAR(255) NOT NULL,
    record_id UUID NOT NULL,
    action VARCHAR(50) NOT NULL,  -- 'INSERT', 'UPDATE', 'DELETE'
    old_values JSONB,
    new_values JSONB,
    changed_by VARCHAR(255) NOT NULL,
    changed_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_audit_log_table_record ON audit_log(table_name, record_id);
CREATE INDEX idx_audit_log_changed_at ON audit_log(changed_at DESC);

-- Updated_at trigger function
CREATE OR REPLACE FUNCTION update_updated_at_column()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- Apply updated_at trigger to experiments
CREATE TRIGGER update_experiments_updated_at
    BEFORE UPDATE ON experiments
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();
```

**Migration Execution Strategy**:

```yaml
migration_execution:
  tool: "sqlx-cli"

  pre_deployment:
    - action: "Backup database"
      command: "pg_dump -h $POSTGRES_HOST -U $POSTGRES_USER $POSTGRES_DB > backup-$(date +%Y%m%d-%H%M%S).sql"
      retention: "30 days"

    - action: "Validate migrations"
      command: "sqlx migrate info --database-url $DATABASE_URL"
      validation: "Ensure no pending failed migrations"

    - action: "Run migrations (dry-run)"
      command: "sqlx migrate run --database-url $DATABASE_URL --dry-run"
      validation: "Review SQL statements to be executed"

  deployment:
    - action: "Run migrations"
      command: "sqlx migrate run --database-url $DATABASE_URL"
      timeout: "10 minutes"
      validation: "All migrations applied successfully"

    - action: "Verify schema"
      command: "sqlx database check --database-url $DATABASE_URL"
      validation: "Schema matches migration history"

  post_deployment:
    - action: "Verify data integrity"
      queries:
        - "SELECT COUNT(*) FROM experiments;"
        - "SELECT COUNT(*) FROM datasets;"
      validation: "Record counts match expectations"

    - action: "Test application connectivity"
      command: "curl -f http://llm-research-lab-service:8080/health/db"
      validation: "Database health check passes"

  rollback_strategy:
    approach: "Backward-compatible migrations only"
    process:
      - "Write additive migrations (add columns with defaults, create tables)"
      - "Avoid breaking changes (drop column/table in separate release)"
      - "Multi-phase migrations for schema changes"

    rollback_procedure:
      - "Restore from backup if migration fails"
      - "Revert to previous application version"
      - "Document rollback in incident report"
```

**Migration Automation in CI/CD**:

```yaml
# .github/workflows/deploy-production.yaml (excerpt)
jobs:
  deploy:
    steps:
      - name: Backup database
        run: |
          pg_dump -h ${{ secrets.POSTGRES_HOST }} \
                  -U ${{ secrets.POSTGRES_USER }} \
                  ${{ secrets.POSTGRES_DB }} > backup.sql
          aws s3 cp backup.sql s3://llm-research-lab-backups/$(date +%Y%m%d-%H%M%S).sql

      - name: Run database migrations
        run: |
          sqlx migrate run --database-url ${{ secrets.DATABASE_URL }}
        timeout-minutes: 10

      - name: Deploy application
        run: |
          helm upgrade --install llm-research-lab helm/llm-research-lab \
            -f helm/values.production.yaml \
            --namespace llm-research-lab \
            --wait --timeout 10m

      - name: Verify deployment
        run: |
          kubectl rollout status deployment/llm-research-lab -n llm-research-lab
          curl -f https://api.llm-research-lab.example.com/health
```

**Migration Best Practices**:

| Practice | Rationale | Implementation |
|----------|-----------|----------------|
| Backward Compatibility | Enable zero-downtime deployments | Add columns with defaults; avoid dropping columns |
| Idempotency | Safe to re-run migrations | Use `IF NOT EXISTS`, `CREATE OR REPLACE` |
| Transactional | Atomic migration application | Wrap in `BEGIN; ... COMMIT;` |
| Versioned | Track schema evolution | Sequential numbered migrations |
| Tested | Prevent production failures | Run migrations in staging first |
| Documented | Knowledge transfer | Comment migrations with purpose and impact |

---

## Summary

This section covered the critical deployment readiness requirements for LLM-Research-Lab:

**3.1 Pre-Deployment Checklist**: Code freeze verification, quality gates (85% coverage, 70% mutation score, zero critical vulns), documentation completeness, release notes, and stakeholder approvals.

**3.2 Environment Certification**: Production environment validation (Kubernetes, PostgreSQL, ClickHouse, Redis, S3), Infrastructure-as-Code verification (Terraform, Helm, Kustomize), secrets management audit (External Secrets Operator, rotation policies), and network security validation (network policies, TLS 1.3, mTLS).

**3.3 Deployment Artifacts**: Multi-stage Dockerfile (optimized to ~150MB), Helm chart with production values (HPA, PodDisruptionBudget, NetworkPolicy), ConfigMaps/Secrets structure (External Secrets Operator integration), and database migration strategy (sqlx with backward-compatible migrations).

**Key Deliverables**:
- Production-ready container image (<200MB, security-hardened)
- Helm chart with comprehensive production configuration
- Automated database migration pipeline
- Validated production infrastructure
- Stakeholder sign-off on deployment readiness

**Next Steps**: Proceed to Section 4 (Operational Handoff) for knowledge transfer and runbook documentation.

---

**Document Metadata**:

| Field | Value |
|-------|-------|
| **Version** | 1.0.0 |
| **Status** | Specification |
| **SPARC Phase** | Phase 5: Completion |
| **Created** | 2025-11-28 |
| **Ecosystem** | LLM DevOps (24+ Module Platform) |
| **Authors** | Platform Engineering Team |

---

*This specification follows the SPARC methodology (Specification → Pseudocode → Architecture → Refinement → Completion). Phase 5 Section 3 covers deployment readiness for enterprise Kubernetes environments.*
