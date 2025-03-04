# Zavora Trading Engine Release Notes

## v0.1.2 (Security Update & Setup Improvements)

### Security Updates
- Upgraded SQLx to 0.8.1 to address security vulnerabilities

### Fixes
- Fixed build issues with missing source files
- Added troubleshooting guide for Docker permissions
- Added Node.js installation instructions for WebSocket tests

## v0.1.1 (Cloud Deployment Update)

### Features
- Complete cloud deployment infrastructure for AWS and GCP
- Kubernetes manifests for all microservices
- Terraform configurations for AWS and GCP infrastructure
- Automated deployment scripts for building and deploying
- Comprehensive deployment documentation

### Components
- K8s manifests for all services
- Terraform modules for cloud resources
- Deployment scripts for CI/CD

### Infrastructure
- EKS/GKE for container orchestration
- RDS/Cloud SQL for PostgreSQL database
- ElastiCache/Memorystore for Redis
- ECR/Artifact Registry for container images

## v0.1.0 (Initial Release)

### Features
- High-performance order matching engine with price-time priority
- Account service with PostgreSQL and in-memory implementations
- Market data service with real-time price information
- API Gateway with REST endpoints and WebSocket support
- Transaction management with ACID guarantees
- Containerized deployment with Docker

### Components
- Matching Engine: Processes limit and market orders
- Account Service: Handles user balances and trade settlements
- Market Data: Provides market statistics and order book data
- API Gateway: Unified interface for client applications
- Common: Shared utilities and data models

### Architecture
- Microservices design with Rust/Tokio
- PostgreSQL for persistent storage
- WebSocket for real-time updates

## Development Roadmap
- Performance optimizations
- Additional order types
- Enhanced market data analytics
- Improved error handling and logging