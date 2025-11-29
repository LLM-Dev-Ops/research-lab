# Makefile for LLM Research Lab
# Provides common operations for development, testing, and deployment

.PHONY: help build test clean docker-build docker-push k8s-deploy k8s-delete

# Variables
IMAGE_NAME ?= llm-research-lab
IMAGE_TAG ?= latest
REGISTRY ?= ghcr.io/yourusername
NAMESPACE ?= llm-research-lab

# Colors for output
BLUE := \033[0;34m
GREEN := \033[0;32m
YELLOW := \033[0;33m
RED := \033[0;31m
NC := \033[0m # No Color

##@ General

help: ## Display this help
	@awk 'BEGIN {FS = ":.*##"; printf "\nUsage:\n  make $(YELLOW)<target>$(NC)\n"} /^[a-zA-Z_0-9-]+:.*?##/ { printf "  $(BLUE)%-15s$(NC) %s\n", $$1, $$2 } /^##@/ { printf "\n$(GREEN)%s$(NC)\n", substr($$0, 5) } ' $(MAKEFILE_LIST)

##@ Development

build: ## Build the Rust project
	@echo "$(BLUE)Building Rust project...$(NC)"
	cargo build --release

test: ## Run tests
	@echo "$(BLUE)Running tests...$(NC)"
	cargo test --workspace

check: ## Run cargo check
	@echo "$(BLUE)Running cargo check...$(NC)"
	cargo check --workspace

fmt: ## Format code
	@echo "$(BLUE)Formatting code...$(NC)"
	cargo fmt --all

fmt-check: ## Check code formatting
	@echo "$(BLUE)Checking code formatting...$(NC)"
	cargo fmt --all -- --check

clippy: ## Run clippy
	@echo "$(BLUE)Running clippy...$(NC)"
	cargo clippy --workspace -- -D warnings

clean: ## Clean build artifacts
	@echo "$(BLUE)Cleaning build artifacts...$(NC)"
	cargo clean
	rm -rf target/

watch: ## Watch and rebuild on changes
	@echo "$(BLUE)Watching for changes...$(NC)"
	cargo watch -x 'run --bin llm-research-lab'

##@ Docker

docker-build: ## Build Docker image
	@echo "$(BLUE)Building Docker image: $(IMAGE_NAME):$(IMAGE_TAG)$(NC)"
	docker build -t $(IMAGE_NAME):$(IMAGE_TAG) .

docker-build-multiarch: ## Build multi-architecture Docker image
	@echo "$(BLUE)Building multi-arch Docker image: $(IMAGE_NAME):$(IMAGE_TAG)$(NC)"
	docker buildx build --platform linux/amd64,linux/arm64 -t $(IMAGE_NAME):$(IMAGE_TAG) .

docker-tag: ## Tag Docker image for registry
	@echo "$(BLUE)Tagging image: $(REGISTRY)/$(IMAGE_NAME):$(IMAGE_TAG)$(NC)"
	docker tag $(IMAGE_NAME):$(IMAGE_TAG) $(REGISTRY)/$(IMAGE_NAME):$(IMAGE_TAG)

docker-push: docker-tag ## Push Docker image to registry
	@echo "$(BLUE)Pushing image: $(REGISTRY)/$(IMAGE_NAME):$(IMAGE_TAG)$(NC)"
	docker push $(REGISTRY)/$(IMAGE_NAME):$(IMAGE_TAG)

docker-run: ## Run Docker container locally
	@echo "$(BLUE)Running Docker container...$(NC)"
	docker run --rm -p 8080:8080 --env-file .env $(IMAGE_NAME):$(IMAGE_TAG)

docker-scan: ## Scan Docker image for vulnerabilities
	@echo "$(BLUE)Scanning Docker image for vulnerabilities...$(NC)"
	docker scan $(IMAGE_NAME):$(IMAGE_TAG)

##@ Docker Compose

compose-up: ## Start all services with docker-compose
	@echo "$(BLUE)Starting all services...$(NC)"
	docker-compose up -d

compose-down: ## Stop all services
	@echo "$(BLUE)Stopping all services...$(NC)"
	docker-compose down

compose-logs: ## Show docker-compose logs
	@echo "$(BLUE)Showing logs...$(NC)"
	docker-compose logs -f

compose-ps: ## Show docker-compose status
	docker-compose ps

compose-restart: ## Restart all services
	@echo "$(BLUE)Restarting all services...$(NC)"
	docker-compose restart

compose-build: ## Build docker-compose services
	@echo "$(BLUE)Building services...$(NC)"
	docker-compose build

compose-clean: ## Stop and remove all containers, networks, volumes
	@echo "$(RED)Removing all containers, networks, and volumes...$(NC)"
	docker-compose down -v --remove-orphans

##@ Kubernetes

k8s-deploy: ## Deploy to Kubernetes
	@echo "$(BLUE)Deploying to Kubernetes namespace: $(NAMESPACE)$(NC)"
	kubectl apply -k k8s/

k8s-delete: ## Delete from Kubernetes
	@echo "$(RED)Deleting from Kubernetes namespace: $(NAMESPACE)$(NC)"
	kubectl delete -k k8s/

k8s-status: ## Show Kubernetes deployment status
	@echo "$(BLUE)Deployment status:$(NC)"
	kubectl get all -n $(NAMESPACE)

k8s-logs: ## Show Kubernetes logs
	@echo "$(BLUE)Application logs:$(NC)"
	kubectl logs -n $(NAMESPACE) -l app.kubernetes.io/name=llm-research-lab --tail=100 -f

k8s-describe: ## Describe Kubernetes deployment
	kubectl describe deployment/llm-research-lab -n $(NAMESPACE)

k8s-port-forward: ## Port forward to application
	@echo "$(BLUE)Port forwarding to localhost:8080$(NC)"
	kubectl port-forward -n $(NAMESPACE) service/llm-research-lab-service 8080:80

k8s-exec: ## Execute shell in pod
	kubectl exec -it -n $(NAMESPACE) deployment/llm-research-lab -- /bin/sh

k8s-restart: ## Restart deployment
	@echo "$(BLUE)Restarting deployment...$(NC)"
	kubectl rollout restart deployment/llm-research-lab -n $(NAMESPACE)

k8s-rollback: ## Rollback deployment
	@echo "$(YELLOW)Rolling back deployment...$(NC)"
	kubectl rollout undo deployment/llm-research-lab -n $(NAMESPACE)

k8s-scale: ## Scale deployment (use REPLICAS=N)
	@echo "$(BLUE)Scaling deployment to $(REPLICAS) replicas...$(NC)"
	kubectl scale deployment/llm-research-lab --replicas=$(REPLICAS) -n $(NAMESPACE)

k8s-update-image: ## Update image in deployment
	@echo "$(BLUE)Updating image to: $(REGISTRY)/$(IMAGE_NAME):$(IMAGE_TAG)$(NC)"
	kubectl set image deployment/llm-research-lab llm-research-lab=$(REGISTRY)/$(IMAGE_NAME):$(IMAGE_TAG) -n $(NAMESPACE)

##@ Database

db-migrate: ## Run database migrations (docker-compose)
	@echo "$(BLUE)Running database migrations...$(NC)"
	docker-compose exec llm-research-lab /app/llm-research-lab migrate

db-shell: ## Access PostgreSQL shell
	@echo "$(BLUE)Connecting to PostgreSQL...$(NC)"
	docker-compose exec postgres psql -U llmlab -d llm_research

db-backup: ## Backup database
	@echo "$(BLUE)Backing up database...$(NC)"
	docker-compose exec postgres pg_dump -U llmlab llm_research > backup-$$(date +%Y%m%d-%H%M%S).sql
	@echo "$(GREEN)Backup saved to: backup-$$(date +%Y%m%d-%H%M%S).sql$(NC)"

db-restore: ## Restore database (use BACKUP=filename)
	@echo "$(YELLOW)Restoring database from: $(BACKUP)$(NC)"
	docker-compose exec -T postgres psql -U llmlab -d llm_research < $(BACKUP)

##@ Testing

test-unit: ## Run unit tests
	cargo test --lib --workspace

test-integration: ## Run integration tests
	cargo test --test '*' --workspace

test-doc: ## Run documentation tests
	cargo test --doc --workspace

test-coverage: ## Generate test coverage report
	@echo "$(BLUE)Generating coverage report...$(NC)"
	cargo tarpaulin --workspace --out Html --output-dir coverage

bench: ## Run benchmarks
	cargo bench --workspace

##@ CI/CD

ci-lint: fmt-check clippy ## Run CI linting checks

ci-test: test ## Run CI tests

ci-build: build docker-build ## Run CI build

ci-all: ci-lint ci-test ci-build ## Run all CI steps

##@ Utilities

install-tools: ## Install development tools
	@echo "$(BLUE)Installing development tools...$(NC)"
	cargo install cargo-watch
	cargo install cargo-tarpaulin
	rustup component add rustfmt clippy

update-deps: ## Update dependencies
	@echo "$(BLUE)Updating dependencies...$(NC)"
	cargo update

audit: ## Audit dependencies for security vulnerabilities
	@echo "$(BLUE)Auditing dependencies...$(NC)"
	cargo audit

tree: ## Show dependency tree
	cargo tree

outdated: ## Check for outdated dependencies
	cargo outdated

version: ## Show version information
	@echo "$(BLUE)Version Information:$(NC)"
	@echo "Rust: $$(rustc --version)"
	@echo "Cargo: $$(cargo --version)"
	@echo "Docker: $$(docker --version)"
	@echo "kubectl: $$(kubectl version --client --short 2>/dev/null || echo 'not installed')"

health-check: ## Check if application is running
	@echo "$(BLUE)Checking application health...$(NC)"
	@curl -f http://localhost:8080/health && echo "$(GREEN)✓ Application is healthy$(NC)" || echo "$(RED)✗ Application is not responding$(NC)"

##@ Documentation

docs: ## Generate and open documentation
	@echo "$(BLUE)Generating documentation...$(NC)"
	cargo doc --workspace --no-deps --open

docs-serve: ## Serve documentation locally
	@echo "$(BLUE)Serving documentation on http://localhost:8000$(NC)"
	cd target/doc && python3 -m http.server 8000

##@ Cleanup

clean-docker: ## Clean Docker resources
	@echo "$(BLUE)Cleaning Docker resources...$(NC)"
	docker system prune -f
	docker volume prune -f

clean-k8s: ## Clean Kubernetes resources
	@echo "$(RED)Cleaning Kubernetes resources...$(NC)"
	kubectl delete namespace $(NAMESPACE) --ignore-not-found=true

clean-all: clean clean-docker ## Clean everything
	@echo "$(GREEN)Cleanup complete!$(NC)"

.DEFAULT_GOAL := help
