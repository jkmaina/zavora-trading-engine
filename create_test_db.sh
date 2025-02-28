#!/bin/bash
# Setup test database for database persistence tests using Docker

set -e

# Define database connection parameters for Docker test database
DB_HOST=${DB_HOST:-localhost}
DB_PORT=${DB_PORT:-5434}  # Using port 5434 mapped to Docker container
DB_USER=${DB_USER:-viabtc}
DB_PASS=${DB_PASS:-viabtc}
TEST_DB_NAME=${TEST_DB_NAME:-viabtc_test}

echo "Setting up Docker test database: $TEST_DB_NAME"

# Check if Docker is installed
if ! command -v docker >/dev/null 2>&1; then
    echo "Docker is not installed. Please install Docker and try again."
    exit 1
fi

# Check if containers are already running
if ! docker ps | grep -q "postgres_test"; then
    echo "Starting PostgreSQL test container with docker compose..."
    docker compose up -d postgres_test
    
    # Wait for PostgreSQL to be ready
    echo "Waiting for PostgreSQL test container to be ready..."
    sleep 5
    
    # Check up to 10 times if PostgreSQL is ready
    for i in {1..10}; do
        if docker exec $(docker ps -q -f name=postgres_test) pg_isready -h localhost -U viabtc; then
            echo "PostgreSQL test container is ready."
            break
        fi
        
        if [ $i -eq 10 ]; then
            echo "PostgreSQL test container failed to start. Check docker logs."
            exit 1
        fi
        
        echo "Waiting for PostgreSQL test container to be ready (attempt $i of 10)..."
        sleep 2
    done
else
    echo "PostgreSQL test container is already running."
fi

# Set TEST_DATABASE_URL for the tests
export TEST_DATABASE_URL="postgres://$DB_USER:$DB_PASS@$DB_HOST:$DB_PORT/$TEST_DB_NAME"
echo "Test database URL: $TEST_DATABASE_URL"
echo "export TEST_DATABASE_URL=\"$TEST_DATABASE_URL\"" > .env.test

echo "Test database setup completed. You can now run database tests with:"
echo "source .env.test && cargo test --test db_tests -- --ignored"