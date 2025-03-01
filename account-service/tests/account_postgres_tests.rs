use account_service::{AccountService, RepositoryType};
use common::decimal::{Quantity, dec};
use common::model::order::{Order, OrderType, Side, Status, TimeInForce};
use common::model::trade::Trade;
use uuid::Uuid;
use tokio::test;

use dotenv::dotenv;

// PostgreSQL integration tests for account service
// These tests require a running PostgreSQL database
// Run with: cargo test --test account_postgres_tests -- --ignored

async fn create_test_service() -> AccountService {
    dotenv().ok(); // Load .env.test if it exists

    let database_url = std::env::var("TEST_DATABASE_URL")
        .expect("TEST_DATABASE_URL must be set to run PostgreSQL tests");

    AccountService::with_repository(RepositoryType::Postgres(Some(database_url)))
        .await
        .expect("Failed to create account service with PostgreSQL repository")
}

#[test]
#[ignore = "Requires test database"]
async fn test_postgres_account_creation() {
    let service = create_test_service().await;
    
    // Create account
    let account = service.create_account().await.unwrap();
    assert!(account.id != Uuid::nil());
    
    // Verify account exists
    let retrieved = service.get_account(account.id).await.unwrap().unwrap();
    assert_eq!(retrieved.id, account.id);
}

#[test]
#[ignore = "Requires test database"]
async fn test_postgres_balance_operations() {
    let service = create_test_service().await;
    
    // Create account
    let account = service.create_account().await.unwrap();
    
    // Deposit funds
    let amount = Quantity::from(100);
    let asset = "USD";
    let balance = service.deposit(account.id, asset, amount).await.unwrap();
    
    // Verify balance
    assert_eq!(balance.account_id, account.id);
    assert_eq!(balance.asset, asset);
    assert_eq!(balance.total, amount);
    assert_eq!(balance.available, amount);
    assert_eq!(balance.locked, Quantity::ZERO);
    
    // Withdraw funds
    let withdraw_amount = Quantity::from(30);
    let balance = service.withdraw(account.id, asset, withdraw_amount).await.unwrap();
    
    // Verify balance after withdrawal
    assert_eq!(balance.total, amount - withdraw_amount);
    assert_eq!(balance.available, amount - withdraw_amount);
    
    // Test insufficient funds
    let over_withdraw = Quantity::from(200);
    let result = service.withdraw(account.id, asset, over_withdraw).await;
    assert!(result.is_err());
}

#[test]
#[ignore = "Requires test database"]
async fn test_postgres_order_reserves() {
    let service = create_test_service().await;
    
    // Create account
    let account = service.create_account().await.unwrap();
    
    // Deposit funds
    service.deposit(account.id, "USD", Quantity::from(1000)).await.unwrap();
    service.deposit(account.id, "BTC", Quantity::from(5)).await.unwrap();
    
    // Create buy order
    let buy_order = Order {
        id: Uuid::new_v4(),
        user_id: account.id,
        market: "BTC/USD".to_string(),
        side: Side::Buy,
        order_type: OrderType::Limit,
        price: Some(Quantity::from(100)),
        quantity: Quantity::from(2),
        filled_quantity: Quantity::ZERO,
        remaining_quantity: Quantity::from(2),
        average_fill_price: None,
        time_in_force: TimeInForce::GTC,
        status: Status::New,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    };
    
    // Reserve funds for buy order
    service.reserve_for_order(&buy_order).await.unwrap();
    
    // Check balance for locked funds
    let usd_balance = service.get_balance(account.id, "USD").await.unwrap().unwrap();
    assert_eq!(usd_balance.locked, Quantity::from(200)); // 2 BTC * $100
    assert_eq!(usd_balance.available, Quantity::from(800));
    
    // Create sell order
    let sell_order = Order {
        id: Uuid::new_v4(),
        user_id: account.id,
        market: "BTC/USD".to_string(),
        side: Side::Sell,
        order_type: OrderType::Limit,
        price: Some(Quantity::from(100)),
        quantity: Quantity::from(1),
        filled_quantity: Quantity::ZERO,
        remaining_quantity: Quantity::from(1),
        average_fill_price: None,
        time_in_force: TimeInForce::GTC,
        status: Status::New,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    };
    
    // Reserve funds for sell order
    service.reserve_for_order(&sell_order).await.unwrap();
    
    // Check BTC balance
    let btc_balance = service.get_balance(account.id, "BTC").await.unwrap().unwrap();
    assert_eq!(btc_balance.locked, Quantity::from(1));
    assert_eq!(btc_balance.available, Quantity::from(4));
    
    // Release funds from canceled order
    let canceled_buy = Order {
        id: buy_order.id,
        user_id: account.id,
        market: "BTC/USD".to_string(),
        side: Side::Buy,
        order_type: OrderType::Limit,
        price: Some(Quantity::from(100)),
        quantity: Quantity::from(2),
        filled_quantity: Quantity::from(1),
        remaining_quantity: Quantity::from(1), // 1 BTC unfilled
        average_fill_price: Some(Quantity::from(100)),
        time_in_force: TimeInForce::GTC,
        status: Status::Cancelled,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    };
    
    // Release funds
    service.release_reserved_funds(&canceled_buy).await.unwrap();
    
    // Verify released funds
    let updated_usd = service.get_balance(account.id, "USD").await.unwrap().unwrap();
    assert_eq!(updated_usd.locked, Quantity::from(100)); // $100 still locked
    assert_eq!(updated_usd.available, Quantity::from(900)); // $900 available
}

#[test]
#[ignore = "Requires test database"]
async fn test_postgres_trade_processing() {
    let service = create_test_service().await;
    
    // Create buyer and seller accounts
    let buyer = service.create_account().await.unwrap();
    let seller = service.create_account().await.unwrap();
    
    // Fund accounts
    service.deposit(buyer.id, "USD", Quantity::from(1000)).await.unwrap();
    service.deposit(seller.id, "BTC", Quantity::from(10)).await.unwrap();
    
    // Create orders
    let buy_order = Order {
        id: Uuid::new_v4(),
        user_id: buyer.id,
        market: "BTC/USD".to_string(),
        side: Side::Buy,
        order_type: OrderType::Limit,
        price: Some(Quantity::from(100)),
        quantity: Quantity::from(3),
        filled_quantity: Quantity::ZERO,
        remaining_quantity: Quantity::from(3),
        average_fill_price: None,
        time_in_force: TimeInForce::GTC,
        status: Status::New,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    };
    
    let sell_order = Order {
        id: Uuid::new_v4(),
        user_id: seller.id,
        market: "BTC/USD".to_string(),
        side: Side::Sell,
        order_type: OrderType::Limit,
        price: Some(Quantity::from(100)),
        quantity: Quantity::from(3),
        filled_quantity: Quantity::ZERO,
        remaining_quantity: Quantity::from(3),
        average_fill_price: None,
        time_in_force: TimeInForce::GTC,
        status: Status::New,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    };
    
    // Reserve funds
    service.reserve_for_order(&buy_order).await.unwrap();
    service.reserve_for_order(&sell_order).await.unwrap();
    
    // Execute trade
    let trade = Trade {
        id: Uuid::new_v4(),
        market: "BTC/USD".to_string(),
        buyer_id: buyer.id,
        seller_id: seller.id,
        buyer_order_id: buy_order.id,
        seller_order_id: sell_order.id,
        price: Quantity::from(100),
        quantity: Quantity::from(3),
        amount: Quantity::from(300), // 3 * 100
        taker_side: Side::Buy,
        created_at: chrono::Utc::now(),
    };
    
    service.process_trade(&trade).await.unwrap();
    
    // Verify final balances
    let buyer_usd = service.get_balance(buyer.id, "USD").await.unwrap().unwrap();
    let buyer_btc = service.get_balance(buyer.id, "BTC").await.unwrap().unwrap();
    let seller_usd = service.get_balance(seller.id, "USD").await.unwrap().unwrap();
    let seller_btc = service.get_balance(seller.id, "BTC").await.unwrap().unwrap();
    
    assert_eq!(buyer_usd.total, Quantity::from(700)); // 1000 - (3 * 100)
    assert_eq!(buyer_usd.available, Quantity::from(700));
    assert_eq!(buyer_usd.locked, Quantity::ZERO);
    
    assert_eq!(buyer_btc.total, Quantity::from(3));
    assert_eq!(buyer_btc.available, Quantity::from(3));
    
    assert_eq!(seller_usd.total, Quantity::from(300)); // 0 + (3 * 100)
    assert_eq!(seller_usd.available, Quantity::from(300));
    
    assert_eq!(seller_btc.total, Quantity::from(7)); // 10 - 3
    assert_eq!(seller_btc.available, Quantity::from(7));
    assert_eq!(seller_btc.locked, Quantity::ZERO);
}