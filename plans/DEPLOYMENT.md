# LLM Research Lab - Deployment Guide

Complete deployment guide for the LLM Research Lab platform using Docker and Kubernetes.

## Table of Contents

- [Docker Deployment](#docker-deployment)
- [Kubernetes Deployment](#kubernetes-deployment)
- [CI/CD Integration](#cicd-integration)
- [Production Checklist](#production-checklist)

## Docker Deployment

### Local Development with Docker Compose

#### Prerequisites

- Docker 24.0+
- Docker Compose 2.20+
- 8GB RAM minimum
- 20GB free disk space

#### Quick Start

```bash
# Clone the repository
git clone https://github.com/yourusername/llm-research-lab.git
cd llm-research-lab

# Start all services
docker-compose up -d

# Check service status
docker-compose ps

# View logs
docker-compose logs -f llm-research-lab

# Access the application
curl http://localhost:8080/health
```

#### Service Access

- **Application**: http://localhost:8080
- **PostgreSQL**: localhost:5432
- **ClickHouse HTTP**: http://localhost:8123
- **ClickHouse Native**: localhost:9000
- **MinIO API**: http://localhost:9000
- **MinIO Console**: http://localhost:9001
- **Redis**: localhost:6379
- **pgAdmin** (optional): http://localhost:5050

#### Managing Services

```bash
# Stop all services
docker-compose down

# Stop and remove volumes (CAUTION: deletes all data)
docker-compose down -v

# Restart a specific service
docker-compose restart llm-research-lab

# View logs for a specific service
docker-compose logs -f postgres

# Execute commands in a container
docker-compose exec llm-research-lab /bin/sh
docker-compose exec postgres psql -U llmlab -d llm_research

# Rebuild application
docker-compose build llm-research-lab
docker-compose up -d llm-research-lab
```

#### Database Management

```bash
# Run migrations
docker-compose exec llm-research-lab /app/llm-research-lab migrate

# Access PostgreSQL
docker-compose exec postgres psql -U llmlab -d llm_research

# Backup database
docker-compose exec postgres pg_dump -U llmlab llm_research > backup.sql

# Restore database
docker-compose exec -T postgres psql -U llmlab -d llm_research < backup.sql

# Access ClickHouse
docker-compose exec clickhouse clickhouse-client --user llmlab --password llmlab_password
```

#### MinIO Configuration

```bash
# Access MinIO console
open http://localhost:9001
# Username: minioadmin
# Password: minioadmin

# Using MinIO client (mc)
docker-compose run --rm minio-init

# List buckets
docker run --rm --network llm-research-network \
  minio/mc ls myminio/llm-research-artifacts
```

### Building Custom Docker Images

#### Multi-Architecture Builds

```bash
# Create a new builder
docker buildx create --name multiarch --driver docker-container --use

# Build for multiple platforms
docker buildx build \
  --platform linux/amd64,linux/arm64 \
  -t llm-research-lab:latest \
  --push \
  .

# Build for specific platform
docker buildx build \
  --platform linux/amd64 \
  -t llm-research-lab:latest \
  --load \
  .
```

#### Optimized Production Build

```bash
# Build with specific Rust version
docker build \
  --build-arg RUST_VERSION=1.83 \
  -t llm-research-lab:v1.0.0 \
  .

# Build with caching
docker build \
  --cache-from llm-research-lab:latest \
  -t llm-research-lab:v1.0.0 \
  .

# Build with BuildKit
DOCKER_BUILDKIT=1 docker build \
  -t llm-research-lab:latest \
  .
```

#### Image Management

```bash
# Tag image
docker tag llm-research-lab:latest your-registry.io/llm-research-lab:v1.0.0

# Push to registry
docker push your-registry.io/llm-research-lab:v1.0.0

# Pull image
docker pull your-registry.io/llm-research-lab:v1.0.0

# Inspect image
docker inspect llm-research-lab:latest

# View image layers
docker history llm-research-lab:latest

# Scan for vulnerabilities
docker scan llm-research-lab:latest
```

### Docker Compose Profiles

```bash
# Start with tools (pgAdmin)
docker-compose --profile tools up -d

# Production mode (no dev tools)
docker-compose --profile production up -d

# Development mode with hot reload
docker-compose --profile dev up -d
```

### Environment Variables

Create a `.env` file for custom configuration:

```bash
# .env
COMPOSE_PROJECT_NAME=llm-research-lab
LLM_RESEARCH_PORT=8080
POSTGRES_PASSWORD=secure_password_here
CLICKHOUSE_PASSWORD=secure_password_here
MINIO_ROOT_PASSWORD=secure_password_here
```

Then start with:

```bash
docker-compose --env-file .env up -d
```

## Kubernetes Deployment

See [k8s/README.md](k8s/README.md) for complete Kubernetes deployment documentation.

### Quick Kubernetes Deployment

```bash
# Navigate to k8s directory
cd k8s

# Deploy using kubectl
kubectl apply -f namespace.yaml
kubectl apply -f configmap.yaml
kubectl apply -f secrets.yaml  # Update secrets first!
kubectl apply -f .

# Or deploy using kustomize
kubectl apply -k .

# Check status
kubectl get all -n llm-research-lab

# Access application (if using port-forward)
kubectl port-forward -n llm-research-lab service/llm-research-lab-service 8080:80
```

### Helm Chart (Future Enhancement)

A Helm chart will be provided for easier Kubernetes deployments:

```bash
# Add Helm repository (when available)
helm repo add llm-research-lab https://charts.llm-research-lab.io
helm repo update

# Install chart
helm install llm-research-lab llm-research-lab/llm-research-lab \
  --namespace llm-research-lab \
  --create-namespace \
  --values values-production.yaml

# Upgrade
helm upgrade llm-research-lab llm-research-lab/llm-research-lab \
  --namespace llm-research-lab \
  --values values-production.yaml

# Uninstall
helm uninstall llm-research-lab --namespace llm-research-lab
```

## CI/CD Integration

### GitHub Actions

Create `.github/workflows/deploy.yml`:

```yaml
name: Build and Deploy

on:
  push:
    branches: [main]
  pull_request:
    branches: [main]

jobs:
  build:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Set up Docker Buildx
        uses: docker/setup-buildx-action@v3

      - name: Login to Container Registry
        uses: docker/login-action@v3
        with:
          registry: ghcr.io
          username: ${{ github.actor }}
          password: ${{ secrets.GITHUB_TOKEN }}

      - name: Build and push
        uses: docker/build-push-action@v5
        with:
          context: .
          push: true
          tags: ghcr.io/${{ github.repository }}:${{ github.sha }}
          cache-from: type=gha
          cache-to: type=gha,mode=max

  deploy:
    needs: build
    runs-on: ubuntu-latest
    if: github.ref == 'refs/heads/main'
    steps:
      - uses: actions/checkout@v4

      - name: Configure kubectl
        uses: azure/k8s-set-context@v3
        with:
          method: kubeconfig
          kubeconfig: ${{ secrets.KUBE_CONFIG }}

      - name: Deploy to Kubernetes
        run: |
          cd k8s
          kustomize edit set image llm-research-lab=ghcr.io/${{ github.repository }}:${{ github.sha }}
          kubectl apply -k .
          kubectl rollout status deployment/llm-research-lab -n llm-research-lab
```

### GitLab CI

Create `.gitlab-ci.yml`:

```yaml
stages:
  - build
  - deploy

variables:
  DOCKER_DRIVER: overlay2
  IMAGE_TAG: $CI_REGISTRY_IMAGE:$CI_COMMIT_SHA

build:
  stage: build
  image: docker:24
  services:
    - docker:24-dind
  script:
    - docker login -u $CI_REGISTRY_USER -p $CI_REGISTRY_PASSWORD $CI_REGISTRY
    - docker build -t $IMAGE_TAG .
    - docker push $IMAGE_TAG
  only:
    - main
    - develop

deploy_production:
  stage: deploy
  image: bitnami/kubectl:latest
  script:
    - kubectl config use-context production
    - cd k8s
    - kustomize edit set image llm-research-lab=$IMAGE_TAG
    - kubectl apply -k .
    - kubectl rollout status deployment/llm-research-lab -n llm-research-lab
  only:
    - main
  when: manual
```

### Jenkins Pipeline

Create `Jenkinsfile`:

```groovy
pipeline {
    agent any

    environment {
        DOCKER_REGISTRY = 'your-registry.io'
        IMAGE_NAME = 'llm-research-lab'
        IMAGE_TAG = "${env.BUILD_NUMBER}"
        KUBECONFIG = credentials('kubeconfig')
    }

    stages {
        stage('Build') {
            steps {
                script {
                    docker.build("${DOCKER_REGISTRY}/${IMAGE_NAME}:${IMAGE_TAG}")
                }
            }
        }

        stage('Test') {
            steps {
                sh 'docker run --rm ${DOCKER_REGISTRY}/${IMAGE_NAME}:${IMAGE_TAG} --test'
            }
        }

        stage('Push') {
            steps {
                script {
                    docker.withRegistry("https://${DOCKER_REGISTRY}", 'docker-credentials') {
                        docker.image("${DOCKER_REGISTRY}/${IMAGE_NAME}:${IMAGE_TAG}").push()
                        docker.image("${DOCKER_REGISTRY}/${IMAGE_NAME}:${IMAGE_TAG}").push('latest')
                    }
                }
            }
        }

        stage('Deploy') {
            when {
                branch 'main'
            }
            steps {
                sh '''
                    cd k8s
                    kustomize edit set image llm-research-lab=${DOCKER_REGISTRY}/${IMAGE_NAME}:${IMAGE_TAG}
                    kubectl apply -k .
                    kubectl rollout status deployment/llm-research-lab -n llm-research-lab
                '''
            }
        }
    }

    post {
        always {
            cleanWs()
        }
    }
}
```

## Production Checklist

### Pre-Deployment

- [ ] Update all secrets in `k8s/secrets.yaml`
- [ ] Configure TLS certificates
- [ ] Set up DNS records
- [ ] Configure backup strategy
- [ ] Set up monitoring and alerting
- [ ] Review resource limits and requests
- [ ] Configure autoscaling parameters
- [ ] Set up log aggregation
- [ ] Review security policies
- [ ] Test disaster recovery procedures

### Security

- [ ] Change all default passwords
- [ ] Use strong JWT secrets (min 256 bits)
- [ ] Enable HTTPS/TLS everywhere
- [ ] Configure network policies
- [ ] Enable pod security standards
- [ ] Scan images for vulnerabilities
- [ ] Set up RBAC properly
- [ ] Enable audit logging
- [ ] Use secrets management (Vault, etc.)
- [ ] Implement rate limiting

### High Availability

- [ ] Deploy across multiple availability zones
- [ ] Configure database replication
- [ ] Set up load balancing
- [ ] Configure pod anti-affinity
- [ ] Set up pod disruption budgets
- [ ] Test failover scenarios
- [ ] Configure health checks
- [ ] Set up circuit breakers
- [ ] Implement retry logic
- [ ] Document recovery procedures

### Performance

- [ ] Load test the application
- [ ] Optimize database queries
- [ ] Configure connection pooling
- [ ] Set up CDN for static assets
- [ ] Optimize Docker image size
- [ ] Configure caching strategy
- [ ] Tune resource limits
- [ ] Enable HTTP/2 or HTTP/3
- [ ] Implement response compression
- [ ] Monitor and optimize metrics

### Monitoring

- [ ] Set up Prometheus for metrics
- [ ] Configure Grafana dashboards
- [ ] Set up alert rules
- [ ] Configure log aggregation (ELK/Loki)
- [ ] Set up distributed tracing
- [ ] Monitor database performance
- [ ] Track application metrics
- [ ] Set up uptime monitoring
- [ ] Configure SLA tracking
- [ ] Document runbooks

### Operations

- [ ] Document deployment procedures
- [ ] Create runbooks for common issues
- [ ] Set up automated backups
- [ ] Test backup restoration
- [ ] Configure log rotation
- [ ] Set up maintenance windows
- [ ] Document scaling procedures
- [ ] Create incident response plan
- [ ] Set up on-call rotation
- [ ] Document rollback procedures

### Compliance

- [ ] Review data retention policies
- [ ] Implement audit logging
- [ ] Document security controls
- [ ] Review access controls
- [ ] Implement encryption at rest
- [ ] Configure encryption in transit
- [ ] Review compliance requirements
- [ ] Document data flow
- [ ] Implement data backup policies
- [ ] Review third-party dependencies

## Troubleshooting

### Docker Issues

```bash
# Container won't start
docker-compose logs llm-research-lab
docker-compose ps

# Out of disk space
docker system prune -a
docker volume prune

# Network issues
docker network inspect llm-research-network
docker-compose down && docker-compose up -d

# Permission issues
sudo chown -R $USER:$USER .
```

### Kubernetes Issues

See [k8s/README.md](k8s/README.md#troubleshooting) for detailed Kubernetes troubleshooting.

## Support

- Documentation: [README.md](README.md)
- Kubernetes Guide: [k8s/README.md](k8s/README.md)
- Issues: https://github.com/yourusername/llm-research-lab/issues
- Discussions: https://github.com/yourusername/llm-research-lab/discussions
