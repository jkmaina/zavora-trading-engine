provider "google" {
  project = var.project_id
  region  = var.region
}

resource "google_container_cluster" "primary" {
  name     = "zavora-${var.environment}"
  location = var.region
  
  # We can't create a cluster with no node pool defined, but we want to only use
  # separately managed node pools. So we create the smallest possible default
  # node pool and immediately delete it.
  remove_default_node_pool = true
  initial_node_count       = 1
  
  networking_mode = "VPC_NATIVE"
  network         = google_compute_network.vpc.name
  subnetwork      = google_compute_subnetwork.subnet.name
  
  ip_allocation_policy {
    cluster_ipv4_cidr_block  = "/16"
    services_ipv4_cidr_block = "/22"
  }
}

resource "google_container_node_pool" "primary_nodes" {
  name       = "primary-node-pool"
  location   = var.region
  cluster    = google_container_cluster.primary.name
  node_count = 3
  
  node_config {
    oauth_scopes = [
      "https://www.googleapis.com/auth/logging.write",
      "https://www.googleapis.com/auth/monitoring",
      "https://www.googleapis.com/auth/devstorage.read_only",
    ]
    
    labels = {
      env = var.environment
    }
    
    machine_type = "e2-standard-2"
    disk_size_gb = 100
    
    metadata = {
      disable-legacy-endpoints = "true"
    }
  }
}

resource "google_compute_network" "vpc" {
  name                    = "zavora-vpc"
  auto_create_subnetworks = false
}

resource "google_compute_subnetwork" "subnet" {
  name          = "zavora-subnet"
  region        = var.region
  network       = google_compute_network.vpc.name
  ip_cidr_range = "10.2.0.0/16"
  
  secondary_ip_range {
    range_name    = "services-range"
    ip_cidr_range = "10.1.0.0/16"
  }
  
  secondary_ip_range {
    range_name    = "pod-ranges"
    ip_cidr_range = "10.0.0.0/16"
  }
}

resource "google_sql_database_instance" "instance" {
  name             = "zavora-postgres-${var.environment}"
  database_version = "POSTGRES_15"
  region           = var.region
  
  settings {
    tier = "db-f1-micro"
    
    ip_configuration {
      ipv4_enabled    = true
      private_network = google_compute_network.vpc.id
    }
  }
  
  deletion_protection = true
}

resource "google_sql_database" "database" {
  name     = "zavora"
  instance = google_sql_database_instance.instance.name
}

resource "google_sql_user" "users" {
  name     = var.db_username
  instance = google_sql_database_instance.instance.name
  password = var.db_password
}

resource "google_redis_instance" "cache" {
  name           = "zavora-redis-${var.environment}"
  tier           = "BASIC"
  memory_size_gb = 1
  
  region                  = var.region
  authorized_network      = google_compute_network.vpc.id
  connect_mode            = "PRIVATE_SERVICE_ACCESS"
  redis_version           = "REDIS_7_0"
  display_name            = "Zavora Redis Instance"
  
  depends_on = [google_service_networking_connection.private_vpc_connection]
}

resource "google_compute_global_address" "private_ip_address" {
  name          = "zavora-private-ip"
  purpose       = "VPC_PEERING"
  address_type  = "INTERNAL"
  prefix_length = 16
  network       = google_compute_network.vpc.id
}

resource "google_service_networking_connection" "private_vpc_connection" {
  network                 = google_compute_network.vpc.id
  service                 = "servicenetworking.googleapis.com"
  reserved_peering_ranges = [google_compute_global_address.private_ip_address.name]
}

resource "google_artifact_registry_repository" "repo" {
  for_each = toset([
    "trading-engine", 
    "matching-engine", 
    "account-service", 
    "market-data", 
    "api-gateway"
  ])
  
  location      = var.region
  repository_id = "zavora-${each.key}"
  description   = "Docker repository for ${each.key}"
  format        = "DOCKER"
}