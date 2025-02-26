#!/bin/bash
# api_test.sh - REST API Test Script for Trading Engine

set -e
BASE_URL="http://localhost:8080/api/v1"
GREEN='\033[0;32m'
RED='\033[0;31m'
NC='\033[0m' # No Color

echo "Testing Trading Engine REST API"
echo "==============================="

# Helper function for testing endpoints
test_endpoint() {
  local endpoint=$1
  local method=${2:-GET}
  local payload=$3
  local expected_status=${4:-200}
  
  echo -n "Testing $method $endpoint... "
  
  # Build curl command
  CURL_CMD="curl -s -X $method -w '%{http_code}' -H 'Content-Type: application/json'"
  
  if [ ! -z "$payload" ]; then
    CURL_CMD="$CURL_CMD -d '$payload'"
  fi
  
  CURL_CMD="$CURL_CMD $BASE_URL$endpoint"
  
  # Execute request
  response=$(eval $CURL_CMD)
  status_code=${response: -3}
  body=${response:0:${#response}-3}
  
  # Check status code
  if [ "$status_code" -eq "$expected_status" ]; then
    echo -e "${GREEN}PASS${NC}"
    # Store the response for later use if needed
    echo "$body" > "/tmp/test_$(echo $endpoint | tr '/' '_')_response.json"
  else
    echo -e "${RED}FAIL${NC} - Expected status $expected_status but got $status_code"
    echo "Response: $body"
    exit 1
  fi
}

# Test 1: Get markets
test_endpoint "/markets"

# Test 2: Create an account
test_endpoint "/accounts" "POST" "{}"
ACCOUNT_ID=$(cat /tmp/test__accounts_response.json | grep -o '"id":"[^"]*' | cut -d'"' -f4)
echo "Created account with ID: $ACCOUNT_ID"

# Test 3: Get account details
test_endpoint "/accounts/$ACCOUNT_ID"

# Test 4: Deposit funds
test_endpoint "/accounts/$ACCOUNT_ID/deposit" "POST" '{"asset":"BTC","amount":"1.0"}'
test_endpoint "/accounts/$ACCOUNT_ID/deposit" "POST" '{"asset":"USD","amount":"50000.0"}'

# Test 5: Get balances
test_endpoint "/accounts/$ACCOUNT_ID/balances"

# Test 6: Get order book
test_endpoint "/markets/BTC%2FUSD/order-book"

# Test 7: Get recent trades
test_endpoint "/markets/BTC%2FUSD/trades"

# Test 8: Place limit buy order
test_endpoint "/orders" "POST" "{\"user_id\":\"$ACCOUNT_ID\",\"market\":\"BTC/USD\",\"side\":\"Buy\",\"order_type\":\"Limit\",\"price\":\"19000.0\",\"quantity\":\"0.1\"}"
BUY_ORDER_ID=$(cat /tmp/test__orders_response.json | grep -o '"id":"[^"]*' | head -1 | cut -d'"' -f4)
echo "Created buy order with ID: $BUY_ORDER_ID"

# Test 9: Place limit sell order
test_endpoint "/orders" "POST" "{\"user_id\":\"$ACCOUNT_ID\",\"market\":\"BTC/USD\",\"side\":\"Sell\",\"order_type\":\"Limit\",\"price\":\"21000.0\",\"quantity\":\"0.05\"}"
SELL_ORDER_ID=$(cat /tmp/test__orders_response.json | grep -o '"id":"[^"]*' | head -1 | cut -d'"' -f4)
echo "Created sell order with ID: $SELL_ORDER_ID"

# Test 10: Get order details
test_endpoint "/orders/$BUY_ORDER_ID"

# Test 11: Cancel order
test_endpoint "/orders/$SELL_ORDER_ID" "POST"

# Test 12: Get tickers
test_endpoint "/markets/tickers"

echo -e "\n${GREEN}All REST API tests passed!${NC}"