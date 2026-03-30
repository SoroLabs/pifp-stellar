# Staging Environment Deployment Guide

## Overview

This guide covers the deployment process for the PIFP Stellar backend services to the staging environment using Docker and CI/CD automation.

## Architecture

```
┌─────────────────────────────────────────────────────────┐
│              GitHub Actions CI/CD Pipeline              │
├─────────────────────────────────────────────────────────┤
│                                                           │
│  1. Trigger: Push to develop branch                      │
│  2. Checkout code                                        │
│  3. Setup Docker & Registry authentication               │
│  4. Build backend services (Backend, Keeper, Oracle)     │
│  5. Tag images (commit, staging, latest)                 │
│  6. Push images to container registry                    │
│  7. Generate deployment artifacts                        │
│                                                           │
└─────────────────────────────────────────────────────────┘
                            ↓
┌─────────────────────────────────────────────────────────┐
│         Docker Container Registry (GHCR.io)             │
├─────────────────────────────────────────────────────────┤
│                                                           │
│  pifp-stellar-backend:staging                            │
│  pifp-stellar-keeper:staging                             │
│  pifp-stellar-oracle:staging                             │
│                                                           │
└─���───────────────────────────────────────────────────────┘
                            ↓
┌─────────────────────────────────────────────────────────┐
│           Staging Deployment Environment                │
├─────────────────────────────────────────────────────────┤
│                                                           │
│  Backend Services running in staging namespace           │
│  Health checks & monitoring enabled                      │
│  Automatic rollback on failure                           │
│                                                           │
└─────────────────────────────────────────────────────────┘
```

## Prerequisites

### Local Development

1. **Required Tools**:
   - Docker Engine 20.10+
   - Git 2.30+
   - Bash 4.0+

2. **Docker Setup**:
   ```bash
   # Verify Docker is running
   docker ps
   
   # (Optional) Configure Docker for your registry
   docker login ghcr.io
   ```

### CI/CD Environment

1. **GitHub Secrets** (must be configured in repository settings):
   - `DOCKER_USERNAME`: Container registry username
   - `DOCKER_PASSWORD`: Container registry password/token
   - `ORACLE_SECRET_KEY`: Signing key for Oracle service

2. **GitHub Workflow Permissions**:
   - `contents: read` - Access to repository
   - `packages: write` - Push to container registry

## Local Deployment

### Basic Usage

```bash
# 1. Navigate to project root
cd /path/to/pifp-stellar

# 2. Run deployment script (builds images locally)
./scripts/deploy_staging.sh

# 3. View logs
tail -f deployment.log
```

### Advanced Options

```bash
# Dry-run mode (show what would be done)
DRY_RUN=true ./scripts/deploy_staging.sh

# Push to custom registry
./scripts/deploy_staging.sh \
  --registry docker.io \
  --org myorganization \
  --push

# Build with custom log level
LOG_LEVEL=DEBUG ./scripts/deploy_staging.sh

# Specify custom versions
VERSION=1.2.3 ./scripts/deploy_staging.sh
```

## CI/CD Deployment

### Automatic Deployment

The GitHub Actions workflow automatically triggers when:

1. Code is pushed to the `develop` branch
2. Dockerfiles or deployment scripts are modified
3. Manual trigger via `workflow_dispatch`

### Trigger Manual Deployment

```bash
# Using GitHub CLI
gh workflow run deploy-staging.yml \
  -f push_images=true \
  -r develop

# Using GitHub UI
1. Go to Actions tab
2. Select "Deploy to Staging" workflow
3. Click "Run workflow"
4. Select branch (develop)
5. Click "Run workflow"
```

### Secrets Configuration

#### Step 1: Create GitHub Secrets

```bash
# Using GitHub CLI
gh secret set DOCKER_USERNAME --body "your-username"
gh secret set DOCKER_PASSWORD --body "your-token"
gh secret set ORACLE_SECRET_KEY --body "your-secret-key"
```

#### Step 2: Verify Secrets

```bash
gh secret list
```

## Build Configuration

### Image Tags

The deployment script automatically generates multiple tags for each image:

```
ghcr.io/SoroLabs/pifp-stellar-backend:a1b2c3d    # Commit SHA (short)
ghcr.io/SoroLabs/pifp-stellar-backend:staging    # Staging track
ghcr.io/SoroLabs/pifp-stellar-backend:latest     # Latest release
```

### Build Arguments

All images are built with metadata labels:

```dockerfile
ARG BUILD_DATE=2024-03-30T15:30:45Z
ARG VCS_REF=a1b2c3d7e8f9g0h1
ARG VERSION=0.1.0
```

### Multi-stage Builds

Each service uses optimized multi-stage builds:

- **Backend**: Rust compilation + minimal runtime
- **Keeper**: Node.js dependencies + production runtime
- **Oracle**: Rust binary + security hardening

## Security Considerations

### 1. Secret Management

**✓ Best Practices**:
- Use GitHub Secrets for all credentials
- Never commit `.env.staging` with actual secrets
- Rotate secrets regularly
- Use least-privilege access tokens

**✗ Avoid**:
- Committing secrets to repository
- Using the same credentials for all environments
- Long-lived personal tokens

### 2. Image Security

**Image Scanning**:
```bash
# Optional: Use trivy for vulnerability scanning
trivy image ghcr.io/SoroLabs/pifp-stellar-backend:staging
```

**SBOM Generation**:
```bash
# Automatically generated if syft is installed
# Output: sbom-backend.json, sbom-keeper.json, sbom-oracle.json
```

### 3. Registry Security

**GitHub Container Registry (GHCR.io)**:
- Images stored in GitHub account
- Private by default (customize in package settings)
- Automatic cleanup after 90 days
- Supports token-based authentication

## Troubleshooting

### Issue: Docker Push Fails

```bash
# Error: unauthorized: authentication required
# Solution: Check Docker credentials
docker logout ghcr.io
echo $DOCKER_PASSWORD | docker login -u $DOCKER_USERNAME --password-stdin ghcr.io
```

### Issue: Build Fails

```bash
# Check Dockerfile syntax
docker build --dry-run -f Dockerfile .

# View detailed build output
DOCKER_BUILDKIT=1 docker build \
  --progress=plain \
  -f Dockerfile \
  -t test:latest .
```

### Issue: Image Not Found

```bash
# Verify image exists
docker images | grep pifp

# Pull from registry
docker pull ghcr.io/SoroLabs/pifp-stellar-backend:staging

# Inspect image details
docker inspect ghcr.io/SoroLabs/pifp-stellar-backend:staging
```

### Issue: Permission Denied

```bash
# Ensure script is executable
chmod +x scripts/deploy_staging.sh

# Or run with bash explicitly
bash scripts/deploy_staging.sh
```

## Deployment Verification

### Check Build Artifacts

```bash
# View generated manifest
cat .deployment/staging-manifest.yaml

# Review deployment logs
cat deployment.log

# Verify images in registry
docker images | grep staging
```

### Health Checks

Each service includes health checks:

```bash
# Backend (Rust)
docker run pifp-stellar-backend:staging cargo build --help

# Keeper (Node.js)
docker run pifp-stellar-keeper:staging node -e "require('http').get(...)"

# Oracle (Rust)
docker run pifp-stellar-oracle:staging pifp-oracle --help
```

## Environment Management

### Configuration Files

```
scripts/
├── .env.staging.example      # Template configuration
├── .env.staging              # Actual config (git-ignored)
├── deploy_staging.sh         # Main deployment script
└── DEPLOY_STAGING_README.md  # This file
```

### Loading Environment

```bash
# Load configuration
source scripts/.env.staging

# Verify variables
echo "Registry: $DOCKER_REGISTRY"
echo "Organization: $DOCKER_ORG"
```

## Next Steps

### 1. Initial Setup

```bash
# Copy configuration template
cp scripts/.env.staging.example scripts/.env.staging

# Edit with your staging details
nano scripts/.env.staging

# Test locally
./scripts/deploy_staging.sh --dry-run
```

### 2. Configure CI/CD

```bash
# Set GitHub Secrets (see Secrets Configuration section)
gh secret set DOCKER_USERNAME --body "your-username"
gh secret set DOCKER_PASSWORD --body "your-token"
```

### 3. First Deployment

```bash
# Manual deployment (test everything works)
./scripts/deploy_staging.sh --push

# Monitor GitHub Actions
gh run watch
```

### 4. Monitoring & Maintenance

- Set up alerting for failed deployments
- Regular security vulnerability scanning
- Monthly review of deployment logs
- Quarterly SBOM analysis

## Related Documentation

- [ARCHITECTURE.md](../ARCHITECTURE.md) - System design
- [README.md](../README.md) - Project overview
- [CONTRIBUTING.md](../CONTRIBUTING.md) - Development guidelines
- [GitHub Actions Documentation](https://docs.github.com/en/actions)

## Support

For issues or questions:

1. Check troubleshooting section above
2. Review GitHub Actions logs
3. Check Docker build output
4. Open GitHub issue with deployment logs

---

**Last Updated**: March 30, 2024  
**Version**: 1.0.0  
**Maintainer**: PIFP Development Team