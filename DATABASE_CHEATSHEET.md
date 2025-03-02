# Zavora Trading Engine Database Commands

## Production Database (Port 5435)

```bash
# Start production database
docker compose up -d postgres

# Check status
docker compose ps

# Load test data
./api_test.sh

# Run API gateway using production database
cargo run --bin api-gateway

# Connect directly to the database
docker exec -it zavora-trading-engine-postgres-1 psql -U viabtc -d viabtc
```

## Test Database (Port 5434)

```bash
# Start test database
./create_test_db.sh

# Run database tests
source .env.test && cargo test --test db_tests -- --ignored

# Connect directly to the test database
docker exec -it zavora-trading-engine-postgres_test-1 psql -U viabtc -d viabtc_test
```

## Example API Requests

```bash
# Get market info
curl -s -X GET "http://localhost:8080/api/markets/btc_usdt"

# Get order book
curl -s -X GET "http://localhost:8080/api/markets/btc_usdt/orderbook"

# Get trades
curl -s -X GET "http://localhost:8080/api/markets/btc_usdt/trades"

# Place order (replace ACCOUNT_ID with your actual account ID)
curl -s -X POST "http://localhost:8080/api/orders" \
    -H "Content-Type: application/json" \
    -d '{
        "account_id": "394f729c-c755-4022-a4d7-f9b510de0d58",
        "market_id": "btc_usdt",
        "side": "buy",
        "order_type": "limit",
        "price": "40000",
        "quantity": "0.1"
    }'
```

## Database Management Commands

```bash
# Get a list of all accounts
docker exec zavora-trading-engine-postgres-1 psql -U viabtc -d viabtc -c "SELECT * FROM accounts;"

# Get all markets
docker exec zavora-trading-engine-postgres-1 psql -U viabtc -d viabtc -c "SELECT * FROM markets;"

# Get recent orders
docker exec zavora-trading-engine-postgres-1 psql -U viabtc -d viabtc -c "SELECT * FROM orders ORDER BY created_at DESC LIMIT 5;"

# Get recent trades
docker exec zavora-trading-engine-postgres-1 psql -U viabtc -d viabtc -c "SELECT * FROM trades ORDER BY executed_at DESC LIMIT 5;"

# Backup the production database
docker exec zavora-trading-engine-postgres-1 pg_dump -U viabtc -d viabtc > backup.sql

# Restore the database (if needed)
cat backup.sql | docker exec -i zavora-trading-engine-postgres-1 psql -U viabtc -d viabtc
```

