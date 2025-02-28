#!/bin/bash
# Script to test the API endpoints

API_URL="http://localhost:8081/api/v1"

echo "Testing API health endpoint..."
curl -s -X GET "$API_URL/health" | jq .

echo "Getting markets..."
curl -s -X GET "$API_URL/markets" | jq .

echo "Creating a new account..."
ACCOUNT_RESPONSE=$(curl -s -X POST "$API_URL/accounts" -H "Content-Type: application/json" -d '{}')
echo $ACCOUNT_RESPONSE | jq .

# Extract account ID
ACCOUNT_ID=$(echo $ACCOUNT_RESPONSE | jq -r '.account.id')

echo "Account ID: $ACCOUNT_ID"

echo "Depositing funds..."
curl -s -X POST "$API_URL/accounts/$ACCOUNT_ID/deposit" \
    -H "Content-Type: application/json" \
    -d '{
        "asset": "BTC",
        "amount": "1.0"
    }' | jq .

echo "Getting balances..."
curl -s -X GET "$API_URL/accounts/$ACCOUNT_ID/balances" | jq .

echo "Placing a limit order..."
curl -s -X POST "$API_URL/orders" \
    -H "Content-Type: application/json" \
    -d '{
        "user_id": "'$ACCOUNT_ID'",
        "market": "BTC/USD",
        "side": "sell",
        "order_type": "limit",
        "price": "20000",
        "quantity": "0.1"
    }' | jq .

echo "Testing complete!"