# Matching Engine

The Matching Engine is the core component of the Zavora Trading Engine, responsible for maintaining order books, matching orders, and generating trades. It provides a high-performance, low-latency implementation of a financial matching engine using a price-time priority algorithm.

## Features

- **Order Book Management**: Efficient maintenance of limit order books for multiple markets
- **Price-Time Priority**: Standard FIFO (First In, First Out) matching at each price level
- **Order Types Support**: Market orders and limit orders
- **Time-in-Force Support**: GTC (Good Till Canceled) and IOC (Immediate or Cancel)
- **Thread-Safe Operations**: Concurrent access for high-throughput environments
- **Order Cancellation**: Fast removal of orders from the book
- **Market Depth**: Easy access to bid/ask levels for market data
- **Trade Generation**: Automatic creation of trades when orders match

## Core Components

### MatchingEngine

The `MatchingEngine` is the main entry point to the matching service, providing methods to:

- Register new markets
- Place orders (market and limit)
- Cancel existing orders
- Retrieve market depth
- Get order information

```rust
let mut engine = MatchingEngine::new();

// Register a market
engine.register_market("BTC/USD".to_string());

// Place a limit buy order
let order = Order::new_limit(
    user_id,
    "BTC/USD".to_string(),
    Side::Buy,
    dec!(20000),  // Price
    dec!(1.0),    // Quantity
    TimeInForce::GTC,
);

// Execute the order against the book
let result = engine.place_order(order)?;

// The result contains:
// - The updated taker order (if not fully filled)
// - Maker orders that were matched
// - Trades that were generated
for trade in &result.trades {
    println!("Trade executed: {} {} at {}", 
             trade.quantity, trade.market, trade.price);
}
```

### OrderBook

Each market has an `OrderBook` that maintains the current state of all open orders:

- Bids (buy orders) sorted by price (highest first)
- Asks (sell orders) sorted by price (lowest first)
- For each price level, orders are sorted by time (oldest first)

```rust
// The internal OrderBook structure maintains:
// - Bids map (price -> orders)
// - Asks map (price -> orders)
// - Last traded price
```

### MatchingResult

When an order is placed, a `MatchingResult` is returned containing:

```rust
pub struct MatchingResult {
    // The updated taker order (if not fully filled)
    pub taker_order: Option<Arc<Order>>,
    // Maker orders that were matched
    pub maker_orders: Vec<Arc<Order>>,
    // Trades that were generated
    pub trades: Vec<Trade>,
}
```

## Order Matching Algorithm

The matching engine implements a standard price-time priority algorithm:

### For Market Orders:
1. Take all liquidity available on the opposite side until filled or no more liquidity
2. Market buy orders start with the lowest ask price
3. Market sell orders start with the highest bid price
4. Any unfilled portion of a market order is canceled

### For Limit Orders:
1. Check if the order can match immediately against the opposite side
2. For buy orders: if price >= lowest ask price
3. For sell orders: if price <= highest bid price
4. Match against available orders at the best price, then next best, and so on
5. Any unfilled portion is added to the order book (if GTC)

## Thread Safety

The Matching Engine is designed for concurrent access:

- Uses `DashMap` for concurrent access to order books
- Employs fine-grained locking for each order book
- Provides thread-safe access to market data
- Supports concurrent order placement and cancellation

## Usage Examples

### Registering a Market

```rust
let engine = MatchingEngine::new();
engine.register_market("BTC/USD".to_string());
```

### Placing a Limit Order

```rust
// Create a limit buy order
let buy_order = Order::new_limit(
    user_id,
    "BTC/USD".to_string(),
    Side::Buy,
    dec!(20000),  // Price in USD
    dec!(1.0),    // Quantity in BTC
    TimeInForce::GTC,
);

// Place the order
let result = engine.place_order(buy_order)?;

// Handle the result
if let Some(taker) = &result.taker_order {
    println!("Order status: {:?}", taker.status);
    println!("Filled quantity: {}", taker.filled_quantity);
    println!("Remaining quantity: {}", taker.remaining_quantity);
}

// Process any trades
for trade in &result.trades {
    println!("Trade executed: {} {} at {}", 
             trade.quantity, trade.market, trade.price);
}
```

### Placing a Market Order

```rust
// Create a market sell order
let sell_order = Order::new_market(
    user_id,
    "BTC/USD".to_string(),
    Side::Sell,
    dec!(0.5),    // Quantity in BTC
);

// Place the order
let result = engine.place_order(sell_order)?;

// Market orders either fill completely or not at all
if result.trades.is_empty() {
    println!("No liquidity available to fill market order");
} else {
    println!("{} trade(s) executed", result.trades.len());
}
```

### Cancelling an Order

```rust
// Cancel an existing order
let canceled_order = engine.cancel_order(order_id)?;
println!("Order canceled: {}", canceled_order.id);
```

### Getting Market Depth

```rust
// Get top 10 levels of the order book
let (bids, asks) = engine.get_market_depth("BTC/USD", 10)?;

println!("Bids:");
for (price, quantity) in &bids {
    println!("  {} @ {}", quantity, price);
}

println!("Asks:");
for (price, quantity) in &asks {
    println!("  {} @ {}", quantity, price);
}
```

## Testing

The matching engine has comprehensive test coverage:

```bash
# Run all tests
cargo test -p matching-engine

# Run a specific test
cargo test -p matching-engine test_matching_limit_orders
```

Key test cases include:
- Basic limit order matching
- Market order execution
- Partial fills
- Price-time priority validation
- Order cancellation
- Empty order book handling
- Market depth retrieval

## Performance Considerations

The matching engine is optimized for performance:

- **Memory Efficiency**: Orders are stored efficiently using Rust's ownership model
- **Algorithm Complexity**: O(1) access to the best price levels
- **Lock Granularity**: Fine-grained locking to minimize contention
- **Cache Friendliness**: Data structures designed to be cache-friendly
- **Minimal Copying**: Use of reference counting (`Arc`) to avoid unnecessary copying

## Integration with Other Services

The matching engine is designed to be integrated with other services in the trading system:

1. **Account Service**: Before an order is placed, the account service should reserve the necessary funds
2. **Market Data Service**: After orders are matched, update the market data service with new trades
3. **API Gateway**: Expose order placement and cancellation through the API

Example integration flow:
```rust
// 1. Account service reserves funds
account_service.reserve_for_order(&order).await?;

// 2. Matching engine processes the order
let result = matching_engine.place_order(order)?;

// 3. Account service processes any trades
for trade in &result.trades {
    account_service.process_trade(trade).await?;
    
    // 4. Market data service updates with new trade
    market_data_service.process_trade(trade).await?;
}

// 5. If order was added to the book, update order book in market data
if result.taker_order.is_some() {
    if let Ok((bids, asks)) = matching_engine.get_market_depth(&order.market, 10) {
        market_data_service.update_order_book(&order.market, bids, asks).await?;
    }
}
```

## Future Enhancements

Planned improvements to the matching engine include:

- **Additional Order Types**: Support for stop, stop-limit, and iceberg orders
- **Additional Time-in-Force Options**: FOK (Fill or Kill) and GTD (Good Till Date)
- **Auction Support**: Opening and closing auction mechanisms
- **Circuit Breakers**: Automatic trading halts on excessive price movements
- **Price Banding**: Rejection of orders outside acceptable price ranges
- **Historical Data**: Recording of order book state for replay and analysis
- **Persistence**: Optional persistence of order book state for recovery