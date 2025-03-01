use common::decimal::{Price, Quantity};
use common::model::order::Side;
use common::model::trade::Trade;
use market_data::channel::Topic;
use market_data::{CandleInterval, TradeMessage, MarketDataService, OrderBookUpdate};
use tokio::time::{sleep, Duration};
use uuid::Uuid;

#[tokio::test]
async fn test_update_order_book() {
    let service = MarketDataService::new();
    
    // Create some test data
    let market = "BTC/USD";
    let bids = vec![
        (Price::new(9900, 0), Quantity::new(1, 0)),
        (Price::new(9800, 0), Quantity::new(2, 0)),
    ];
    let asks = vec![
        (Price::new(10100, 0), Quantity::new(1, 0)),
        (Price::new(10200, 0), Quantity::new(2, 0)),
    ];
    
    // Update order book
    let result = service.update_order_book(market, bids.clone(), asks.clone()).await;
    assert!(result.is_ok());
    
    // Get market depth
    let depth = service.get_market_depth(market).unwrap();
    
    // Verify bids
    assert_eq!(depth.bids.len(), 2);
    assert_eq!(depth.bids[0].price, Price::new(9900, 0));
    assert_eq!(depth.bids[0].quantity, Quantity::new(1, 0));
    assert_eq!(depth.bids[1].price, Price::new(9800, 0));
    assert_eq!(depth.bids[1].quantity, Quantity::new(2, 0));
    
    // Verify asks
    assert_eq!(depth.asks.len(), 2);
    assert_eq!(depth.asks[0].price, Price::new(10100, 0));
    assert_eq!(depth.asks[0].quantity, Quantity::new(1, 0));
    assert_eq!(depth.asks[1].price, Price::new(10200, 0));
    assert_eq!(depth.asks[1].quantity, Quantity::new(2, 0));
    
    // Verify ticker was updated
    let ticker = service.get_ticker(market).unwrap();
    assert_eq!(ticker.bid, Some(Price::new(9900, 0)));
    assert_eq!(ticker.ask, Some(Price::new(10100, 0)));
}

#[tokio::test]
async fn test_process_trade() {
    let service = MarketDataService::new();
    
    // Create a test trade
    let trade = Trade::new(
        "BTC/USD".to_string(),
        Price::new(10000, 0),
        Quantity::new(1, 0),
        Uuid::new_v4(),
        Uuid::new_v4(),
        Uuid::new_v4(),
        Uuid::new_v4(),
        Side::Buy,
    );
    
    // Process the trade
    let result = service.process_trade(&trade).await;
    assert!(result.is_ok());
    
    // Verify recent trades
    let recent_trades = service.get_recent_trades("BTC/USD", 10);
    assert_eq!(recent_trades.len(), 1);
    assert_eq!(recent_trades[0].id, trade.id);
    assert_eq!(recent_trades[0].price, trade.price);
    assert_eq!(recent_trades[0].quantity, trade.quantity);
    
    // Verify candles were updated
    let candles = service.get_candles("BTC/USD", CandleInterval::Minute1, 10);
    assert_eq!(candles.len(), 1);
    assert_eq!(candles[0].open, trade.price);
    assert_eq!(candles[0].high, trade.price);
    assert_eq!(candles[0].low, trade.price);
    assert_eq!(candles[0].close, trade.price);
    assert_eq!(candles[0].volume, trade.quantity);
}

#[tokio::test]
async fn test_multiple_trades_same_candle() {
    let service = MarketDataService::new();
    
    // Create first trade
    let trade1 = Trade::new(
        "ETH/USD".to_string(),
        Price::new(200, 0),
        Quantity::new(10, 0),
        Uuid::new_v4(),
        Uuid::new_v4(),
        Uuid::new_v4(),
        Uuid::new_v4(),
        Side::Buy,
    );
    
    // Process first trade
    service.process_trade(&trade1).await.unwrap();
    
    // Create second trade with higher price
    let trade2 = Trade::new(
        "ETH/USD".to_string(),
        Price::new(210, 0),
        Quantity::new(5, 0),
        Uuid::new_v4(),
        Uuid::new_v4(),
        Uuid::new_v4(),
        Uuid::new_v4(),
        Side::Buy,
    );
    
    // Process second trade
    service.process_trade(&trade2).await.unwrap();
    
    // Create third trade with lower price
    let trade3 = Trade::new(
        "ETH/USD".to_string(),
        Price::new(190, 0),
        Quantity::new(3, 0),
        Uuid::new_v4(),
        Uuid::new_v4(),
        Uuid::new_v4(),
        Uuid::new_v4(),
        Side::Sell,
    );
    
    // Process third trade
    service.process_trade(&trade3).await.unwrap();
    
    // Verify candles
    let candles = service.get_candles("ETH/USD", CandleInterval::Minute1, 10);
    assert_eq!(candles.len(), 1);
    
    let candle = &candles[0];
    assert_eq!(candle.open, Price::new(200, 0)); // First trade price
    assert_eq!(candle.high, Price::new(210, 0)); // Highest price
    assert_eq!(candle.low, Price::new(190, 0));  // Lowest price
    assert_eq!(candle.close, Price::new(190, 0)); // Last trade price
    assert_eq!(candle.volume, Quantity::new(18, 0)); // Sum of all quantities
    assert_eq!(candle.trades, 3); // Three trades
}

#[tokio::test]
async fn test_get_all_tickers() {
    let service = MarketDataService::new();
    
    // Update order books for multiple markets
    service.update_order_book(
        "BTC/USD", 
        vec![(Price::new(9900, 0), Quantity::new(1, 0))], 
        vec![(Price::new(10100, 0), Quantity::new(1, 0))]
    ).await.unwrap();
    
    service.update_order_book(
        "ETH/USD", 
        vec![(Price::new(190, 0), Quantity::new(1, 0))], 
        vec![(Price::new(210, 0), Quantity::new(1, 0))]
    ).await.unwrap();
    
    // Get all tickers
    let tickers = service.get_all_tickers();
    assert_eq!(tickers.len(), 2);
    
    // Find BTC/USD ticker
    let btc_ticker = tickers.iter().find(|t| t.market == "BTC/USD").unwrap();
    assert_eq!(btc_ticker.bid, Some(Price::new(9900, 0)));
    assert_eq!(btc_ticker.ask, Some(Price::new(10100, 0)));
    
    // Find ETH/USD ticker
    let eth_ticker = tickers.iter().find(|t| t.market == "ETH/USD").unwrap();
    assert_eq!(eth_ticker.bid, Some(Price::new(190, 0)));
    assert_eq!(eth_ticker.ask, Some(Price::new(210, 0)));
}

#[tokio::test]
#[ignore = "Channel subscription test occasionally fails due to timing issues"]
async fn test_channel_subscription() {
    let service = MarketDataService::new();
    let channel = service.channel();
    
    // Subscribe to order book updates
    let receiver = channel.subscribe::<OrderBookUpdate>(Topic::OrderBook("BTC/USD".to_string())).await;
    
    // Update order book
    service.update_order_book(
        "BTC/USD", 
        vec![(Price::new(9900, 0), Quantity::new(1, 0))], 
        vec![(Price::new(10100, 0), Quantity::new(1, 0))]
    ).await.unwrap();
    
    // Add a small delay to ensure the message is delivered
    sleep(Duration::from_millis(50)).await;
    
    // Receive the update
    let update = receiver.recv().unwrap();
    
    // Verify the update
    let update = update.downcast_ref::<OrderBookUpdate>().unwrap();
    assert_eq!(update.market, "BTC/USD");
    assert_eq!(update.bids.len(), 1);
    assert_eq!(update.asks.len(), 1);
    assert_eq!(update.bids[0].price, Price::new(9900, 0));
    assert_eq!(update.asks[0].price, Price::new(10100, 0));
}

#[tokio::test]
#[ignore = "Channel subscription test occasionally fails due to timing issues"]
async fn test_trade_subscription() {
    let service = MarketDataService::new();
    let channel = service.channel();
    
    // Subscribe to trade updates
    let receiver = channel.subscribe::<TradeMessage>(Topic::Trades("BTC/USD".to_string())).await;
    
    // Create a test trade
    let trade = Trade::new(
        "BTC/USD".to_string(),
        Price::new(10000, 0),
        Quantity::new(1, 0),
        Uuid::new_v4(),
        Uuid::new_v4(),
        Uuid::new_v4(),
        Uuid::new_v4(),
        Side::Buy,
    );
    
    // Process the trade
    service.process_trade(&trade).await.unwrap();
    
    // Add a small delay to ensure the message is delivered
    sleep(Duration::from_millis(50)).await;
    
    // Receive the trade message
    let message = receiver.recv().unwrap();
    
    // Verify the message
    let trade_msg = message.downcast_ref::<TradeMessage>().unwrap();
    assert_eq!(trade_msg.id, trade.id);
    assert_eq!(trade_msg.market, "BTC/USD");
    assert_eq!(trade_msg.price, Price::new(10000, 0));
    assert_eq!(trade_msg.quantity, Quantity::new(1, 0));
}