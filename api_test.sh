#!/bin/bash
# API test script - run some direct database commands to test the database

# Configuration
DB_HOST=${DB_HOST:-"localhost"}
DB_PORT=${DB_PORT:-"5435"}
DB_USER=${DB_USER:-"viabtc"}
DB_PASS=${DB_PASS:-"viabtc"}
DB_NAME=${DB_NAME:-"viabtc"}

# Check if psql is available
if ! command -v psql &> /dev/null; then
    echo "Error: PostgreSQL client (psql) is not installed."
    echo "Please install PostgreSQL client to run this script."
    exit 1
fi

# Helper function to execute SQL queries
execute_sql() {
    local query=$1
    PGPASSWORD=$DB_PASS psql -h $DB_HOST -p $DB_PORT -U $DB_USER -d $DB_NAME -c "$query"
}

# Test data
USER_ID="user123"
MARKET_ID="btc_usdt"
ACCOUNT_ID=$(uuidgen 2>/dev/null || echo "00000000-0000-0000-0000-000000000001")
ORDER_ID=$(uuidgen 2>/dev/null || echo "00000000-0000-0000-0000-000000000002")

echo "=== Direct Database Access Tests ==="
echo ""

echo "1. Creating a new account"
execute_sql "INSERT INTO accounts (id, user_id, created_at, updated_at) 
             VALUES ('$ACCOUNT_ID', '$USER_ID', NOW(), NOW()) 
             RETURNING id, user_id;"

echo ""
echo "2. Retrieving the account"
execute_sql "SELECT * FROM accounts WHERE id = '$ACCOUNT_ID';"

echo ""
echo "3. Creating a new market"
execute_sql "INSERT INTO markets (id, base_asset, quote_asset, min_price, max_price, tick_size, min_quantity, max_quantity, step_size)
             VALUES ('$MARKET_ID', 'BTC', 'USDT', '0.01', '1000000', '0.01', '0.001', '1000', '0.001')
             ON CONFLICT (id) DO UPDATE SET 
             base_asset = EXCLUDED.base_asset,
             quote_asset = EXCLUDED.quote_asset,
             updated_at = NOW()
             RETURNING id, base_asset, quote_asset;"

echo ""
echo "4. Creating a buy order"
BUY_ORDER_ID=$(uuidgen 2>/dev/null || echo "00000000-0000-0000-0000-000000000003")
execute_sql "INSERT INTO orders (id, account_id, market_id, side, order_type, price, quantity, filled_quantity, status)
             VALUES ('$BUY_ORDER_ID', '$ACCOUNT_ID', '$MARKET_ID', 0, 0, '40000', '0.1', '0', 0)
             RETURNING id, market_id, side, price, quantity;"

echo ""
echo "5. Creating a sell order"
SELL_ORDER_ID=$(uuidgen 2>/dev/null || echo "00000000-0000-0000-0000-000000000004")
execute_sql "INSERT INTO orders (id, account_id, market_id, side, order_type, price, quantity, filled_quantity, status)
             VALUES ('$SELL_ORDER_ID', '$ACCOUNT_ID', '$MARKET_ID', 1, 0, '41000', '0.05', '0', 0)
             RETURNING id, market_id, side, price, quantity;"

echo ""
echo "6. Listing all orders"
execute_sql "SELECT id, market_id, side, order_type, price, quantity, status FROM orders ORDER BY created_at DESC LIMIT 5;"

echo ""
echo "7. Creating a simulated trade"
TRADE_ID=$(uuidgen 2>/dev/null || echo "00000000-0000-0000-0000-000000000005")
execute_sql "INSERT INTO trades (id, market_id, maker_order_id, taker_order_id, price, quantity)
             VALUES ('$TRADE_ID', '$MARKET_ID', '$SELL_ORDER_ID', '$BUY_ORDER_ID', '40500', '0.025')
             RETURNING id, market_id, price, quantity;"

echo ""
echo "8. Listing recent trades"
execute_sql "SELECT id, market_id, price, quantity, executed_at FROM trades ORDER BY executed_at DESC LIMIT 5;"

echo ""
echo "9. Creating market summary"
execute_sql "INSERT INTO market_summaries (market_id, open_price, high_price, low_price, close_price, volume)
             VALUES ('$MARKET_ID', '40000', '41000', '39500', '40500', '10.5')
             ON CONFLICT (market_id) DO UPDATE SET
             high_price = GREATEST(EXCLUDED.high_price, market_summaries.high_price),
             low_price = LEAST(EXCLUDED.low_price, market_summaries.low_price),
             close_price = EXCLUDED.close_price,
             volume = EXCLUDED.volume,
             updated_at = NOW()
             RETURNING market_id, open_price, high_price, low_price, close_price, volume;"

echo ""
echo "10. Getting market summary"
execute_sql "SELECT * FROM market_summaries WHERE market_id = '$MARKET_ID';"

echo ""
echo "=== Database tests completed ==="

# Generate curl commands for REST API testing (when the API is running)
echo ""
echo "=== REST API Test Commands ==="
echo "Once your API is running, you can use these curl commands to test it:"
echo ""

echo "# Get account"
echo "curl -s -X GET \"http://localhost:8080/api/accounts/$ACCOUNT_ID\""
echo ""

echo "# Get market"
echo "curl -s -X GET \"http://localhost:8080/api/markets/$MARKET_ID\""
echo ""

echo "# Create a new buy order"
echo "curl -s -X POST \"http://localhost:8080/api/orders\" \\
    -H \"Content-Type: application/json\" \\
    -d '{
        \"account_id\": \"$ACCOUNT_ID\",
        \"market_id\": \"$MARKET_ID\",
        \"side\": \"buy\",
        \"order_type\": \"limit\",
        \"price\": \"40000\",
        \"quantity\": \"0.1\"
    }'"
echo ""

echo "# Get order book"
echo "curl -s -X GET \"http://localhost:8080/api/markets/$MARKET_ID/orderbook\""
echo ""

echo "# Get recent trades"
echo "curl -s -X GET \"http://localhost:8080/api/markets/$MARKET_ID/trades\""
echo ""