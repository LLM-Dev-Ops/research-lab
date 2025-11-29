#!/usr/bin/env bash

################################################################################
# LLM Research Lab Deployment Script
#
# This script helps deploy the LLM Research Lab application to various
# environments (local, docker-compose, kubernetes).
#
# Usage:
#   ./deploy.sh [OPTIONS] COMMAND
#
# Commands:
#   local       - Run locally (cargo run)
#   docker      - Deploy with Docker Compose
#   k8s         - Deploy to Kubernetes
#   build       - Build Docker image
#   test        - Run tests
#   clean       - Clean up resources
#
# Options:
#   -h, --help              Show this help message
#   -e, --env ENV           Environment (dev, staging, prod)
#   -n, --namespace NS      Kubernetes namespace
#   -r, --registry REG      Docker registry
#   -t, --tag TAG           Image tag
#   -v, --verbose           Verbose output
################################################################################

set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Default values
ENVIRONMENT="${ENVIRONMENT:-dev}"
NAMESPACE="${NAMESPACE:-llm-research-lab}"
REGISTRY="${REGISTRY:-ghcr.io/yourusername}"
IMAGE_NAME="llm-research-lab"
IMAGE_TAG="${IMAGE_TAG:-latest}"
VERBOSE=false

# Script directory
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

################################################################################
# Helper Functions
################################################################################

log_info() {
    echo -e "${BLUE}[INFO]${NC} $*"
}

log_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $*"
}

log_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $*"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $*"
}

log_verbose() {
    if [[ "$VERBOSE" == "true" ]]; then
        echo -e "${BLUE}[VERBOSE]${NC} $*"
    fi
}

show_help() {
    sed -n '/^# Usage:/,/^$/p' "$0" | sed 's/^# //g'
}

check_command() {
    if ! command -v "$1" &> /dev/null; then
        log_error "Command '$1' not found. Please install it first."
        return 1
    fi
    log_verbose "Found command: $1"
    return 0
}

confirm() {
    local message="$1"
    local response

    read -p "$message (y/N): " -n 1 -r response
    echo
    [[ "$response" =~ ^[Yy]$ ]]
}

wait_for_service() {
    local service="$1"
    local url="$2"
    local max_attempts="${3:-30}"
    local attempt=1

    log_info "Waiting for $service to be ready..."

    while [ $attempt -le $max_attempts ]; do
        if curl -sf "$url" > /dev/null 2>&1; then
            log_success "$service is ready!"
            return 0
        fi

        log_verbose "Attempt $attempt/$max_attempts: $service not ready yet..."
        sleep 2
        ((attempt++))
    done

    log_error "$service failed to become ready after $max_attempts attempts"
    return 1
}

################################################################################
# Deployment Functions
################################################################################

deploy_local() {
    log_info "Starting local development server..."

    # Check dependencies
    check_command "cargo" || return 1

    # Build and run
    log_info "Building and running with cargo..."
    cargo run --bin llm-research-lab
}

deploy_docker() {
    log_info "Deploying with Docker Compose..."

    # Check dependencies
    check_command "docker" || return 1
    check_command "docker-compose" || return 1

    # Build and start services
    log_info "Starting Docker Compose services..."
    docker-compose up -d

    # Wait for services
    sleep 5
    wait_for_service "PostgreSQL" "http://localhost:5432" 10 || log_warning "PostgreSQL might not be ready"
    wait_for_service "Application" "http://localhost:8080/health" 30

    log_success "Docker Compose deployment complete!"
    log_info "Application is running at: http://localhost:8080"
    log_info "MinIO Console: http://localhost:9001 (admin/minioadmin)"
    log_info "pgAdmin: http://localhost:5050 (admin@llmresearch.local/admin)"

    log_info "\nTo view logs: docker-compose logs -f"
    log_info "To stop: docker-compose down"
}

deploy_k8s() {
    log_info "Deploying to Kubernetes..."

    # Check dependencies
    check_command "kubectl" || return 1

    # Check if kubectl can connect
    if ! kubectl version --short &> /dev/null; then
        log_error "Cannot connect to Kubernetes cluster"
        return 1
    fi

    # Check if kustomize is available
    local use_kustomize=false
    if check_command "kustomize" 2>/dev/null; then
        use_kustomize=true
        log_info "Using kustomize for deployment"
    else
        log_warning "kustomize not found, using kubectl apply"
    fi

    # Confirm deployment
    if [[ "$ENVIRONMENT" == "prod" ]]; then
        log_warning "You are about to deploy to PRODUCTION!"
        if ! confirm "Are you sure you want to continue?"; then
            log_info "Deployment cancelled"
            return 0
        fi
    fi

    # Deploy
    cd "$SCRIPT_DIR/k8s"

    if [[ "$use_kustomize" == "true" ]]; then
        log_info "Applying Kubernetes manifests with kustomize..."
        kubectl apply -k .
    else
        log_info "Applying Kubernetes manifests..."
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
    fi

    # Wait for deployment
    log_info "Waiting for deployment to be ready..."
    kubectl wait --for=condition=available --timeout=300s \
        deployment/llm-research-lab -n "$NAMESPACE" || {
        log_error "Deployment failed to become ready"
        log_info "Checking pod status..."
        kubectl get pods -n "$NAMESPACE"
        return 1
    }

    log_success "Kubernetes deployment complete!"
    log_info "\nDeployment status:"
    kubectl get all -n "$NAMESPACE"

    log_info "\nUseful commands:"
    log_info "  View logs: kubectl logs -n $NAMESPACE -l app.kubernetes.io/name=llm-research-lab -f"
    log_info "  Port forward: kubectl port-forward -n $NAMESPACE service/llm-research-lab-service 8080:80"
    log_info "  Shell access: kubectl exec -it -n $NAMESPACE deployment/llm-research-lab -- /bin/sh"
}

build_image() {
    log_info "Building Docker image..."

    # Check dependencies
    check_command "docker" || return 1

    local full_image_name="$REGISTRY/$IMAGE_NAME:$IMAGE_TAG"

    log_info "Building image: $full_image_name"
    docker build -t "$IMAGE_NAME:$IMAGE_TAG" .

    # Tag for registry
    if [[ "$REGISTRY" != "" ]]; then
        log_info "Tagging image for registry..."
        docker tag "$IMAGE_NAME:$IMAGE_TAG" "$full_image_name"

        if confirm "Push image to registry?"; then
            log_info "Pushing image to registry..."
            docker push "$full_image_name"
            log_success "Image pushed successfully!"
        fi
    fi

    log_success "Build complete: $IMAGE_NAME:$IMAGE_TAG"
}

run_tests() {
    log_info "Running tests..."

    # Check dependencies
    check_command "cargo" || return 1

    log_info "Running Rust tests..."
    cargo test --workspace

    log_success "All tests passed!"
}

clean_resources() {
    log_info "Cleaning up resources..."

    log_info "Cleaning Docker Compose resources..."
    if check_command "docker-compose" 2>/dev/null; then
        docker-compose down -v --remove-orphans
    fi

    log_info "Cleaning Docker images and volumes..."
    if confirm "Remove Docker images and volumes?"; then
        docker system prune -af
        docker volume prune -f
    fi

    log_info "Cleaning Rust build artifacts..."
    if check_command "cargo" 2>/dev/null; then
        cargo clean
    fi

    log_success "Cleanup complete!"
}

################################################################################
# Main Script
################################################################################

main() {
    # Parse options
    while [[ $# -gt 0 ]]; do
        case $1 in
            -h|--help)
                show_help
                exit 0
                ;;
            -e|--env)
                ENVIRONMENT="$2"
                shift 2
                ;;
            -n|--namespace)
                NAMESPACE="$2"
                shift 2
                ;;
            -r|--registry)
                REGISTRY="$2"
                shift 2
                ;;
            -t|--tag)
                IMAGE_TAG="$2"
                shift 2
                ;;
            -v|--verbose)
                VERBOSE=true
                shift
                ;;
            local|docker|k8s|build|test|clean)
                COMMAND="$1"
                shift
                break
                ;;
            *)
                log_error "Unknown option: $1"
                show_help
                exit 1
                ;;
        esac
    done

    # Check if command is provided
    if [[ -z "${COMMAND:-}" ]]; then
        log_error "No command provided"
        show_help
        exit 1
    fi

    # Show configuration
    log_info "Configuration:"
    log_info "  Environment: $ENVIRONMENT"
    log_info "  Namespace: $NAMESPACE"
    log_info "  Registry: $REGISTRY"
    log_info "  Image: $IMAGE_NAME:$IMAGE_TAG"
    echo

    # Execute command
    case $COMMAND in
        local)
            deploy_local
            ;;
        docker)
            deploy_docker
            ;;
        k8s)
            deploy_k8s
            ;;
        build)
            build_image
            ;;
        test)
            run_tests
            ;;
        clean)
            clean_resources
            ;;
        *)
            log_error "Unknown command: $COMMAND"
            show_help
            exit 1
            ;;
    esac
}

# Run main function
main "$@"
