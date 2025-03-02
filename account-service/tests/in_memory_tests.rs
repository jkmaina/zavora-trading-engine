use common::decimal::{Quantity, dec};
use common::model::account::{Account, Balance};
use common::model::order::{Order, OrderType, Side, Status, TimeInForce};
use common::model::trade::Trade;
use account_service::{AccountService, InMemoryAccountRepository, RepositoryType};
use uuid::Uuid;

// No longer needed as all tests are now using #[tokio::test]

#[tokio::test]
async fn test_create_account() {
    let _repo = InMemoryAccountRepository::new();
    let account_id = Uuid::new_v4();
    
    // Verify basic operations
    assert!(_repo.accounts.is_empty());
    
    // Add an account
    let account = Account {
        id: account_id,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    };
    _repo.accounts.insert(account_id, account);
    
    // Check it was added
    assert_eq!(_repo.accounts.len(), 1);
    assert!(_repo.accounts.contains_key(&account_id));
}

#[tokio::test]
async fn test_balance_operations() {
    let _repo = InMemoryAccountRepository::new();
    let account_id = Uuid::new_v4();
    
    // Create a balance
    let mut balance = Balance::new(account_id, "BTC".to_string());
    
    // Test deposit
    balance.deposit(dec!(1));
    assert_eq!(balance.total, dec!(1));
    assert_eq!(balance.available, dec!(1));
    
    // Test lock
    balance.lock(dec!(0.5)).unwrap(); // Lock 0.5 BTC
    assert_eq!(balance.total, dec!(1));
    assert_eq!(balance.available, dec!(0.5));
    assert_eq!(balance.locked, dec!(0.5));
    
    // Test unlock
    balance.unlock(dec!(0.2)); // Unlock 0.2 BTC
    assert_eq!(balance.total, dec!(1));
    assert_eq!(balance.available, dec!(0.7));
    assert_eq!(balance.locked, dec!(0.3));
    
    // Test withdraw
    balance.withdraw(dec!(0.5)).unwrap(); // Withdraw 0.5 BTC
    assert_eq!(balance.total, dec!(0.5));
    assert_eq!(balance.available, dec!(0.2));
    assert_eq!(balance.locked, dec!(0.3));
    
    // Test insufficient balance
    let withdraw_result = balance.withdraw(dec!(1));
    assert!(withdraw_result.is_err());
}

#[tokio::test]
async fn test_account_service_operations() {
    // Create service with in-memory repository
    let service = AccountService::with_repository(RepositoryType::InMemory).await.unwrap();
    
    // Create account
    let account = service.create_account().await.unwrap();
    assert!(account.id != Uuid::nil());
    
    // Deposit funds
    let usd_amount = dec!(1000);
    let btc_amount = dec!(5);
    service.deposit(account.id, "USD", usd_amount).await.unwrap();
    service.deposit(account.id, "BTC", btc_amount).await.unwrap();
    
    // Check balances
    let usd_balance = service.get_balance(account.id, "USD").await.unwrap().unwrap();
    let btc_balance = service.get_balance(account.id, "BTC").await.unwrap().unwrap();
    
    assert_eq!(usd_balance.total, usd_amount);
    assert_eq!(usd_balance.available, usd_amount);
    assert_eq!(btc_balance.total, btc_amount);
    assert_eq!(btc_balance.available, btc_amount);
    
    // Create and process orders
    let buy_order = Order {
        id: Uuid::new_v4(),
        user_id: account.id,
        market: "BTC/USD".to_string(),
        side: Side::Buy,
        order_type: OrderType::Limit,
        price: Some(dec!(100)),
        quantity: dec!(2),
        filled_quantity: Quantity::ZERO,
        remaining_quantity: dec!(2),
        average_fill_price: None,
        time_in_force: TimeInForce::GTC,
        status: Status::New,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    };
    
    // Reserve funds
    service.reserve_for_order(&buy_order).await.unwrap();
    
    // Check updated balance (funds should be locked)
    let updated_usd = service.get_balance(account.id, "USD").await.unwrap().unwrap();
    assert_eq!(updated_usd.locked, dec!(200)); // 2 BTC * $100
    assert_eq!(updated_usd.available, dec!(800));
    assert_eq!(updated_usd.total, dec!(1000));
}

#[tokio::test]
async fn test_trade_execution() {
    // Create service
    let service = AccountService::with_repository(RepositoryType::InMemory).await.unwrap();
    
    // Create two accounts
    let buyer = service.create_account().await.unwrap();
    let seller = service.create_account().await.unwrap();
    
    // Fund accounts
    service.deposit(buyer.id, "USD", dec!(1000)).await.unwrap();
    service.deposit(seller.id, "BTC", dec!(10)).await.unwrap();
    
    // Create orders
    let buy_order = Order {
        id: Uuid::new_v4(),
        user_id: buyer.id,
        market: "BTC/USD".to_string(),
        side: Side::Buy,
        order_type: OrderType::Limit,
        price: Some(dec!(100)),
        quantity: dec!(3),
        filled_quantity: Quantity::ZERO,
        remaining_quantity: dec!(3),
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
        price: Some(dec!(100)),
        quantity: dec!(3),
        filled_quantity: Quantity::ZERO,
        remaining_quantity: dec!(3),
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
        price: dec!(100),
        quantity: dec!(3),
        amount: dec!(300), // 3 * 100
        taker_side: Side::Buy,
        created_at: chrono::Utc::now(),
    };
    
    service.process_trade(&trade).await.unwrap();
    
    // Verify final balances
    let buyer_usd = service.get_balance(buyer.id, "USD").await.unwrap().unwrap();
    let buyer_btc = service.get_balance(buyer.id, "BTC").await.unwrap().unwrap();
    let seller_usd = service.get_balance(seller.id, "USD").await.unwrap().unwrap();
    let seller_btc = service.get_balance(seller.id, "BTC").await.unwrap().unwrap();
    
    assert_eq!(buyer_usd.total, dec!(700)); // 1000 - (3 * 100)
    assert_eq!(buyer_usd.available, dec!(700));
    assert_eq!(buyer_usd.locked, Quantity::ZERO);
    
    assert_eq!(buyer_btc.total, dec!(3));
    assert_eq!(buyer_btc.available, dec!(3));
    
    assert_eq!(seller_usd.total, dec!(300)); // 0 + (3 * 100)
    assert_eq!(seller_usd.available, dec!(300));
    
    assert_eq!(seller_btc.total, dec!(7)); // 10 - 3
    assert_eq!(seller_btc.available, dec!(7));
    assert_eq!(seller_btc.locked, Quantity::ZERO);
}