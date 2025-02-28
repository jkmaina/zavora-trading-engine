use std::sync::Arc;
use uuid::Uuid;
use common::decimal::Quantity;
use common::error::Error;
use common::model::order::{Order, OrderType, Side, TimeInForce};
use common::model::trade::Trade;
use account_service::service::AccountService;

#[test]
fn test_create_account() {
    let service = AccountService::new();
    let account = service.create_account();
    
    assert!(account.id != Uuid::nil());
    assert_eq!(account.created_at.date(), chrono::Utc::now().date());
}

#[test]
fn test_get_account() {
    let service = AccountService::new();
    let account = service.create_account();
    
    let retrieved = service.get_account(account.id).unwrap();
    assert_eq!(retrieved.id, account.id);
    
    let non_existent = service.get_account(Uuid::new_v4());
    assert!(non_existent.is_none());
}

#[test]
fn test_deposit() {
    let service = AccountService::new();
    let account = service.create_account();
    
    let balance = service.deposit(account.id, "BTC", Quantity::new(1, 0)).unwrap();
    
    assert_eq!(balance.account_id, account.id);
    assert_eq!(balance.asset, "BTC");
    assert_eq!(balance.total, Quantity::new(1, 0));
    assert_eq!(balance.available, Quantity::new(1, 0));
    assert_eq!(balance.locked, Quantity::ZERO);
}

#[test]
fn test_withdraw_success() {
    let service = AccountService::new();
    let account = service.create_account();
    
    // Deposit first
    service.deposit(account.id, "ETH", Quantity::new(5, 0)).unwrap();
    
    // Then withdraw
    let balance = service.withdraw(account.id, "ETH", Quantity::new(2, 0)).unwrap();
    
    assert_eq!(balance.total, Quantity::new(3, 0));
    assert_eq!(balance.available, Quantity::new(3, 0));
}

#[test]
fn test_withdraw_insufficient_balance() {
    let service = AccountService::new();
    let account = service.create_account();
    
    // Deposit a small amount
    service.deposit(account.id, "ETH", Quantity::new(1, 0)).unwrap();
    
    // Try to withdraw more than available
    let result = service.withdraw(account.id, "ETH", Quantity::new(2, 0));
    
    assert!(result.is_err());
    if let Err(Error::InsufficientBalance(_)) = result {
        // Expected error
    } else {
        panic!("Expected InsufficientBalance error");
    }
}

#[test]
fn test_reserve_for_buy_order() {
    let service = AccountService::new();
    let account = service.create_account();
    
    // Deposit quote currency
    service.deposit(account.id, "USD", Quantity::new(1000, 0)).unwrap();
    
    // Create a buy order
    let order = Order {
        id: Uuid::new_v4(),
        user_id: account.id,
        market: "BTC/USD".to_string(),
        side: Side::Buy,
        order_type: OrderType::Limit,
        price: Some(Quantity::new(10000, 0)),
        quantity: Quantity::new(0, 1), // 0.1 BTC
        remaining_quantity: Quantity::new(0, 1),
        filled_quantity: Quantity::ZERO,
        status: common::model::order::OrderStatus::Open,
        time_in_force: TimeInForce::GTC,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
        average_fill_price: None,
    };
    
    // Reserve funds
    let result = service.reserve_for_order(&order);
    assert!(result.is_ok());
    
    // Check balance
    let balance = service.get_balance(account.id, "USD").unwrap();
    assert_eq!(balance.total, Quantity::new(1000, 0));
    assert_eq!(balance.available, Quantity::new(0, 0));
    assert_eq!(balance.locked, Quantity::new(1000, 0));
}

#[test]
fn test_reserve_for_sell_order() {
    let service = AccountService::new();
    let account = service.create_account();
    
    // Deposit base currency
    service.deposit(account.id, "BTC", Quantity::new(1, 0)).unwrap();
    
    // Create a sell order
    let order = Order {
        id: Uuid::new_v4(),
        user_id: account.id,
        market: "BTC/USD".to_string(),
        side: Side::Sell,
        order_type: OrderType::Limit,
        price: Some(Quantity::new(10000, 0)),
        quantity: Quantity::new(0, 5), // 0.5 BTC
        remaining_quantity: Quantity::new(0, 5),
        filled_quantity: Quantity::ZERO,
        status: common::model::order::OrderStatus::Open,
        time_in_force: TimeInForce::GTC,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
        average_fill_price: None,
    };
    
    // Reserve funds
    let result = service.reserve_for_order(&order);
    assert!(result.is_ok());
    
    // Check balance
    let balance = service.get_balance(account.id, "BTC").unwrap();
    assert_eq!(balance.total, Quantity::new(1, 0));
    assert_eq!(balance.available, Quantity::new(0, 5)); // 0.5 BTC
    assert_eq!(balance.locked, Quantity::new(0, 5)); // 0.5 BTC
}

#[test]
fn test_release_reserved_funds() {
    let service = AccountService::new();
    let account = service.create_account();
    
    // Deposit quote currency
    service.deposit(account.id, "USD", Quantity::new(1000, 0)).unwrap();
    
    // Create a buy order
    let order = Order {
        id: Uuid::new_v4(),
        user_id: account.id,
        market: "BTC/USD".to_string(),
        side: Side::Buy,
        order_type: OrderType::Limit,
        price: Some(Quantity::new(10000, 0)),
        quantity: Quantity::new(0, 1), // 0.1 BTC
        remaining_quantity: Quantity::new(0, 1),
        filled_quantity: Quantity::ZERO,
        status: common::model::order::OrderStatus::Open,
        time_in_force: TimeInForce::GTC,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
        average_fill_price: None,
    };
    
    // Reserve funds
    service.reserve_for_order(&order).unwrap();
    
    // Release funds
    let result = service.release_reserved_funds(&order);
    assert!(result.is_ok());
    
    // Check balance
    let balance = service.get_balance(account.id, "USD").unwrap();
    assert_eq!(balance.total, Quantity::new(1000, 0));
    assert_eq!(balance.available, Quantity::new(1000, 0));
    assert_eq!(balance.locked, Quantity::ZERO);
}

#[test]
fn test_process_trade() {
    let service = AccountService::new();
    
    // Create buyer and seller accounts
    let buyer = service.create_account();
    let seller = service.create_account();
    
    // Deposit funds
    service.deposit(buyer.id, "USD", Quantity::new(10000, 0)).unwrap();
    service.deposit(seller.id, "BTC", Quantity::new(1, 0)).unwrap();
    
    // Create buy and sell orders
    let buy_order = Order {
        id: Uuid::new_v4(),
        user_id: buyer.id,
        market: "BTC/USD".to_string(),
        side: Side::Buy,
        order_type: OrderType::Limit,
        price: Some(Quantity::new(10000, 0)),
        quantity: Quantity::new(0, 1), // 0.1 BTC
        remaining_quantity: Quantity::new(0, 1),
        filled_quantity: Quantity::ZERO,
        status: common::model::order::OrderStatus::Open,
        time_in_force: TimeInForce::GTC,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
        average_fill_price: None,
    };
    
    let sell_order = Order {
        id: Uuid::new_v4(),
        user_id: seller.id,
        market: "BTC/USD".to_string(),
        side: Side::Sell,
        order_type: OrderType::Limit,
        price: Some(Quantity::new(10000, 0)),
        quantity: Quantity::new(0, 1), // 0.1 BTC
        remaining_quantity: Quantity::new(0, 1),
        filled_quantity: Quantity::ZERO,
        status: common::model::order::OrderStatus::Open,
        time_in_force: TimeInForce::GTC,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
        average_fill_price: None,
    };
    
    // Reserve funds for orders
    service.reserve_for_order(&buy_order).unwrap();
    service.reserve_for_order(&sell_order).unwrap();
    
    // Create a trade
    let trade = Trade {
        id: Uuid::new_v4(),
        market: "BTC/USD".to_string(),
        price: Quantity::new(10000, 0),
        quantity: Quantity::new(0, 1), // 0.1 BTC
        buyer_order_id: buy_order.id,
        seller_order_id: sell_order.id,
        buyer_id: buyer.id,
        seller_id: seller.id,
        taker_side: Side::Buy,
        created_at: chrono::Utc::now(),
    };
    
    // Process the trade
    let result = service.process_trade(&trade);
    assert!(result.is_ok());
    
    // Check buyer balances
    let buyer_btc = service.get_balance(buyer.id, "BTC").unwrap();
    let buyer_usd = service.get_balance(buyer.id, "USD").unwrap();
    
    assert_eq!(buyer_btc.total, Quantity::new(0, 1)); // 0.1 BTC
    assert_eq!(buyer_btc.available, Quantity::new(0, 1));
    assert_eq!(buyer_usd.total, Quantity::new(9000, 0)); // 10000 - 1000
    assert_eq!(buyer_usd.locked, Quantity::ZERO);
    
    // Check seller balances
    let seller_btc = service.get_balance(seller.id, "BTC").unwrap();
    let seller_usd = service.get_balance(seller.id, "USD").unwrap();
    
    assert_eq!(seller_btc.total, Quantity::new(0, 9)); // 1.0 - 0.1 BTC
    assert_eq!(seller_btc.locked, Quantity::ZERO);
    assert_eq!(seller_usd.total, Quantity::new(1000, 0));
    assert_eq!(seller_usd.available, Quantity::new(1000, 0));
}
