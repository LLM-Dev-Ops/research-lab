# LLM Research Lab - Kubernetes Deployment

Complete Kubernetes deployment manifests for the LLM Research Lab platform.

## Table of Contents

- [Overview](#overview)
- [Prerequisites](#prerequisites)
- [Quick Start](#quick-start)
- [Configuration](#configuration)
- [Deployment](#deployment)
- [Monitoring](#monitoring)
- [Scaling](#scaling)
- [Troubleshooting](#troubleshooting)
- [Production Considerations](#production-considerations)

## Overview

This directory contains production-ready Kubernetes manifests for deploying the LLM Research Lab application along with its dependencies:

- **Application**: LLM Research Lab API (3 replicas with HPA)
- **Database**: PostgreSQL 16 (StatefulSet)
- **Analytics**: ClickHouse 24 (StatefulSet)
- **Object Storage**: MinIO (S3-compatible)
- **Cache**: Redis 7

## Prerequisites

### Required Tools

```bash
# Kubernetes cluster (1.24+)
kubectl version --client

# Kustomize (optional, but recommended)
kustomize version

# Helm (for cert-manager installation)
helm version
```

### Cluster Requirements

- **Kubernetes Version**: 1.24 or higher
- **Minimum Resources**:
  - CPU: 8 cores
  - Memory: 16 GB
  - Storage: 200 GB
- **Ingress Controller**: Nginx or Traefik
- **Storage Class**: Default or custom (configure in PVCs)
- **Metrics Server**: For HPA functionality

### Optional Components

- **cert-manager**: For automatic TLS certificate management
- **Prometheus**: For metrics collection
- **Grafana**: For metrics visualization

## Quick Start

### 1. Install Prerequisites

```bash
# Install metrics-server (required for HPA)
kubectl apply -f https://github.com/kubernetes-sigs/metrics-server/releases/latest/download/components.yaml

# Install cert-manager (optional, for TLS)
kubectl apply -f https://github.com/cert-manager/cert-manager/releases/download/v1.13.0/cert-manager.yaml

# Install nginx-ingress-controller (if not already installed)
helm repo add ingress-nginx https://kubernetes.github.io/ingress-nginx
helm repo update
helm install nginx-ingress ingress-nginx/ingress-nginx
```

### 2. Configure Secrets

**IMPORTANT**: Before deploying to production, update all secrets in `secrets.yaml`:

```bash
# Generate a strong JWT secret
echo -n "$(openssl rand -base64 32)" | base64

# Update secrets.yaml with the generated value
# Replace all placeholder values with actual production secrets
```

### 3. Deploy with kubectl

```bash
# Deploy all resources
kubectl apply -f namespace.yaml
kubectl apply -f configmap.yaml
kubectl apply -f secrets.yaml
kubectl apply -f postgres-statefulset.yaml
kubectl apply -f clickhouse-statefulset.yaml
kubectl apply -f minio-deployment.yaml
kubectl apply -f redis-deployment.yaml
kubectl apply -f deployment.yaml
kubectl apply -f service.yaml
kubectl apply -f ingress.yaml
kubectl apply -f hpa.yaml
kubectl apply -f pdb.yaml

# Wait for all pods to be ready
kubectl wait --for=condition=ready pod -l app.kubernetes.io/name=llm-research-lab -n llm-research-lab --timeout=300s
```

### 4. Deploy with Kustomize (Recommended)

```bash
# Apply all resources using kustomize
kubectl apply -k .

# Or build and pipe to kubectl
kustomize build . | kubectl apply -f -
```

### 5. Verify Deployment

```bash
# Check all resources
kubectl get all -n llm-research-lab

# Check pod status
kubectl get pods -n llm-research-lab

# Check logs
kubectl logs -n llm-research-lab -l app.kubernetes.io/name=llm-research-lab --tail=100

# Check HPA status
kubectl get hpa -n llm-research-lab

# Check PVC status
kubectl get pvc -n llm-research-lab
```

## Configuration

### Environment-Specific Configuration

Create overlays for different environments:

```bash
mkdir -p overlays/{dev,staging,production}

# overlays/production/kustomization.yaml
cat <<EOF > overlays/production/kustomization.yaml
apiVersion: kustomize.config.k8s.io/v1beta1
kind: Kustomization

bases:
  - ../../base

namespace: llm-research-lab-prod

images:
  - name: llm-research-lab
    newName: your-registry.io/llm-research-lab
    newTag: v1.0.0

replicas:
  - name: llm-research-lab
    count: 5

patches:
  - path: production-resources.yaml
EOF

# Deploy production environment
kubectl apply -k overlays/production
```

### ConfigMap Updates

```bash
# Edit ConfigMap
kubectl edit configmap llm-research-lab-config -n llm-research-lab

# Or update from file
kubectl create configmap llm-research-lab-config \
  --from-file=config/ \
  --dry-run=client -o yaml | kubectl apply -f -
```

### Secret Management

For production, use one of these secret management solutions:

1. **Sealed Secrets**:
```bash
kubeseal --format=yaml < secrets.yaml > sealed-secrets.yaml
kubectl apply -f sealed-secrets.yaml
```

2. **External Secrets Operator**:
```yaml
apiVersion: external-secrets.io/v1beta1
kind: ExternalSecret
metadata:
  name: llm-research-lab-secrets
spec:
  secretStoreRef:
    name: aws-secretsmanager
    kind: SecretStore
  target:
    name: llm-research-lab-secrets
  data:
    - secretKey: JWT_SECRET
      remoteRef:
        key: llm-research-lab/jwt-secret
```

## Deployment

### Building and Pushing Docker Image

```bash
# Build image
docker build -t llm-research-lab:latest .

# Tag for your registry
docker tag llm-research-lab:latest your-registry.io/llm-research-lab:v1.0.0

# Push to registry
docker push your-registry.io/llm-research-lab:v1.0.0

# Update deployment with new image
kubectl set image deployment/llm-research-lab \
  llm-research-lab=your-registry.io/llm-research-lab:v1.0.0 \
  -n llm-research-lab
```

### Rolling Updates

```bash
# Update deployment
kubectl rollout status deployment/llm-research-lab -n llm-research-lab

# Check rollout history
kubectl rollout history deployment/llm-research-lab -n llm-research-lab

# Rollback if needed
kubectl rollout undo deployment/llm-research-lab -n llm-research-lab

# Rollback to specific revision
kubectl rollout undo deployment/llm-research-lab --to-revision=2 -n llm-research-lab
```

### Database Migrations

```bash
# Run migrations in a Job
kubectl create job --from=cronjob/db-migrate db-migrate-manual-1 -n llm-research-lab

# Or exec into a pod
kubectl exec -it deployment/llm-research-lab -n llm-research-lab -- /app/llm-research-lab migrate
```

## Monitoring

### Access Logs

```bash
# Stream logs from all pods
kubectl logs -n llm-research-lab -l app.kubernetes.io/name=llm-research-lab -f

# Logs from specific pod
kubectl logs -n llm-research-lab <pod-name> -f

# Previous pod logs (after crash)
kubectl logs -n llm-research-lab <pod-name> --previous
```

### Health Checks

```bash
# Check health endpoint
kubectl port-forward -n llm-research-lab service/llm-research-lab-service 8080:80
curl http://localhost:8080/health

# Check all endpoints
kubectl get endpoints -n llm-research-lab
```

### Metrics

```bash
# Get HPA metrics
kubectl get hpa -n llm-research-lab -w

# Get resource usage
kubectl top pods -n llm-research-lab
kubectl top nodes
```

## Scaling

### Manual Scaling

```bash
# Scale deployment
kubectl scale deployment/llm-research-lab --replicas=5 -n llm-research-lab

# Scale StatefulSet (PostgreSQL)
kubectl scale statefulset/postgres --replicas=1 -n llm-research-lab
```

### Auto-Scaling (HPA)

The HPA is configured to scale between 3-10 replicas based on:
- CPU utilization (target: 70%)
- Memory utilization (target: 80%)

```bash
# View HPA status
kubectl describe hpa llm-research-lab-hpa -n llm-research-lab

# Update HPA
kubectl edit hpa llm-research-lab-hpa -n llm-research-lab
```

### Vertical Scaling (Resources)

```bash
# Update resource limits
kubectl set resources deployment/llm-research-lab \
  --limits=cpu=2000m,memory=2Gi \
  --requests=cpu=1000m,memory=1Gi \
  -n llm-research-lab
```

## Troubleshooting

### Common Issues

#### Pods Not Starting

```bash
# Describe pod to see events
kubectl describe pod <pod-name> -n llm-research-lab

# Check events
kubectl get events -n llm-research-lab --sort-by='.lastTimestamp'

# Check logs
kubectl logs <pod-name> -n llm-research-lab --previous
```

#### Database Connection Issues

```bash
# Test PostgreSQL connection
kubectl run -it --rm debug --image=postgres:16-alpine --restart=Never -n llm-research-lab -- \
  psql postgres://llmlab:llmlab_password@postgres-service:5432/llm_research

# Test ClickHouse connection
kubectl run -it --rm debug --image=clickhouse/clickhouse-client --restart=Never -n llm-research-lab -- \
  clickhouse-client --host clickhouse-service --port 9000
```

#### Storage Issues

```bash
# Check PVC status
kubectl get pvc -n llm-research-lab

# Describe PVC
kubectl describe pvc <pvc-name> -n llm-research-lab

# Check storage class
kubectl get storageclass
```

#### Ingress Issues

```bash
# Check ingress
kubectl describe ingress llm-research-lab-ingress -n llm-research-lab

# Check ingress controller logs
kubectl logs -n ingress-nginx -l app.kubernetes.io/name=ingress-nginx

# Test service directly
kubectl port-forward -n llm-research-lab service/llm-research-lab-service 8080:80
```

### Debug Commands

```bash
# Shell into pod
kubectl exec -it <pod-name> -n llm-research-lab -- /bin/sh

# Run debug container
kubectl debug <pod-name> -n llm-research-lab -it --image=busybox

# Port forward for local testing
kubectl port-forward -n llm-research-lab deployment/llm-research-lab 8080:8080
```

## Production Considerations

### Security

1. **Update all default secrets** in `secrets.yaml`
2. **Use a secret management solution** (Vault, Sealed Secrets, External Secrets)
3. **Enable network policies** to restrict pod-to-pod communication
4. **Use Pod Security Standards** (PSS) or Pod Security Policies (PSP)
5. **Scan images** for vulnerabilities before deployment
6. **Enable audit logging** on the cluster
7. **Use RBAC** to restrict access

### High Availability

1. **Multi-zone deployment**: Spread pods across availability zones
2. **Database replication**: Configure PostgreSQL streaming replication
3. **Backup strategy**: Regular automated backups of PostgreSQL and ClickHouse
4. **Disaster recovery**: Document and test recovery procedures
5. **Pod disruption budgets**: Ensure minimum availability during updates

### Performance

1. **Resource tuning**: Adjust CPU/memory based on actual usage
2. **Database optimization**: Tune PostgreSQL and ClickHouse configurations
3. **Connection pooling**: Configure appropriate pool sizes
4. **Caching strategy**: Optimize Redis cache TTLs
5. **CDN**: Use CDN for static assets

### Monitoring & Alerting

1. **Prometheus**: Collect metrics from all components
2. **Grafana**: Create dashboards for visualization
3. **Alertmanager**: Configure alerts for critical issues
4. **Logging**: Centralized logging with ELK/Loki
5. **Tracing**: Distributed tracing with Jaeger/Tempo

### Cost Optimization

1. **Right-size resources**: Use VPA for recommendations
2. **Use spot instances**: For non-critical workloads
3. **Storage optimization**: Use appropriate storage classes
4. **Auto-scaling**: Configure HPA to scale down during low traffic
5. **Resource cleanup**: Remove unused resources regularly

### Backup & Recovery

```bash
# PostgreSQL backup
kubectl exec -n llm-research-lab postgres-0 -- \
  pg_dump -U llmlab llm_research | gzip > backup-$(date +%Y%m%d).sql.gz

# Restore PostgreSQL
gunzip -c backup-20231201.sql.gz | \
  kubectl exec -i -n llm-research-lab postgres-0 -- \
  psql -U llmlab llm_research

# ClickHouse backup
kubectl exec -n llm-research-lab clickhouse-0 -- \
  clickhouse-client --query="BACKUP DATABASE llm_research_analytics TO '/backups/backup-$(date +%Y%m%d)'"
```

## Additional Resources

- [Kubernetes Documentation](https://kubernetes.io/docs/)
- [Kustomize Documentation](https://kustomize.io/)
- [PostgreSQL on Kubernetes](https://www.postgresql.org/docs/current/high-availability.html)
- [ClickHouse Operations](https://clickhouse.com/docs/en/operations/)
- [MinIO Kubernetes](https://min.io/docs/minio/kubernetes/)

## Support

For issues or questions:
- Create an issue in the repository
- Check the troubleshooting section
- Review pod logs and events
- Consult Kubernetes documentation
