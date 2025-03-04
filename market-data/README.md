# Market Data Service

The Market Data Service is responsible for collecting, processing, and distributing real-time market data within the Zavora Trading Engine. It serves as the central hub for all market-related information, providing a consistent and up-to-date view of market activity.

## Features

- **Real-time Order Book Updates**: Maintain current state of order books
- **Trade Data Processing**: Record and broadcast executed trades
- **Market Statistics**: Calculate and provide market summaries and price data
- **Price Candles**: Generate time-series price data at various intervals
- **WebSocket Broadcasting**: Distribute market data to clients in real time
- **Historical Data**: Store and retrieve historical market data
- **Concurrent Access**: Thread-safe data structures for high throughput

## Core Components

### MarketDataService

The `MarketDataService` is the main component, providing methods to:

- Process trades from the matching engine
- Update order book snapshots
- Generate market statistics
- Broadcast data to subscribed clients
- Query historical data

```rust
// Create a new market data service
let market_data_service = MarketDataService::new();

// Process a new trade
market_data_service.process_trade(&trade).await?;

// Update an order book
market_data_service.update_order_book(
    "BTC/USD", 
    bids, // Vector of (price, quantity) pairs
    asks  // Vector of (price, quantity) pairs
).await?;
```

### Data Models

The service defines several data models for different market data types:

#### MarketDepth

Represents the current state of an order book:

```rust
pub struct MarketDepth {
    pub market: String,
    pub bids: Vec<PriceLevel>,  // Ordered by price (highest first)
    pub asks: Vec<PriceLevel>,  // Ordered by price (lowest first)
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

pub struct PriceLevel {
    pub price: Price,
    pub quantity: Quantity,
}
```

#### TradeMessage

Represents an executed trade, formatted for distribution:

```rust
pub struct TradeMessage {
    pub id: String,
    pub market: String,
    pub price: Price,
    pub quantity: Quantity,
    pub side: String,  // "buy" or "sell"
    pub timestamp: chrono::DateTime<chrono::Utc>,
}
```

#### Ticker

Represents the current market ticker:

```rust
pub struct Ticker {
    pub market: String,
    pub bid: Option<Price>,        // Best bid price
    pub ask: Option<Price>,        // Best ask price
    pub last: Option<Price>,       // Last trade price
    pub volume: Quantity,          // 24h volume
    pub change: Option<Price>,     // 24h price change
    pub change_percent: Option<f64>, // 24h price change percent
    pub timestamp: chrono::DateTime<chrono::Utc>,
}
```

#### Candle

Represents price candles (OHLCV) for time-series analysis:

```rust
pub struct Candle {
    pub market: String,
    pub interval: CandleInterval,  // 1m, 5m, 15m, 1h, 4h, 1d, etc.
    pub open: Price,
    pub high: Price,
    pub low: Price,
    pub close: Price,
    pub volume: Quantity,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}
```

### MarketDataChannel

The `MarketDataChannel` handles real-time data distribution:

- Manages subscriptions to different market data topics
- Broadcasts updates to subscribed clients
- Handles WebSocket connections and message formatting

```rust
// Topics that clients can subscribe to
pub enum Topic {
    OrderBook(String),  // Subscribe to order book for a specific market
    Trades(String),     // Subscribe to trades for a specific market
    Ticker(String),     // Subscribe to ticker for a specific market
    AllTickers,         // Subscribe to all tickers
}
```

## Usage Examples

### Processing a Trade

When a trade occurs in the matching engine, it should be passed to the market data service:

```rust
// Process a trade to update market data
market_data_service.process_trade(&trade).await?;
```

This will:
1. Update the recent trades list
2. Update the market ticker
3. Update price candles
4. Broadcast the trade to subscribed clients

### Updating an Order Book

After changes to the order book, update the market data service:

```rust
// Get the current order book from the matching engine
let (bids, asks) = matching_engine.get_market_depth("BTC/USD", 10)?;

// Update the market data service
market_data_service.update_order_book("BTC/USD", bids, asks).await?;
```

This will:
1. Update the stored market depth
2. Update the ticker bid/ask
3. Broadcast the order book update to subscribed clients

### Getting Market Data

Clients can retrieve the latest market data:

```rust
// Get the current market depth
let depth = market_data_service.get_market_depth("BTC/USD").await?;

// Get the current ticker
let ticker = market_data_service.get_ticker("BTC/USD").await?;

// Get recent trades
let trades = market_data_service.get_recent_trades("BTC/USD", 100).await?;

// Get candles for a specific interval
let candles = market_data_service.get_candles("BTC/USD", CandleInterval::FifteenMinutes, 24).await?;
```

### Subscribing to Updates

Clients can subscribe to real-time updates:

```rust
// Subscribe to order book updates
let subscription_id = market_data_service.subscribe(Topic::OrderBook("BTC/USD".to_string()), callback).await?;

// Callback function to handle updates
async fn callback(message: String) {
    println!("Received update: {}", message);
}

// Unsubscribe when no longer needed
market_data_service.unsubscribe(subscription_id).await?;
```

## Database Integration

The Market Data Service can be configured to persist market data to a database:

```rust
// Create a service with database persistence
let market_data_service = MarketDataService::with_repository(
    Some("postgres://user:pass@localhost/db".to_string())
).await?;
```

This enables:
- Historical data retrieval
- Recovery after service restart
- Data analytics and reporting

## Performance Considerations

The Market Data Service is optimized for performance:

- **Concurrent Data Structures**: Uses `DashMap` for thread-safe access
- **Efficient Broadcasting**: Only sends updates to interested subscribers
- **Incremental Updates**: Supports delta updates to minimize bandwidth
- **Throttling**: Configurable update frequency to avoid overwhelming consumers
- **Optimized Storage**: Compact data representation for memory efficiency

## WebSocket API Integration

The Market Data Service seamlessly integrates with the WebSocket API:

```rust
// In the WebSocket handler
let request = serde_json::from_str::<SubscriptionRequest>(&message)?;

// Subscribe to the requested topic
match request.topic.as_str() {
    "orderbook" => {
        let topic = Topic::OrderBook(request.market.clone());
        let callback = |message: String| send_to_websocket(message, ws_sender.clone());
        market_data_service.subscribe(topic, callback).await?;
    },
    // ... other topics
}
```

## Testing

The Market Data Service includes comprehensive tests:

```bash
# Run all tests
cargo test -p market-data

# Run a specific test
cargo test -p market-data test_process_trade
```

Key test cases include:
- Trade processing
- Order book updates
- Candle generation
- Subscription management
- Historical data retrieval

## Future Enhancements

Planned improvements to the Market Data Service include:

- **Advanced Statistics**: VWAP, market impact measures, and volatility metrics
- **Market Events**: Trading halts, circuit breakers, and auction notifications
- **Data Compression**: Efficient encoding for high-volume data
- **Data Snapshots**: Periodic full snapshots for faster client synchronization
- **Multi-tier Storage**: Hot/warm/cold data storage for efficient retrieval
- **Analytics API**: Advanced query capabilities for market analysis