# Zavora Trading Engine - Cloud Deployment Guide

This guide covers deploying Zavora Trading Engine to both AWS and GCP using Kubernetes and Terraform.

## Architecture Overview

The deployment architecture consists of:

- Kubernetes for container orchestration
- PostgreSQL database (RDS on AWS, Cloud SQL on GCP)
- Redis for caching (ElastiCache on AWS, Memorystore on GCP)
- Load Balancer (ALB on AWS, Cloud Load Balancing on GCP)
- Container Registry (ECR on AWS, Artifact Registry on GCP)

## Prerequisites

- Terraform >= 1.0.0
- kubectl >= 1.20
- AWS CLI or Google Cloud SDK
- Docker

## Folder Structure

```
deploy/
├── k8s/                # Kubernetes manifests
│   ├── base/           # Base configurations
│   └── overlays/       # Environment-specific overlays
│       ├── dev/        
│       └── prod/       
├── terraform/          # Infrastructure as Code
│   ├── aws/            # AWS-specific configurations
│   └── gcp/            # GCP-specific configurations
└── scripts/            # Deployment scripts
```

## Deployment Steps

1. Build and push container images
2. Create infrastructure with Terraform
3. Deploy application with Kubernetes
4. Configure networking and security

See the [DEPLOYMENT.md](./DEPLOYMENT.md) file for detailed instructions.

## Release Management

The project uses semantic versioning with GitHub releases that align with the RELEASE_NOTES.md file.

### Creating a GitHub Release

Use the provided script to create GitHub releases directly from the RELEASE_NOTES.md file:

```bash
# Create a GitHub release for a specific tag
./scripts/create_github_release.sh v0.1.1 "Cloud Deployment Update"
```

The script extracts the release notes for the specified version from RELEASE_NOTES.md and creates a GitHub release with the extracted content.

### Updating Release Notes

When adding new features or making significant changes:

1. Update the RELEASE_NOTES.md file with a new version section at the top
2. Commit and push the changes
3. Tag the commit with the new version number
4. Create a GitHub release using the script