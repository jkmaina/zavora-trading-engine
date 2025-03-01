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

See the specific provider directories for detailed instructions.