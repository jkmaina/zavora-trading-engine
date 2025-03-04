# Zavora Trading Engine - Deployment Guide

This guide explains how to deploy the Zavora Trading Engine to AWS or GCP.

## Prerequisites

### For all deployments
- Terraform >= 1.0.0
- kubectl >= 1.20
- Docker

### For AWS deployment
- AWS CLI configured with appropriate permissions
- IAM permissions for EKS, ECR, RDS, ElastiCache

### For GCP deployment
- Google Cloud SDK configured with appropriate permissions
- IAM permissions for GKE, Artifact Registry, Cloud SQL, Memorystore

## Deployment Steps

### 1. Infrastructure Provisioning

#### AWS

```bash
cd deploy/terraform/aws

# Initialize Terraform
terraform init

# Create a terraform.tfvars file with your variables
cat > terraform.tfvars << EOF
region      = "us-west-2"
environment = "dev"
db_username = "zavora"
db_password = "your-password"
EOF

# Plan the deployment
terraform plan -out=tfplan

# Apply the infrastructure
terraform apply tfplan

# Save the outputs for later use
export REGISTRY_URL=$(terraform output -raw ecr_repository_urls | jq -r '.["api-gateway"]' | sed 's|/zavora/api-gateway||')
```

#### GCP

```bash
cd deploy/terraform/gcp

# Initialize Terraform
terraform init

# Create a terraform.tfvars file with your variables
cat > terraform.tfvars << EOF
project_id  = "your-project-id"
region      = "us-central1"
environment = "dev"
db_username = "zavora"
db_password = "your-secure-password"
EOF

# Plan the deployment
terraform plan -out=tfplan

# Apply the infrastructure
terraform apply tfplan

# Save the outputs for later use
export REGISTRY_URL=$(terraform output -raw artifact_registry_repos | jq -r '.["api-gateway"]' | sed 's|/zavora-api-gateway||')
```

### 2. Build and Push Docker Images

```bash
cd ../../..  # Navigate to project root

# For AWS
./deploy/scripts/build_and_push.sh aws $REGISTRY_URL v0.1.0

# For GCP
./deploy/scripts/build_and_push.sh gcp $REGISTRY_URL v0.1.0
```

### 3. Deploy to Kubernetes

```bash
# For AWS
./deploy/scripts/deploy.sh aws $REGISTRY_URL v0.1.0 dev

# For GCP
./deploy/scripts/deploy.sh gcp $REGISTRY_URL v0.1.0 dev
```

## Accessing the Deployment

After deployment, the script will output a URL where the API gateway can be accessed.

## Monitoring and Maintenance

### Logs

```bash
# View logs for a specific service
kubectl logs -f -l app=api-gateway -n zavora
kubectl logs -f -l app=trading-engine -n zavora
```

### Scaling

```bash
# Scale a deployment
kubectl scale deployment/api-gateway --replicas=3 -n zavora
```

### Updating

To deploy a new version:

1. Build and push new Docker images with a new tag
2. Run the deploy script with the new tag

```bash
./deploy/scripts/build_and_push.sh aws $REGISTRY_URL v0.1.1
./deploy/scripts/deploy.sh aws $REGISTRY_URL v0.1.1 dev
```

## Cleanup

To destroy all resources:

```bash
# For AWS
cd deploy/terraform/aws
terraform destroy

# For GCP
cd deploy/terraform/gcp
terraform destroy
```