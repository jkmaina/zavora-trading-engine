# API Gateway

The API Gateway serves as the unified entry point for the Zavora Trading Engine, providing RESTful HTTP and WebSocket interfaces for clients to interact with the system. It handles request routing, validation, and communication with the internal microservices.

## Features

- **RESTful API**: Standard HTTP endpoints following REST principles
- **WebSocket API**: Real-time data streaming and command interface
- **Request Validation**: Input validation and error handling
- **Authentication**: (Planned) User authentication and authorization
- **Rate Limiting**: (Planned) Protection against excessive requests
- **Logging and Monitoring**: Request tracking and performance metrics
- **Error Handling**: Standardized error responses and problem details
- **Cross-Origin Support**: Configurable CORS for web clients

## Endpoints

### Health Check

- `GET /api/v1/health` - API server status check

### Account Management

- `POST /api/v1/accounts` - Create a new account
- `GET /api/v1/accounts/:id` - Get account details
- `GET /api/v1/accounts/:id/balances` - Get account balances
- `POST /api/v1/accounts/:id/deposit` - Deposit funds
- `POST /api/v1/accounts/:id/withdraw` - Withdraw funds

### Market Data

- `GET /api/v1/markets` - List all markets
- `GET /api/v1/markets/:market/order-book` - Get market order book
- `GET /api/v1/markets/:market/ticker` - Get market ticker
- `GET /api/v1/markets/:market/trades` - Get recent trades
- `GET /api/v1/markets/:market/candles` - Get OHLCV candles
- `GET /api/v1/markets/tickers` - Get all market tickers

### Order Management

- `POST /api/v1/orders` - Place a new order
- `GET /api/v1/orders/:id` - Get order details
- `POST /api/v1/orders/:id` - Cancel an order
- `GET /api/v1/accounts/:id/orders` - List account orders

### WebSocket

- `WebSocket /ws` - WebSocket connection for real-time data and commands

## Architecture

### AppState

The API Gateway uses a shared application state to provide access to all required services:

```rust
pub struct AppState {
    /// Matching engine
    pub matching_engine: Arc<MatchingEngine>,
    /// Account service
    pub account_service: Arc<AccountService>,
    /// Market data service
    pub market_data_service: Arc<MarketDataService>,
    /// Available markets
    pub markets: Vec<Market>,
}
```

This state is shared across all API handlers using Axum's state management.

### REST API Handlers

API endpoints are implemented using Axum's routing and handler system:

```rust
let api_routes = axum::Router::new()
    // Account routes
    .route("/accounts", axum::routing::post(api::account::create_account))
    .route("/accounts/:id", axum::routing::get(api::account::get_account))
    // Market routes
    .route("/markets", axum::routing::get(api::market::get_markets))
    // Order routes
    .route("/orders", axum::routing::post(api::order::place_order));
```

Each handler is an async function that follows a standard structure with consistent parameter ordering:

```rust
pub async fn create_account(
    State(state): State<Arc<AppState>>,          // State always comes first
    Json(request): Json<CreateAccountRequest>,   // Request body comes last
) -> Result<ApiResponse<Account>, ApiError> {    // Standardized return type
    // Call the service
    let account = state.account_service.create_account().await
        .map_err(ApiError::Common)?;
    
    // Return a standardized response
    Ok(ApiResponse::new(account))
}
```

### WebSocket Handlers

The WebSocket interface enables real-time communication:

```rust
pub async fn ws_handler(
    State(state): State<Arc<AppState>>,
    ws: WebSocketUpgrade,
) -> impl IntoResponse {
    ws.on_upgrade(|socket| handle_socket(socket, state))
}
```

WebSocket messages follow a standard format:

```json
{
  "op": "subscribe",
  "channel": "orderbook",
  "market": "BTC/USD"
}
```

### Standardized Responses

All API responses follow a consistent format to improve predictability for clients:

#### Single Resource Response

```json
{
  "data": {
    "id": "123e4567-e89b-12d3-a456-426614174000",
    "property1": "value1",
    "property2": "value2"
  },
  "meta": {
    "request_id": "7f5d0fde-9c9a-4b6a-8c1a-6b5f3c5e1d2a"
  }
}
```

#### Collection Response

```json
{
  "data": [
    {
      "id": "123e4567-e89b-12d3-a456-426614174000",
      "property1": "value1"
    },
    {
      "id": "223e4567-e89b-12d3-a456-426614174000",
      "property1": "value2"
    }
  ],
  "meta": {
    "request_id": "7f5d0fde-9c9a-4b6a-8c1a-6b5f3c5e1d2a"
  }
}
```

### Error Handling

The API Gateway provides standardized error responses using a custom error type:

```rust
#[derive(Debug, thiserror::Error)]
pub enum ApiError {
    #[error("Not found: {0}")]
    NotFound(String),
    
    #[error("Invalid request: {0}")]
    BadRequest(String),
    
    #[error("Unauthorized: {0}")]
    Unauthorized(String),
    
    #[error("Forbidden: {0}")]
    Forbidden(String),
    
    #[error("Internal server error: {0}")]
    Internal(String),
    
    #[error("Common error: {0}")]
    Common(#[from] common::error::Error),
}
```

Error responses are formatted as JSON:

```json
{
  "error": {
    "code": "not_found",
    "message": "Account not found: 123e4567-e89b-12d3-a456-426614174000",
    "details": null
  },
  "request_id": "9f83c01a-1234-5678-9abc-def012345678"
}
```

## Request/Response Examples

### Creating an Account

**Request:**
```http
POST /api/v1/accounts
Content-Type: application/json

{}
```

**Response:**
```http
HTTP/1.1 200 OK
Content-Type: application/json

{
  "data": {
    "id": "123e4567-e89b-12d3-a456-426614174000",
    "created_at": "2025-02-27T12:34:56Z",
    "updated_at": "2025-02-27T12:34:56Z"
  },
  "meta": {
    "request_id": "7f5d0fde-9c9a-4b6a-8c1a-6b5f3c5e1d2a"
  }
}
```

### Placing an Order

**Request:**
```http
POST /api/v1/orders
Content-Type: application/json

{
  "user_id": "123e4567-e89b-12d3-a456-426614174000",
  "market": "BTC/USD",
  "side": "buy",
  "order_type": "limit",
  "price": "20000",
  "quantity": "0.1"
}
```

**Response:**
```http
HTTP/1.1 200 OK
Content-Type: application/json

{
  "data": {
    "order": {
      "id": "abcdef12-3456-7890-abcd-ef1234567890",
      "user_id": "123e4567-e89b-12d3-a456-426614174000",
      "market": "BTC/USD",
      "side": "buy",
      "order_type": "limit",
      "price": "20000",
      "quantity": "0.1",
      "remaining_quantity": "0.1",
      "filled_quantity": "0",
      "status": "new",
      "time_in_force": "GTC",
      "created_at": "2025-02-27T12:34:56Z",
      "updated_at": "2025-02-27T12:34:56Z"
    },
    "trades": []
  },
  "meta": {
    "request_id": "7f5d0fde-9c9a-4b6a-8c1a-6b5f3c5e1d2a"
  }
}
```

### WebSocket Subscription

**Client Message:**
```json
{
  "op": "subscribe",
  "channel": "orderbook",
  "market": "BTC/USD"
}
```

**Server Message:**
```json
{
  "type": "orderbook",
  "market": "BTC/USD",
  "data": {
    "bids": [
      ["20000", "1.5"],
      ["19500", "2.3"]
    ],
    "asks": [
      ["20100", "1.2"],
      ["20200", "3.4"]
    ],
    "timestamp": "2025-02-27T12:34:56Z"
  }
}
```

## Configuration

The API Gateway can be configured using environment variables:

- `API_PORT`: HTTP port to listen on (default: 8081)
- `API_HOST`: Host address to bind to (default: 0.0.0.0)
- `RUST_LOG`: Logging level (default: info)
- `DATABASE_URL`: Connection string for the database
- `CORS_ORIGINS`: Allowed CORS origins (comma separated)

## Performance Considerations

The API Gateway is designed for high performance:

- **Asynchronous Processing**: Non-blocking I/O using Tokio
- **Connection Pooling**: Efficient reuse of service connections
- **Request Batching**: Support for processing multiple operations
- **Caching**: (Planned) Response caching for frequently accessed data
- **Load Balancing**: (Planned) Distribution of requests across instances

## Security Considerations

The API Gateway implements several security measures:

- **Input Validation**: Strict validation of all input parameters
- **CORS Protection**: Configurable cross-origin resource sharing
- **Error Handling**: Limited error information to prevent information leakage
- **Rate Limiting**: (Planned) Protection against brute force and DoS attacks
- **Authentication**: (Planned) Token-based authentication for secure access

## Extending the API

To add new endpoints to the API Gateway:

1. Define the request and response models in the appropriate module
2. Implement the handler function
3. Add the route to the router
4. Update documentation

Example of adding a new endpoint:

```rust
// 1. Define models
#[derive(Debug, Deserialize)]
pub struct NewEndpointRequest {
    pub parameter: String,
}

#[derive(Debug, Serialize)]
pub struct NewEndpointResponse {
    pub result: String,
}

// 2. Implement handler
pub async fn new_endpoint(
    State(state): State<Arc<AppState>>,
    Json(request): Json<NewEndpointRequest>,
) -> Result<Json<NewEndpointResponse>, ApiError> {
    // Implementation
    Ok(Json(NewEndpointResponse { 
        result: format!("Processed: {}", request.parameter) 
    }))
}

// 3. Add route
let api_routes = router.route("/new-endpoint", axum::routing::post(new_endpoint));
```

## Monitoring and Observability

The API Gateway includes monitoring features:

- **Request Logging**: Detailed logs of all requests and responses
- **Metrics**: (Planned) Performance metrics for endpoints
- **Tracing**: Request tracing through the system
- **Health Checks**: Endpoint for monitoring system health

## Future Enhancements

Planned improvements to the API Gateway include:

- **API Versioning**: Support for multiple API versions
- **GraphQL Interface**: Alternative to REST for more flexible queries
- **Request Throttling**: Graduated rate limiting based on user tier
- **Documentation**: OpenAPI/Swagger integration
- **Authentication**: OAuth2 and JWT support
- **Circuit Breaking**: Automatic handling of downstream service failures