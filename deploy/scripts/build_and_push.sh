#!/bin/bash
set -e

# Script to build and push Docker images
# Usage: ./build_and_push.sh [aws|gcp] [registry_url] [tag]

if [ $# -lt 3 ]; then
  echo "Usage: $0 [aws|gcp] [registry_url] [tag]"
  echo "Example for AWS: $0 aws 123456789012.dkr.ecr.us-west-2.amazonaws.com v0.1.0"
  echo "Example for GCP: $0 gcp us-central1-docker.pkg.dev/my-project-id v0.1.0"
  exit 1
fi

CLOUD_PROVIDER=$1
REGISTRY_URL=$2
TAG=$3

# Current directory should be the project root
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "${SCRIPT_DIR}/../.." && pwd)"
cd "${PROJECT_ROOT}"

# List of services to build
SERVICES=("trading-engine" "account-service" "market-data" "matching-engine" "api-gateway")

# Login to AWS ECR or Google Artifact Registry
if [ "$CLOUD_PROVIDER" == "aws" ]; then
  echo "Logging in to AWS ECR..."
  aws ecr get-login-password --region $(echo $REGISTRY_URL | cut -d'.' -f4) | \
    docker login --username AWS --password-stdin $REGISTRY_URL
elif [ "$CLOUD_PROVIDER" == "gcp" ]; then
  echo "Logging in to Google Artifact Registry..."
  gcloud auth configure-docker $(echo $REGISTRY_URL | cut -d'/' -f1) --quiet
else
  echo "Unknown cloud provider: $CLOUD_PROVIDER"
  exit 1
fi

# Build and push each service
for SERVICE in "${SERVICES[@]}"; do
  echo "Building and pushing ${SERVICE}..."
  
  # Build the Docker image
  docker build -t ${REGISTRY_URL}/zavora/${SERVICE}:${TAG} \
    --build-arg SERVICE=${SERVICE} \
    -f Dockerfile .
  
  # Push the Docker image
  docker push ${REGISTRY_URL}/zavora/${SERVICE}:${TAG}
  
  echo "${SERVICE} built and pushed successfully!"
done

echo "All services built and pushed successfully!"