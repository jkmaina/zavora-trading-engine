provider "aws" {
  region = var.region
}

module "vpc" {
  source = "terraform-aws-modules/vpc/aws"
  
  name = "zavora-vpc"
  cidr = "10.0.0.0/16"
  
  azs             = ["${var.region}a", "${var.region}b", "${var.region}c"]
  private_subnets = ["10.0.1.0/24", "10.0.2.0/24", "10.0.3.0/24"]
  public_subnets  = ["10.0.101.0/24", "10.0.102.0/24", "10.0.103.0/24"]
  
  enable_nat_gateway = true
  single_nat_gateway = true
  
  tags = {
    Environment = var.environment
    Project     = "zavora"
  }
}

module "eks" {
  source = "terraform-aws-modules/eks/aws"
  
  cluster_name    = "zavora-${var.environment}"
  cluster_version = "1.28"
  
  vpc_id          = module.vpc.vpc_id
  subnet_ids      = module.vpc.private_subnets
  
  cluster_endpoint_public_access = true
  
  eks_managed_node_groups = {
    default = {
      min_size     = 2
      max_size     = 5
      desired_size = 3
      
      instance_types = ["t3.medium"]
      capacity_type  = "ON_DEMAND"
    }
  }
  
  tags = {
    Environment = var.environment
    Project     = "zavora"
  }
}

module "rds" {
  source  = "terraform-aws-modules/rds/aws"
  
  identifier = "zavora-postgres-${var.environment}"
  
  engine            = "postgres"
  engine_version    = "15"
  instance_class    = "db.t3.medium"
  allocated_storage = 20
  
  db_name     = "zavora"
  username    = var.db_username
  password    = var.db_password
  port        = 5432
  
  # Fixed configuration for VPC
  vpc_security_group_ids = [aws_security_group.rds.id]
  create_db_subnet_group = true
  db_subnet_group_name   = "zavora-db-subnet-group"
  subnet_ids             = module.vpc.private_subnets
  
  family               = "postgres15"
  major_engine_version = "15"
  
  tags = {
    Environment = var.environment
    Project     = "zavora"
  }
}

module "elasticache" {
  source = "terraform-aws-modules/elasticache/aws"
  
  cluster_id           = "zavora-redis-${var.environment}"
  replication_group_id = "zavora-redis-${var.environment}"
  engine               = "redis"
  engine_version       = "7.0"
  node_type            = "cache.t3.small"
  num_cache_nodes      = 1
  
  # Fixed configuration to avoid duplicate subnet group
  subnet_group_name   = "zavora-redis-subnet-group"
  create_subnet_group = true
  subnet_ids          = module.vpc.private_subnets
  vpc_id              = module.vpc.vpc_id 

  security_group_ids  = [aws_security_group.redis.id]
  
  tags = {
    Environment = var.environment
    Project     = "zavora"
  }
}

resource "aws_security_group" "rds" {
  name        = "zavora-rds-sg-${var.environment}"
  description = "Allow PostgreSQL inbound traffic from EKS"
  vpc_id      = module.vpc.vpc_id
  
  ingress {
    from_port       = 5432
    to_port         = 5432
    protocol        = "tcp"
    security_groups = [module.eks.cluster_security_group_id]
  }
  
  egress {
    from_port   = 0
    to_port     = 0
    protocol    = "-1"
    cidr_blocks = ["0.0.0.0/0"]
  }
  
  tags = {
    Environment = var.environment
    Project     = "zavora"
  }
}

resource "aws_security_group" "redis" {
  name        = "zavora-redis-sg-${var.environment}"
  description = "Allow Redis inbound traffic from EKS"
  vpc_id      = module.vpc.vpc_id
  
  ingress {
    from_port       = 6379
    to_port         = 6379
    protocol        = "tcp"
    security_groups = [module.eks.cluster_security_group_id]
  }
  
  egress {
    from_port   = 0
    to_port     = 0
    protocol    = "-1"
    cidr_blocks = ["0.0.0.0/0"]
  }
  
  tags = {
    Environment = var.environment
    Project     = "zavora"
  }
}

module "ecr" {
  source = "terraform-aws-modules/ecr/aws"
  
  for_each = toset([
    "trading-engine", 
    "matching-engine", 
    "account-service", 
    "market-data", 
    "api-gateway"
  ])
  
  repository_name = "zavora/${each.key}"
  
  repository_lifecycle_policy = jsonencode({
    rules = [
      {
        rulePriority = 1,
        description  = "Keep last 30 images",
        selection = {
          tagStatus     = "any",
          countType     = "imageCountMoreThan",
          countNumber   = 30
        },
        action = {
          type = "expire"
        }
      }
    ]
  })
  
  tags = {
    Environment = var.environment
    Project     = "zavora"
  }
}