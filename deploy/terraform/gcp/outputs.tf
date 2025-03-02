output "gke_cluster_endpoint" {
  description = "GKE cluster endpoint"
  value       = google_container_cluster.primary.endpoint
}

output "gke_cluster_name" {
  description = "GKE cluster name"
  value       = google_container_cluster.primary.name
}

output "sql_instance" {
  description = "Cloud SQL instance connection name"
  value       = google_sql_database_instance.instance.connection_name
}

output "artifact_registry_repos" {
  description = "Artifact Registry repository URLs"
  value = {
    for name in ["trading-engine", "matching-engine", "account-service", "market-data", "api-gateway"] :
    name => "${var.region}-docker.pkg.dev/${var.project_id}/zavora-${name}"
  }
}