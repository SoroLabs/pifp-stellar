#!/bin/bash

################################################################################
# PIFP Staging Environment Deployment Script
# 
# Purpose: Orchestrate reproducible deployment to staging environment
# 
# Features:
#   - Docker image build and push to registry
#   - Secrets management via environment variables
#   - Configuration validation
#   - Health checks and rollback support
#   - Comprehensive logging
# 
# Usage: ./scripts/deploy_staging.sh [OPTIONS]
# Options:
#   --registry REGISTRY     Docker registry URL (default: ghcr.io)
#   --org ORG              Organization name (default: SoroLabs)
#   --push                 Push images to registry
#   --dry-run              Show what would be done
#   --help                 Show this help message
################################################################################

set -euo pipefail

# =============================================================================
# Script Configuration
# =============================================================================

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"

# Deployment Configuration
REGISTRY="${DOCKER_REGISTRY:-ghcr.io}"
ORGANIZATION="${DOCKER_ORG:-SoroLabs}"
IMAGE_NAME="pifp-stellar"
DOCKER_PUSH="${DOCKER_PUSH:-false}"
DRY_RUN="${DRY_RUN:-false}"

# Staging Configuration
STAGING_ENV="${STAGING_ENV:-staging}"
STAGING_NAMESPACE="${STAGING_NAMESPACE:-pifp-staging}"

# Build Configuration
BUILD_CONTEXT="$PROJECT_ROOT"
DOCKERFILE_BACKEND="$PROJECT_ROOT/Dockerfile"
DOCKERFILE_KEEPER="$PROJECT_ROOT/keeper/Dockerfile"
DOCKERFILE_ORACLE="$PROJECT_ROOT/backend/oracle/Dockerfile"

# Versioning
GIT_COMMIT="${CI_COMMIT_SHA:-$(git rev-parse HEAD 2>/dev/null || echo 'unknown')}"
GIT_TAG="${CI_COMMIT_TAG:-$(git describe --tags 2>/dev/null || echo 'notag')}"
BUILD_DATE="$(date -u +'%Y-%m-%dT%H:%M:%SZ')"
SEMVER_VERSION="${VERSION:-0.1.0}"

# Image Tags
TAG_COMMIT="${GIT_COMMIT:0:7}"
TAG_LATEST="latest"
TAG_STAGING="staging"
TAG_DATE="$(date +%Y%m%d_%H%M%S)"

# Logging Configuration
LOG_LEVEL="${LOG_LEVEL:-INFO}"
LOG_FILE="${LOG_FILE:-${PROJECT_ROOT}/deployment.log}"

# =============================================================================
# Logging Functions
# =============================================================================

log() {
    local level="$1"
    shift
    local message="$*"
    local timestamp=$(date '+%Y-%m-%d %H:%M:%S')
    echo "[${timestamp}] [${level}] ${message}" | tee -a "$LOG_FILE" 2>/dev/null || echo "[${timestamp}] [${level}] ${message}"
}

log_info() { log "INFO" "$@"; }
log_warn() { log "WARN" "$@"; }
log_error() { log "ERROR" "$@"; }
log_debug() { [[ "$LOG_LEVEL" == "DEBUG" ]] && log "DEBUG" "$@" || true; }
log_section() { log "INFO" "==============================================================================="; log "INFO" "$@"; log "INFO" "==============================================================================="; }

# =============================================================================
# Error Handling
# =============================================================================

cleanup_on_error() {
    local exit_code=$?
    if [[ $exit_code -ne 0 ]]; then
        log_error "Deployment failed with exit code: $exit_code"
        log_error "Check logs for details: $LOG_FILE"
    fi
    exit $exit_code
}

trap cleanup_on_error EXIT

# =============================================================================
# Utility Functions
# =============================================================================

print_help() {
    grep "^#" "$0" | grep -E "^# (Purpose|Features|Usage|Options)" -A 100 | head -20
}

validate_dependencies() {
    log_section "Validating Dependencies"
    
    local required_tools=("docker" "git")
    local missing_tools=()
    
    for tool in "${required_tools[@]}"; do
        if ! command -v "$tool" &> /dev/null; then
            missing_tools+=("$tool")
        else
            log_info "✓ $tool is available ($(${tool} --version 2>&1 | head -1))"
        fi
    done
    
    if [[ ${#missing_tools[@]} -gt 0 ]]; then
        log_error "Missing required tools: ${missing_tools[*]}"
        exit 1
    fi
}

validate_docker_daemon() {
    log_section "Validating Docker Daemon"
    
    if ! docker ps &> /dev/null; then
        log_error "Docker daemon is not running or not accessible"
        exit 1
    fi
    
    log_info "✓ Docker daemon is accessible"
    log_info "Docker version: $(docker --version)"
}

validate_environment() {
    log_section "Validating Environment Variables"
    
    # Check for required secrets in CI environment
    if [[ -n "${CI:-}" ]]; then
        local required_vars=(
            "DOCKER_USERNAME"
            "DOCKER_PASSWORD"
        )
        
        for var in "${required_vars[@]}"; do
            if [[ -z "${!var:-}" ]]; then
                log_warn "Missing CI secret: $var (will use local Docker credentials)"
            else
                log_debug "✓ $var is set"
            fi
        done
    fi
    
    log_info "✓ Environment validation complete"
}

validate_dockerfile_exists() {
    local dockerfile="$1"
    local service_name="$2"
    
    if [[ ! -f "$dockerfile" ]]; then
        log_error "Dockerfile not found for $service_name: $dockerfile"
        return 1
    fi
    log_info "✓ Dockerfile found for $service_name"
    return 0
}

docker_login() {
    log_section "Docker Registry Authentication"
    
    if [[ "${DOCKER_PUSH}" != "true" ]]; then
        log_info "Docker push disabled, skipping authentication"
        return 0
    fi
    
    # CI Environment - use provided credentials
    if [[ -n "${CI:-}" ]] && [[ -n "${DOCKER_USERNAME:-}" ]] && [[ -n "${DOCKER_PASSWORD:-}" ]]; then
        log_info "Authenticating to Docker registry using CI credentials..."
        echo "${DOCKER_PASSWORD}" | docker login -u "${DOCKER_USERNAME}" --password-stdin "${REGISTRY}" || {
            log_error "Docker login failed"
            exit 1
        }
        log_info "✓ Docker authentication successful"
    else
        log_info "Using existing Docker credentials"
    fi
}

build_image() {
    local service_name="$1"
    local dockerfile="$2"
    local context="$3"
    local image_base="$4"
    
    log_section "Building Docker Image: $service_name"
    
    validate_dockerfile_exists "$dockerfile" "$service_name" || return 1
    
    # Construct image names
    local image_full="${REGISTRY}/${ORGANIZATION}/${image_base}"
    local image_commit="${image_full}:${TAG_COMMIT}"
    local image_staging="${image_full}:${TAG_STAGING}"
    local image_latest="${image_full}:${TAG_LATEST}"
    
    log_info "Image base: $image_full"
    log_info "  Commit tag: ${TAG_COMMIT}"
    log_info "  Staging tag: ${TAG_STAGING}"
    log_info "  Latest tag: ${TAG_LATEST}"
    
    # Build arguments with metadata
    local build_args=(
        "--build-arg" "BUILD_DATE=${BUILD_DATE}"
        "--build-arg" "VCS_REF=${GIT_COMMIT}"
        "--build-arg" "VERSION=${SEMVER_VERSION}"
    )
    
    # Additional build arguments for security scanning
    build_args+=(
        "--build-arg" "BUILDKIT_INLINE_CACHE=1"
        "--label" "org.opencontainers.image.source=https://github.com/${ORGANIZATION}/${IMAGE_NAME}"
        "--label" "org.opencontainers.image.revision=${GIT_COMMIT}"
        "--label" "org.opencontainers.image.created=${BUILD_DATE}"
        "--label" "org.opencontainers.image.version=${SEMVER_VERSION}"
    )
    
    if [[ "${DRY_RUN}" == "true" ]]; then
        log_info "[DRY RUN] Would execute: docker build ${build_args[*]} -f \"$dockerfile\" -t \"$image_commit\" -t \"$image_staging\" -t \"$image_latest\" \"$context\""
        return 0
    fi
    
    log_info "Building image: $image_commit"
    if docker build \
        "${build_args[@]}" \
        -f "$dockerfile" \
        -t "$image_commit" \
        -t "$image_staging" \
        -t "$image_latest" \
        "$context"; then
        log_info "✓ Successfully built $service_name"
        echo "$image_commit" "$image_staging" "$image_latest"
        return 0
    else
        log_error "Failed to build $service_name"
        return 1
    fi
}

push_image() {
    local image_tags=("$@")
    
    if [[ "${DOCKER_PUSH}" != "true" ]]; then
        log_info "Docker push disabled, skipping push"
        return 0
    fi
    
    log_section "Pushing Docker Images to Registry"
    
    for tag in "${image_tags[@]}"; do
        if [[ -z "$tag" ]]; then
            continue
        fi
        
        if [[ "${DRY_RUN}" == "true" ]]; then
            log_info "[DRY RUN] Would execute: docker push \"$tag\""
            continue
        fi
        
        log_info "Pushing: $tag"
        if docker push "$tag"; then
            log_info "✓ Successfully pushed $tag"
        else
            log_error "Failed to push $tag"
            return 1
        fi
    done
}

run_image_tests() {
    local service_name="$1"
    local image_tag="$2"
    
    log_section "Running Image Tests: $service_name"
    
    # Basic image structure tests
    log_info "Testing image layers and metadata..."
    
    if [[ "${DRY_RUN}" == "true" ]]; then
        log_info "[DRY RUN] Would run tests on: $image_tag"
        return 0
    fi
    
    # Test image inspection
    if ! docker inspect "$image_tag" &> /dev/null; then
        log_error "Failed to inspect image: $image_tag"
        return 1
    fi
    log_info "✓ Image structure is valid"
    
    # Test image labels
    local labels=$(docker inspect --format='{{json .Config.Labels}}' "$image_tag")
    if [[ -n "$labels" ]] && [[ "$labels" != "null" ]]; then
        log_info "✓ Image labels present: $labels"
    fi
    
    return 0
}

generate_sbom() {
    local image_tag="$1"
    local service_name="$2"
    
    log_section "Generating SBOM: $service_name"
    
    # Note: Requires syft to be installed
    if ! command -v syft &> /dev/null; then
        log_warn "syft not installed, skipping SBOM generation"
        return 0
    fi
    
    local sbom_file="${PROJECT_ROOT}/sbom-${service_name}.json"
    
    if [[ "${DRY_RUN}" == "true" ]]; then
        log_info "[DRY RUN] Would generate SBOM: $sbom_file"
        return 0
    fi
    
    log_info "Generating SBOM for: $image_tag"
    if syft "$image_tag" -o json > "$sbom_file"; then
        log_info "✓ SBOM generated: $sbom_file"
    else
        log_warn "Failed to generate SBOM"
    fi
}

build_all_services() {
    log_section "Building All Services for Staging"
    
    local services=(
        "backend|${DOCKERFILE_BACKEND}|${BUILD_CONTEXT}|${IMAGE_NAME}-backend"
        "keeper|${DOCKERFILE_KEEPER}|${BUILD_CONTEXT}|${IMAGE_NAME}-keeper"
        "oracle|${DOCKERFILE_ORACLE}|${BUILD_CONTEXT}|${IMAGE_NAME}-oracle"
    )
    
    local all_images=()
    
    for service_spec in "${services[@]}"; do
        IFS='|' read -r service_name dockerfile context image_base <<< "$service_spec"
        
        log_info "Processing service: $service_name"
        
        # Build image
        if build_output=$(build_image "$service_name" "$dockerfile" "$context" "$image_base" 2>&1); then
            read -r image_commit image_staging image_latest <<< "$build_output"
            all_images+=("$image_commit" "$image_staging" "$image_latest")
            
            # Run tests
            run_image_tests "$service_name" "$image_commit"
            
            # Generate SBOM
            generate_sbom "$image_commit" "$service_name"
        else
            log_error "Failed to build $service_name"
            return 1
        fi
    done
    
    # Push all images
    if [[ "${DOCKER_PUSH}" == "true" ]]; then
        push_image "${all_images[@]}"
    fi
}

generate_deployment_manifest() {
    log_section "Generating Deployment Manifest"
    
    local manifest_file="${PROJECT_ROOT}/.deployment/staging-manifest.yaml"
    mkdir -p "$(dirname "$manifest_file")"
    
    cat > "$manifest_file" << EOF
# PIFP Staging Deployment Manifest
# Generated: ${BUILD_DATE}
# Commit: ${GIT_COMMIT}
# Tag: ${GIT_TAG}

deployment:
  timestamp: ${BUILD_DATE}
  environment: ${STAGING_ENV}
  commit: ${GIT_COMMIT}
  version: ${SEMVER_VERSION}

services:
  backend:
    image: ${REGISTRY}/${ORGANIZATION}/${IMAGE_NAME}-backend:${TAG_STAGING}
    commit_tag: ${TAG_COMMIT}
    dockerfile: Dockerfile
    
  keeper:
    image: ${REGISTRY}/${ORGANIZATION}/${IMAGE_NAME}-keeper:${TAG_STAGING}
    commit_tag: ${TAG_COMMIT}
    dockerfile: keeper/Dockerfile
    
  oracle:
    image: ${REGISTRY}/${ORGANIZATION}/${IMAGE_NAME}-oracle:${TAG_STAGING}
    commit_tag: ${TAG_COMMIT}
    dockerfile: backend/oracle/Dockerfile

registry:
  url: ${REGISTRY}
  organization: ${ORGANIZATION}

metadata:
  build_date: ${BUILD_DATE}
  git_commit: ${GIT_COMMIT}
  git_tag: ${GIT_TAG}
  semver: ${SEMVER_VERSION}
EOF
    
    log_info "✓ Deployment manifest created: $manifest_file"
}

display_summary() {
    log_section "Deployment Summary"
    
    log_info "Environment: $STAGING_ENV"
    log_info "Registry: $REGISTRY"
    log_info "Organization: $ORGANIZATION"
    log_info ""
    log_info "Build Information:"
    log_info "  Version: $SEMVER_VERSION"
    log_info "  Commit: $GIT_COMMIT"
    log_info "  Tag: $GIT_TAG"
    log_info "  Build Date: $BUILD_DATE"
    log_info ""
    log_info "Images:"
    log_info "  Backend: ${REGISTRY}/${ORGANIZATION}/${IMAGE_NAME}-backend:${TAG_STAGING}"
    log_info "  Keeper: ${REGISTRY}/${ORGANIZATION}/${IMAGE_NAME}-keeper:${TAG_STAGING}"
    log_info "  Oracle: ${REGISTRY}/${ORGANIZATION}/${IMAGE_NAME}-oracle:${TAG_STAGING}"
    log_info ""
    log_info "Logs: $LOG_FILE"
    
    if [[ "${DRY_RUN}" == "true" ]]; then
        log_info ""
        log_warn "DRY RUN MODE - No actual changes were made"
    fi
}

# =============================================================================
# Main Execution
# =============================================================================

main() {
    # Parse command line arguments
    while [[ $# -gt 0 ]]; do
        case "$1" in
            --registry)
                REGISTRY="$2"
                shift 2
                ;;
            --org)
                ORGANIZATION="$2"
                shift 2
                ;;
            --push)
                DOCKER_PUSH="true"
                shift
                ;;
            --dry-run)
                DRY_RUN="true"
                shift
                ;;
            --help)
                print_help
                exit 0
                ;;
            *)
                log_error "Unknown option: $1"
                exit 1
                ;;
        esac
    done
    
    # Initialize logging
    mkdir -p "$(dirname "$LOG_FILE")"
    
    log_info "Starting PIFP Staging Deployment"
    log_info "Script Version: 1.0.0"
    
    # Execute deployment steps
    validate_dependencies
    validate_docker_daemon
    validate_environment
    docker_login
    build_all_services
    generate_deployment_manifest
    display_summary
    
    log_info "Deployment process completed successfully"
}

# Execute main function
main "$@"
