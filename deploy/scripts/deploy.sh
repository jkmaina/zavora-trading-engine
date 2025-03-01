#!/bin/bash
set -e

# Script to deploy Zavora Trading Engine to Kubernetes
# Usage: ./deploy.sh [aws|gcp] [registry_url] [tag] [env]

if [ $# -lt 4 ]; then
  echo "Usage: $0 [aws|gcp] [registry_url] [tag] [env]"
  echo "Example for AWS: $0 aws 123456789012.dkr.ecr.us-west-2.amazonaws.com v0.1.0 dev"
  echo "Example for GCP: $0 gcp us-central1-docker.pkg.dev/my-project-id v0.1.0 dev"
  exit 1
fi

CLOUD_PROVIDER=$1
REGISTRY_URL=$2
TAG=$3
ENV=$4

# Current directory should be the project root
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "${SCRIPT_DIR}/../.." && pwd)"
DEPLOY_DIR="${PROJECT_ROOT}/deploy"
cd "${DEPLOY_DIR}"

# Connect to the Kubernetes cluster
if [ "$CLOUD_PROVIDER" == "aws" ]; then
  echo "Connecting to EKS cluster..."
  aws eks update-kubeconfig --name zavora-${ENV} --region $(echo $REGISTRY_URL | cut -d'.' -f4)
elif [ "$CLOUD_PROVIDER" == "gcp" ]; then
  echo "Connecting to GKE cluster..."
  gcloud container clusters get-credentials zavora-${ENV} --region $(echo $REGISTRY_URL | cut -d'-' -f1)
else
  echo "Unknown cloud provider: $CLOUD_PROVIDER"
  exit 1
fi

# Create overlay directory if it doesn't exist
mkdir -p "${DEPLOY_DIR}/k8s/overlays/${ENV}"

# Create kustomization.yaml in overlay directory
cat > "${DEPLOY_DIR}/k8s/overlays/${ENV}/kustomization.yaml" << EOF
apiVersion: kustomize.config.k8s.io/v1beta1
kind: Kustomization

resources:
  - ../../base

namespace: zavora

patches:
  - path: registry-patch.yaml
EOF

# Create registry patch to update image URLs
cat > "${DEPLOY_DIR}/k8s/overlays/${ENV}/registry-patch.yaml" << EOF
apiVersion: apps/v1
kind: Deployment
metadata:
  name: api-gateway
spec:
  template:
    spec:
      containers:
      - name: api-gateway
        image: ${REGISTRY_URL}/zavora/api-gateway:${TAG}
---
apiVersion: apps/v1
kind: Deployment
metadata:
  name: account-service
spec:
  template:
    spec:
      containers:
      - name: account-service
        image: ${REGISTRY_URL}/zavora/account-service:${TAG}
---
apiVersion: apps/v1
kind: Deployment
metadata:
  name: market-data
spec:
  template:
    spec:
      containers:
      - name: market-data
        image: ${REGISTRY_URL}/zavora/market-data:${TAG}
---
apiVersion: apps/v1
kind: Deployment
metadata:
  name: matching-engine
spec:
  template:
    spec:
      containers:
      - name: matching-engine
        image: ${REGISTRY_URL}/zavora/matching-engine:${TAG}
---
apiVersion: apps/v1
kind: Deployment
metadata:
  name: trading-engine
spec:
  template:
    spec:
      containers:
      - name: trading-engine
        image: ${REGISTRY_URL}/zavora/trading-engine:${TAG}
EOF

# Create Kubernetes secret for postgres
echo "Creating Kubernetes secrets..."

# Prompt for database credentials if not in environment
if [ -z "$DB_USERNAME" ]; then
  read -p "Enter database username: " DB_USERNAME
fi

if [ -z "$DB_PASSWORD" ]; then
  read -s -p "Enter database password: " DB_PASSWORD
  echo
fi

kubectl create namespace zavora --dry-run=client -o yaml | kubectl apply -f -

kubectl create secret generic postgres-secret \
  --namespace zavora \
  --from-literal=username=$DB_USERNAME \
  --from-literal=password=$DB_PASSWORD \
  --dry-run=client -o yaml | kubectl apply -f -

# Deploy using kustomize
echo "Deploying Zavora Trading Engine..."
kubectl apply -k "${DEPLOY_DIR}/k8s/overlays/${ENV}"

# Wait for deployments to be ready
echo "Waiting for deployments to be ready..."
kubectl wait --for=condition=available --timeout=300s --namespace zavora deployment/api-gateway
kubectl wait --for=condition=available --timeout=300s --namespace zavora deployment/account-service
kubectl wait --for=condition=available --timeout=300s --namespace zavora deployment/market-data
kubectl wait --for=condition=available --timeout=300s --namespace zavora deployment/matching-engine
kubectl wait --for=condition=available --timeout=300s --namespace zavora deployment/trading-engine

echo "Zavora Trading Engine deployed successfully!"

# Create Ingress if it doesn't exist
if ! kubectl get ingress zavora-ingress -n zavora &> /dev/null; then
  echo "Creating Ingress..."
  if [ "$CLOUD_PROVIDER" == "aws" ]; then
    # For AWS ALB Ingress Controller
    cat > "${DEPLOY_DIR}/k8s/overlays/${ENV}/ingress-aws.yaml" << EOF
apiVersion: networking.k8s.io/v1
kind: Ingress
metadata:
  name: zavora-ingress
  namespace: zavora
  annotations:
    kubernetes.io/ingress.class: alb
    alb.ingress.kubernetes.io/scheme: internet-facing
    alb.ingress.kubernetes.io/target-type: ip
spec:
  rules:
  - http:
      paths:
      - path: /
        pathType: Prefix
        backend:
          service:
            name: api-gateway
            port:
              number: 80
EOF
    kubectl apply -f "${DEPLOY_DIR}/k8s/overlays/${ENV}/ingress-aws.yaml"
  elif [ "$CLOUD_PROVIDER" == "gcp" ]; then
    # For GCP Ingress
    cat > "${DEPLOY_DIR}/k8s/overlays/${ENV}/ingress-gcp.yaml" << EOF
apiVersion: networking.k8s.io/v1
kind: Ingress
metadata:
  name: zavora-ingress
  namespace: zavora
spec:
  defaultBackend:
    service:
      name: api-gateway
      port:
        number: 80
EOF
    kubectl apply -f "${DEPLOY_DIR}/k8s/overlays/${ENV}/ingress-gcp.yaml"
  fi
fi

echo "Getting ingress URL..."
if [ "$CLOUD_PROVIDER" == "aws" ]; then
  # For AWS
  sleep 30  # Wait for ALB to be provisioned
  INGRESS_URL=$(kubectl get ingress zavora-ingress -n zavora -o jsonpath='{.status.loadBalancer.ingress[0].hostname}')
elif [ "$CLOUD_PROVIDER" == "gcp" ]; then
  # For GCP
  sleep 30  # Wait for load balancer to be provisioned
  INGRESS_URL=$(kubectl get ingress zavora-ingress -n zavora -o jsonpath='{.status.loadBalancer.ingress[0].ip}')
fi

echo "Zavora Trading Engine is accessible at: http://${INGRESS_URL}"