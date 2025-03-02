# Database Testing Guide for Zavora Trading Engine

This document describes how to set up and run database tests for the Zavora Trading Engine.

## Database Overview

The Zavora Trading Engine uses PostgreSQL for persistence with the following setup:

1. **Production Database**: 
   - Runs on port 5435 (host) mapping to port 5432 (container)
   - Used for development and running the application
   - Contains real data for the application

2. **Test Database**:
   - Runs on port 5434 (host) mapping to port 5432 (container)
   - Used for automated tests
   - Gets reset between test runs

Both databases automatically apply the schema migrations during startup.

## Prerequisites

Before running the database tests, you need:

1. Docker and Docker Compose installed (recommended)
2. Or a local PostgreSQL instance (alternative setup)

## Setting up the Databases

### Production Database Setup

```bash
# Start the production PostgreSQL database in Docker
docker compose up -d postgres

# Verify it's running
docker compose ps

# Populate it with test data
./api_test.sh
```

The production database will be accessible at:
- Host: localhost
- Port: 5435
- User: viabtc
- Password: viabtc
- Database: viabtc
- Connection URL: `postgres://viabtc:viabtc@localhost:5435/viabtc`

### Test Database Setup

```bash
# Run the provided script to set up a Docker-based test database
./create_test_db.sh

# This will:
# 1. Start a PostgreSQL container on port 5434
# 2. Create the viabtc_test database
# 3. Apply all migrations from the migrations folder
# 4. Set the TEST_DATABASE_URL in .env.test
```

The test database will be accessible at:
- Host: localhost
- Port: 5434
- User: viabtc
- Password: viabtc
- Database: viabtc_test
- Connection URL: `postgres://viabtc:viabtc@localhost:5434/viabtc_test`

### Manual Test Database Setup (Alternative)

If you prefer, you can set up a test database manually with a local PostgreSQL instance:

```bash
# Set variables for database connection
export DB_USER=viabtc
export DB_PASS=viabtc
export DB_HOST=localhost
export DB_PORT=5432
export TEST_DB_NAME=viabtc_test

# Create the test database
createdb -h $DB_HOST -p $DB_PORT -U $DB_USER $TEST_DB_NAME

# Apply migrations (assuming you have psql installed)
psql -h $DB_HOST -p $DB_PORT -U $DB_USER -d $TEST_DB_NAME -f migrations/20240227000000_initial_schema.sql

# Set environment variable for tests
export TEST_DATABASE_URL="postgres://$DB_USER:$DB_PASS@$DB_HOST:$DB_PORT/$TEST_DB_NAME"
```

## Running the Tests

### Database Integration Tests

These tests verify the persistence layer and database interactions:

```bash
# If using the Docker setup or .env.test file:
source .env.test && cargo test --test db_tests -- --ignored

# Or specify the database URL directly:
TEST_DATABASE_URL="postgres://viabtc:viabtc@localhost:5434/viabtc_test" cargo test --test db_tests -- --ignored
```

### Testing the Production Database

To test the production database directly and populate it with test data:

```bash
# Run the API test script
./api_test.sh
```

This script will:
1. Create test accounts, markets, orders, and trades
2. Generate market summaries
3. Output curl commands for testing the REST API

## Test Types

### Automated Database Tests

The database persistence tests (`tests/db_tests.rs`) include:

1. **Basic Operations**: Tests inserting, selecting, and deleting records
2. **Multiple Rows**: Tests handling multiple rows of data
3. **Transactions**: Tests transaction commit and rollback functionality

### Manual API Tests

Once your API is running, you can use the curl commands output by `api_test.sh` to test the REST endpoints:

```bash
# Get market information
curl -s -X GET "http://localhost:8080/api/markets/btc_usdt"

# Get order book
curl -s -X GET "http://localhost:8080/api/markets/btc_usdt/orderbook"

# Create a new order
curl -s -X POST "http://localhost:8080/api/orders" \
    -H "Content-Type: application/json" \
    -d '{
        "account_id": "ACCOUNT_ID",
        "market_id": "btc_usdt",
        "side": "buy",
        "order_type": "limit",
        "price": "40000",
        "quantity": "0.1"
    }'
```

## Adding Your Own Tests

To add your own database persistence tests:

1. Add a new test function in `/tests/db_tests.rs`
2. Use the `run_db_test` helper function to manage database connections
3. Use SQL tables for testing (prefer regular tables over temporary tables)

Example:

```rust
#[test]
#[ignore = "Requires test database, run with RUST_TEST_THREADS=1 cargo test -- --ignored"]
fn test_your_feature() {
    run_db_test(|pool| {
        Box::pin(async move {
            // Create test table
            sqlx::query("
                CREATE TABLE IF NOT EXISTS your_test_table (
                    id SERIAL PRIMARY KEY,
                    name TEXT NOT NULL
                )
            ")
            .execute(&pool)
            .await
            .expect("Failed to create table");
            
            // Your test code here
            
            // Clean up
            sqlx::query("DROP TABLE IF EXISTS your_test_table")
                .execute(&pool)
                .await
                .expect("Failed to drop table");
        })
    });
}
```

## Docker Database Management

### Production Database Commands

```bash
# Start production database
docker compose up -d postgres

# Stop production database
docker compose stop postgres

# Remove production database (deletes all data)
docker compose down -v

# Execute SQL on production database
docker exec zavora-trading-engine-postgres-1 psql -U viabtc -d viabtc -c "SELECT * FROM markets;"
```

### Test Database Commands

```bash
# Start test database
./create_test_db.sh

# Stop test database
docker compose stop postgres_test

# Remove test database (deletes all data)
docker compose down -v

# Execute SQL on test database
docker exec zavora-trading-engine-postgres_test-1 psql -U viabtc -d viabtc_test -c "SELECT * FROM markets;"
```

## Troubleshooting

1. **Port conflicts**: If you see errors about ports already in use, modify the port mappings in `docker-compose.yml`
2. **Database connection failures**: Ensure the Docker containers are running with `docker compose ps`
3. **Missing schema**: The schema migrations should be applied automatically, but you can manually run them using psql

## Best Practices

1. Always clean up after tests to avoid polluting the test database
2. Use transactions for tests that require multiple operations
3. Make tests independent of each other
4. Use descriptive test names that explain what is being tested
5. For CI/CD, consider using Docker for a consistent test environment
6. Regularly reset your test database to ensure clean test runs