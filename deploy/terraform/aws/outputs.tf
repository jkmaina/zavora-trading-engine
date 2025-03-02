output "eks_cluster_endpoint" {
  description = "EKS cluster endpoint"
  value       = module.eks.cluster_endpoint
}

output "eks_cluster_name" {
  description = "EKS cluster name"
  value       = module.eks.cluster_name
}

output "rds_hostname" {
  description = "RDS instance hostname"
  value       = module.rds.db_instance_address
  sensitive   = true
}

output "ecr_repository_urls" {
  description = "ECR repository URLs"
  value = {
    for name in ["trading-engine", "matching-engine", "account-service", "market-data", "api-gateway"] :
    name => module.ecr[name].repository_url
  }
}