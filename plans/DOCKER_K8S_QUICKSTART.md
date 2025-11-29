# Docker & Kubernetes Quick Start Guide

Quick reference for deploying LLM Research Lab with Docker and Kubernetes.

## Docker Compose - Local Development

### Start Everything
```bash
# Start all services
docker-compose up -d

# View logs
docker-compose logs -f

# Check status
docker-compose ps

# Access application
curl http://localhost:8080/health
```

### Services & Ports
- **Application**: http://localhost:8080
- **PostgreSQL**: localhost:5432 (user: llmlab, pass: llmlab_password, db: llm_research)
- **ClickHouse**: http://localhost:8123
- **MinIO Console**: http://localhost:9001 (admin: minioadmin/minioadmin)
- **Redis**: localhost:6379

### Common Commands
```bash
# Rebuild application
docker-compose build llm-research-lab

# Restart service
docker-compose restart llm-research-lab

# View specific logs
docker-compose logs -f postgres

# Execute command
docker-compose exec llm-research-lab /bin/sh
docker-compose exec postgres psql -U llmlab -d llm_research

# Stop everything
docker-compose down

# Stop and remove volumes (DELETES ALL DATA)
docker-compose down -v
```

### Database Operations
```bash
# Access PostgreSQL
docker-compose exec postgres psql -U llmlab -d llm_research

# Backup database
docker-compose exec postgres pg_dump -U llmlab llm_research > backup.sql

# Restore database
docker-compose exec -T postgres psql -U llmlab -d llm_research < backup.sql

# Access ClickHouse
docker-compose exec clickhouse clickhouse-client --user llmlab --password llmlab_password
```

## Kubernetes - Production Deployment

### Prerequisites
```bash
# Install kubectl
curl -LO "https://dl.k8s.io/release/$(curl -L -s https://dl.k8s.io/release/stable.txt)/bin/linux/amd64/kubectl"
chmod +x kubectl
sudo mv kubectl /usr/local/bin/

# Install kustomize (optional)
curl -s "https://raw.githubusercontent.com/kubernetes-sigs/kustomize/master/hack/install_kustomize.sh" | bash
sudo mv kustomize /usr/local/bin/

# Verify cluster access
kubectl cluster-info
kubectl get nodes
```

### Quick Deploy
```bash
cd k8s

# IMPORTANT: Update secrets first!
vim secrets.yaml

# Deploy with kustomize
kubectl apply -k .

# Or deploy with kubectl
kubectl apply -f namespace.yaml
kubectl apply -f configmap.yaml
kubectl apply -f secrets.yaml
kubectl apply -f .

# Check status
kubectl get all -n llm-research-lab

# Wait for ready
kubectl wait --for=condition=available --timeout=300s \
  deployment/llm-research-lab -n llm-research-lab
```

### Access Application
```bash
# Port forward to local machine
kubectl port-forward -n llm-research-lab service/llm-research-lab-service 8080:80

# Access at http://localhost:8080
curl http://localhost:8080/health
```

### Common Operations
```bash
# View logs
kubectl logs -n llm-research-lab -l app.kubernetes.io/name=llm-research-lab -f

# Get pod status
kubectl get pods -n llm-research-lab

# Describe deployment
kubectl describe deployment/llm-research-lab -n llm-research-lab

# Shell into pod
kubectl exec -it -n llm-research-lab deployment/llm-research-lab -- /bin/sh

# View events
kubectl get events -n llm-research-lab --sort-by='.lastTimestamp'

# Check HPA status
kubectl get hpa -n llm-research-lab

# Check PVC status
kubectl get pvc -n llm-research-lab
```

### Scaling
```bash
# Manual scale
kubectl scale deployment/llm-research-lab --replicas=5 -n llm-research-lab

# View HPA metrics
kubectl get hpa -n llm-research-lab -w

# Check resource usage
kubectl top pods -n llm-research-lab
```

### Updates & Rollbacks
```bash
# Update image
kubectl set image deployment/llm-research-lab \
  llm-research-lab=your-registry.io/llm-research-lab:v1.1.0 \
  -n llm-research-lab

# Check rollout status
kubectl rollout status deployment/llm-research-lab -n llm-research-lab

# View rollout history
kubectl rollout history deployment/llm-research-lab -n llm-research-lab

# Rollback
kubectl rollout undo deployment/llm-research-lab -n llm-research-lab

# Rollback to specific revision
kubectl rollout undo deployment/llm-research-lab --to-revision=2 -n llm-research-lab
```

### Database Access
```bash
# PostgreSQL
kubectl exec -it -n llm-research-lab postgres-0 -- \
  psql -U llmlab -d llm_research

# ClickHouse
kubectl exec -it -n llm-research-lab clickhouse-0 -- \
  clickhouse-client --user llmlab --password llmlab_password

# Redis
kubectl exec -it -n llm-research-lab redis-0 -- redis-cli
```

### Cleanup
```bash
# Delete deployment
kubectl delete -k k8s/

# Or delete namespace (removes everything)
kubectl delete namespace llm-research-lab
```

## Docker Image Management

### Build Image
```bash
# Build locally
docker build -t llm-research-lab:latest .

# Multi-arch build
docker buildx build --platform linux/amd64,linux/arm64 \
  -t llm-research-lab:latest .

# Build with tag
docker build -t llm-research-lab:v1.0.0 .
```

### Registry Operations
```bash
# Tag for registry
docker tag llm-research-lab:latest your-registry.io/llm-research-lab:v1.0.0

# Login to registry
docker login your-registry.io

# Push image
docker push your-registry.io/llm-research-lab:v1.0.0

# Pull image
docker pull your-registry.io/llm-research-lab:v1.0.0
```

## Makefile Shortcuts

```bash
# Development
make build          # Build Rust project
make test          # Run tests
make fmt           # Format code
make clippy        # Run linting

# Docker Compose
make compose-up     # Start all services
make compose-down   # Stop all services
make compose-logs   # View logs

# Kubernetes
make k8s-deploy     # Deploy to k8s
make k8s-status     # Show status
make k8s-logs       # View logs

# Database
make db-migrate     # Run migrations
make db-shell       # PostgreSQL shell
make db-backup      # Backup database

# CI/CD
make ci-all         # Run all CI checks
```

## Deployment Script

```bash
# Make executable
chmod +x deploy.sh

# Local development
./deploy.sh local

# Docker Compose
./deploy.sh docker

# Kubernetes
./deploy.sh k8s

# Build image
./deploy.sh build

# Run tests
./deploy.sh test

# Cleanup
./deploy.sh clean

# With options
./deploy.sh -e prod -n llm-research-lab-prod -t v1.0.0 k8s
```

## Troubleshooting

### Docker Compose Issues
```bash
# View all logs
docker-compose logs

# Restart service
docker-compose restart llm-research-lab

# Rebuild and restart
docker-compose up -d --build llm-research-lab

# Check network
docker network inspect llm-research-network

# Clean and restart
docker-compose down
docker system prune -f
docker-compose up -d
```

### Kubernetes Issues
```bash
# Check pod logs
kubectl logs -n llm-research-lab <pod-name>

# Check previous logs (after crash)
kubectl logs -n llm-research-lab <pod-name> --previous

# Describe pod (see events)
kubectl describe pod -n llm-research-lab <pod-name>

# Check events
kubectl get events -n llm-research-lab --sort-by='.lastTimestamp'

# Debug pod
kubectl debug -n llm-research-lab <pod-name> -it --image=busybox

# Check resource usage
kubectl top pods -n llm-research-lab
kubectl top nodes
```

### Database Connection Issues
```bash
# Test PostgreSQL (Docker)
docker-compose exec postgres psql -U llmlab -d llm_research -c "SELECT version();"

# Test PostgreSQL (K8s)
kubectl run -it --rm debug --image=postgres:16-alpine --restart=Never -n llm-research-lab -- \
  psql postgres://llmlab:llmlab_password@postgres-service:5432/llm_research

# Test ClickHouse (K8s)
kubectl run -it --rm debug --image=clickhouse/clickhouse-client --restart=Never -n llm-research-lab -- \
  clickhouse-client --host clickhouse-service --port 9000
```

## Security Checklist

Before deploying to production:

- [ ] Update all secrets in `k8s/secrets.yaml`
- [ ] Change default passwords (PostgreSQL, ClickHouse, MinIO, Redis)
- [ ] Generate strong JWT secret (min 256 bits)
- [ ] Configure TLS/HTTPS for ingress
- [ ] Set up proper RBAC in Kubernetes
- [ ] Enable network policies
- [ ] Scan Docker images for vulnerabilities
- [ ] Use proper secret management (Vault, Sealed Secrets, etc.)
- [ ] Enable audit logging
- [ ] Configure backup strategy

## Monitoring

### Health Checks
```bash
# Docker Compose
curl http://localhost:8080/health

# Kubernetes (port-forward first)
kubectl port-forward -n llm-research-lab service/llm-research-lab-service 8080:80
curl http://localhost:8080/health
```

### Logs
```bash
# Docker Compose - All services
docker-compose logs -f

# Docker Compose - Specific service
docker-compose logs -f llm-research-lab

# Kubernetes - All application pods
kubectl logs -n llm-research-lab -l app.kubernetes.io/name=llm-research-lab -f

# Kubernetes - Specific pod
kubectl logs -n llm-research-lab <pod-name> -f
```

## Additional Resources

- Full Documentation: [DEPLOYMENT.md](DEPLOYMENT.md)
- Kubernetes Guide: [k8s/README.md](k8s/README.md)
- Project README: [README.md](README.md)
- Makefile Help: `make help`
- Deploy Script Help: `./deploy.sh --help`
