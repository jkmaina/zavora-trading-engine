use std::sync::Arc;
use uuid::Uuid;
use common::decimal::{Price, Quantity};
use common::model::order::{Order, Status, OrderType, Side, TimeInForce};
use matching_engine::engine::MatchingEngine;

fn create_test_order(
    user_id: Uuid,
    market: &str,
    side: Side,
    order_type: OrderType,
    price: Option<Price>,
    quantity: Quantity
) -> Order {
    Order {
        id: Uuid::new_v4(),
        user_id,
        market: market.to_string(),
        side,
        order_type,
        price,
        quantity,
        remaining_quantity: quantity,
        filled_quantity: Quantity::ZERO,
        status: Status::New,
        time_in_force: TimeInForce::GTC,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
        average_fill_price: None,
    }
}

#[test]
fn test_register_market() {
    let mut engine = MatchingEngine::new();
    engine.register_market("BTC/USD".to_string());
    
    // Try to place an order to verify the market exists
    let user_id = Uuid::new_v4();
    let order = create_test_order(
        user_id,
        "BTC/USD",
        Side::Buy,
        OrderType::Limit,
        Some(Quantity::new(10000, 0)),
        Quantity::new(1, 0)
    );
    
    let result = engine.place_order(order);
    assert!(result.is_ok());
}

#[test]
fn test_place_limit_order() {
    let mut engine = MatchingEngine::new();
    engine.register_market("BTC/USD".to_string());
    
    let user_id = Uuid::new_v4();
    let order = create_test_order(
        user_id,
        "BTC/USD",
        Side::Buy,
        OrderType::Limit,
        Some(Quantity::new(10000, 0)),
        Quantity::new(1, 0)
    );
    
    let result = engine.place_order(order.clone());
    assert!(result.is_ok());
    
    let matching_result = result.unwrap();
    assert!(matching_result.taker_order.is_some());
    assert_eq!(matching_result.maker_orders.len(), 0);
    assert_eq!(matching_result.trades.len(), 0);
    
    // Verify the order is in the book
    let stored_order = engine.get_order(order.id);
    assert!(stored_order.is_some());
    assert_eq!(stored_order.unwrap().id, order.id);
}

#[test]
fn test_matching_limit_orders() {
    let mut engine = MatchingEngine::new();
    engine.register_market("BTC/USD".to_string());
    
    // Create a sell order first
    let seller_id = Uuid::new_v4();
    let sell_order = create_test_order(
        seller_id,
        "BTC/USD",
        Side::Sell,
        OrderType::Limit,
        Some(Quantity::new(10000, 0)),
        Quantity::new(1, 0)
    );
    
    let result = engine.place_order(sell_order.clone());
    assert!(result.is_ok());
    
    // Now create a matching buy order
    let buyer_id = Uuid::new_v4();
    let buy_order = create_test_order(
        buyer_id,
        "BTC/USD",
        Side::Buy,
        OrderType::Limit,
        Some(Quantity::new(10000, 0)),
        Quantity::new(1, 0)
    );
    
    let result = engine.place_order(buy_order.clone());
    assert!(result.is_ok());
    
    let matching_result = result.unwrap();
    assert!(matching_result.taker_order.is_some());
    assert_eq!(matching_result.maker_orders.len(), 1);
    assert_eq!(matching_result.trades.len(), 1);
    
    // Verify the trade
    let trade = &matching_result.trades[0];
    assert_eq!(trade.market, "BTC/USD");
    assert_eq!(trade.price, Quantity::new(10000, 0));
    assert_eq!(trade.quantity, Quantity::new(1, 0));
    assert_eq!(trade.buyer_id, buyer_id);
    assert_eq!(trade.seller_id, seller_id);
}

#[test]
fn test_partial_fill() {
    let mut engine = MatchingEngine::new();
    engine.register_market("BTC/USD".to_string());
    
    // Create a sell order first
    let seller_id = Uuid::new_v4();
    let sell_order = create_test_order(
        seller_id,
        "BTC/USD",
        Side::Sell,
        OrderType::Limit,
        Some(Quantity::new(10000, 0)),
        Quantity::new(2, 0)
    );
    
    let result = engine.place_order(sell_order.clone());
    assert!(result.is_ok());
    
    // Now create a smaller buy order
    let buyer_id = Uuid::new_v4();
    let buy_order = create_test_order(
        buyer_id,
        "BTC/USD",
        Side::Buy,
        OrderType::Limit,
        Some(Quantity::new(10000, 0)),
        Quantity::new(1, 0)
    );
    
    let result = engine.place_order(buy_order.clone());
    assert!(result.is_ok());
    
    let matching_result = result.unwrap();
    assert!(matching_result.taker_order.is_some());
    assert_eq!(matching_result.maker_orders.len(), 1);
    assert_eq!(matching_result.trades.len(), 1);
    
    // Verify the trade
    let trade = &matching_result.trades[0];
    assert_eq!(trade.quantity, Quantity::new(1, 0));
    
    // Verify the sell order is still in the book with reduced quantity
    let stored_sell_order = engine.get_order(sell_order.id);
    assert!(stored_sell_order.is_some());
    let stored_sell = stored_sell_order.unwrap();
    assert_eq!(stored_sell.remaining_quantity, Quantity::new(1, 0));
    assert_eq!(stored_sell.filled_quantity, Quantity::new(1, 0));
    assert_eq!(stored_sell.status, Status::PartiallyFilled);
}

#[test]
fn test_market_order() {
    let mut engine = MatchingEngine::new();
    engine.register_market("BTC/USD".to_string());
    
    // Create a sell limit order first
    let seller_id = Uuid::new_v4();
    let sell_order = create_test_order(
        seller_id,
        "BTC/USD",
        Side::Sell,
        OrderType::Limit,
        Some(Quantity::new(10000, 0)),
        Quantity::new(1, 0)
    );
    
    let result = engine.place_order(sell_order.clone());
    assert!(result.is_ok());
    
    // Now create a market buy order
    let buyer_id = Uuid::new_v4();
    let buy_order = create_test_order(
        buyer_id,
        "BTC/USD",
        Side::Buy,
        OrderType::Market,
        None,
        Quantity::new(1, 0)
    );
    
    let result = engine.place_order(buy_order.clone());
    assert!(result.is_ok());
    
    let matching_result = result.unwrap();
    assert!(matching_result.taker_order.is_some());
    assert_eq!(matching_result.maker_orders.len(), 1);
    assert_eq!(matching_result.trades.len(), 1);
    
    // Verify the trade
    let trade = &matching_result.trades[0];
    assert_eq!(trade.price, Quantity::new(10000, 0)); // Should execute at limit price
    assert_eq!(trade.quantity, Quantity::new(1, 0));
}

#[test]
fn test_cancel_order() {
    let mut engine = MatchingEngine::new();
    engine.register_market("BTC/USD".to_string());
    
    // Place a limit order
    let user_id = Uuid::new_v4();
    let order = create_test_order(
        user_id,
        "BTC/USD",
        Side::Buy,
        OrderType::Limit,
        Some(Quantity::new(10000, 0)),
        Quantity::new(1, 0)
    );
    
    let result = engine.place_order(order.clone());
    assert!(result.is_ok());
    
    // Cancel the order
    let cancel_result = engine.cancel_order(order.id);
    assert!(cancel_result.is_ok());
    
    let cancelled_order = cancel_result.unwrap();
    assert_eq!(cancelled_order.id, order.id);
    assert_eq!(cancelled_order.status, Status::Cancelled);
    
    // Try to cancel again (should fail)
    let cancel_again = engine.cancel_order(order.id);
    assert!(cancel_again.is_err());
}

#[test]
fn test_get_market_depth() {
    let mut engine = MatchingEngine::new();
    engine.register_market("BTC/USD".to_string());
    
    // Place some buy orders
    let user_id = Uuid::new_v4();
    
    let buy_order1 = create_test_order(
        user_id,
        "BTC/USD",
        Side::Buy,
        OrderType::Limit,
        Some(Quantity::new(9900, 0)),
        Quantity::new(1, 0)
    );
    
    let buy_order2 = create_test_order(
        user_id,
        "BTC/USD",
        Side::Buy,
        OrderType::Limit,
        Some(Quantity::new(10000, 0)),
        Quantity::new(2, 0)
    );
    
    // Place some sell orders
    let sell_order1 = create_test_order(
        user_id,
        "BTC/USD",
        Side::Sell,
        OrderType::Limit,
        Some(Quantity::new(10100, 0)),
        Quantity::new(1, 0)
    );
    
    let sell_order2 = create_test_order(
        user_id,
        "BTC/USD",
        Side::Sell,
        OrderType::Limit,
        Some(Quantity::new(10200, 0)),
        Quantity::new(2, 0)
    );
    
    engine.place_order(buy_order1).unwrap();
    engine.place_order(buy_order2).unwrap();
    engine.place_order(sell_order1).unwrap();
    engine.place_order(sell_order2).unwrap();
    
    // Get market depth
    let depth_result = engine.get_market_depth("BTC/USD", 10);
    assert!(depth_result.is_ok());
    
    let (bids, asks) = depth_result.unwrap();
    
    // Verify bids (highest price first)
    assert_eq!(bids.len(), 2);
    assert_eq!(bids[0].0, Quantity::new(10000, 0));
    assert_eq!(bids[0].1, Quantity::new(2, 0));
    assert_eq!(bids[1].0, Quantity::new(9900, 0));
    assert_eq!(bids[1].1, Quantity::new(1, 0));
    
    // Verify asks (lowest price first)
    assert_eq!(asks.len(), 2);
    assert_eq!(asks[0].0, Quantity::new(10100, 0));
    assert_eq!(asks[0].1, Quantity::new(1, 0));
    assert_eq!(asks[1].0, Quantity::new(10200, 0));
    assert_eq!(asks[1].1, Quantity::new(2, 0));
}

#[test]
fn test_price_time_priority() {
    let mut engine = MatchingEngine::new();
    engine.register_market("BTC/USD".to_string());
    
    let user_id = Uuid::new_v4();
    
    // Place two sell orders at the same price
    let sell_order1 = create_test_order(
        user_id,
        "BTC/USD",
        Side::Sell,
        OrderType::Limit,
        Some(Quantity::new(10000, 0)),
        Quantity::new(1, 0)
    );
    
    // Small delay to ensure different timestamps
    std::thread::sleep(std::time::Duration::from_millis(10));
    
    let sell_order2 = create_test_order(
        user_id,
        "BTC/USD",
        Side::Sell,
        OrderType::Limit,
        Some(Quantity::new(10000, 0)),
        Quantity::new(1, 0)
    );
    
    engine.place_order(sell_order1.clone()).unwrap();
    engine.place_order(sell_order2.clone()).unwrap();
    
    // Now place a buy order that matches only one sell order
    let buy_order = create_test_order(
        user_id,
        "BTC/USD",
        Side::Buy,
        OrderType::Limit,
        Some(Quantity::new(10000, 0)),
        Quantity::new(1, 0)
    );
    
    let result = engine.place_order(buy_order).unwrap();
    
    // Verify that the first sell order was matched (time priority)
    assert_eq!(result.maker_orders.len(), 1);
    assert_eq!(result.maker_orders[0].id, sell_order1.id);
}
