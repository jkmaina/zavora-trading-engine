use uuid::Uuid;
use common::decimal::{Quantity, dec};
use common::error::Error;
use common::model::order::{Order, OrderType, Side, Status, TimeInForce};
use common::model::trade::Trade;
use account_service::{AccountService, RepositoryType};
use tokio::runtime::Runtime;
#[cfg(not(feature = "db_tests"))]
use std::env;

// Focus on in-memory tests for now
const SKIP_POSTGRES_TESTS: bool = true;

// Helper function to run async tests 
fn run_async<F>(test: F)
where
    F: FnOnce() -> futures::future::BoxFuture<'static, ()> + Send + 'static,
{
    // Create runtime
    let rt = Runtime::new().unwrap();
    
    // Run the test
    rt.block_on(async {
        test().await;
    });
}

// In-memory repository tests
mod in_memory_tests {
    use super::*;
    
    #[test]
    fn test_create_account() {
        run_async(|| {
            Box::pin(async move {
                let service = AccountService::new();
                let account = service.create_account().await.unwrap();
                
                assert!(account.id != Uuid::nil());
                assert_eq!(account.created_at.date_naive(), chrono::Utc::now().date_naive());
            })
        });
    }
    
    #[test]
    fn test_get_account() {
        run_async(|| {
            Box::pin(async move {
                let service = AccountService::new();
                let account = service.create_account().await.unwrap();
                
                let retrieved = service.get_account(account.id).await.unwrap().unwrap();
                assert_eq!(retrieved.id, account.id);
                
                let non_existent = service.get_account(Uuid::new_v4()).await.unwrap();
                assert!(non_existent.is_none());
            })
        });
    }
    
    #[test]
    fn test_deposit() {
        run_async(|| {
            Box::pin(async move {
                let service = AccountService::new();
                let account = service.create_account().await.unwrap();
                
                let balance = service.deposit(account.id, "BTC", dec!(1)).await.unwrap();
                
                assert_eq!(balance.account_id, account.id);
                assert_eq!(balance.asset, "BTC");
                assert_eq!(balance.total, dec!(1));
                assert_eq!(balance.available, dec!(1));
                assert_eq!(balance.locked, Quantity::ZERO);
            })
        });
    }
    
    #[test]
    fn test_withdraw_success() {
        run_async(|| {
            Box::pin(async move {
                let service = AccountService::new();
                let account = service.create_account().await.unwrap();
                
                // Deposit first
                service.deposit(account.id, "ETH", dec!(5)).await.unwrap();
                
                // Then withdraw
                let balance = service.withdraw(account.id, "ETH", dec!(2)).await.unwrap();
                
                assert_eq!(balance.total, dec!(3));
                assert_eq!(balance.available, dec!(3));
            })
        });
    }
    
    #[test]
    fn test_withdraw_insufficient_balance() {
        run_async(|| {
            Box::pin(async move {
                let service = AccountService::new();
                let account = service.create_account().await.unwrap();
                
                // Deposit small amount
                service.deposit(account.id, "XRP", dec!(1)).await.unwrap();
                
                // Attempt to withdraw too much
                let result = service.withdraw(account.id, "XRP", dec!(2)).await;
                
                assert!(result.is_err());
                match result {
                    Err(Error::InsufficientBalance(_)) => (),
                    _ => panic!("Expected InsufficientBalance error"),
                }
            })
        });
    }
    
    #[test]
    fn test_reserve_for_order_buy() {
        run_async(|| {
            Box::pin(async move {
                let service = AccountService::new();
                let account = service.create_account().await.unwrap();
                
                // Deposit quote currency
                service.deposit(account.id, "USD", dec!(1000)).await.unwrap();
                
                // Create buy order
                let order = Order {
                    id: Uuid::new_v4(),
                    user_id: account.id,
                    market: "BTC/USD".to_string(),
                    side: Side::Buy,
                    order_type: OrderType::Limit,
                    price: Some(dec!(10000)),
                    quantity: dec!(0.1), // 0.1 BTC
                    remaining_quantity: dec!(0.1),
                    filled_quantity: Quantity::ZERO,
                    status: Status::New,
                    time_in_force: TimeInForce::GTC,
                    created_at: chrono::Utc::now(),
                    updated_at: chrono::Utc::now(),
                    average_fill_price: None,
                };
                
                // Reserve funds
                let result = service.reserve_for_order(&order).await;
                assert!(result.is_ok());
                
                // Check balance
                let balance = service.get_balance(account.id, "USD").await.unwrap().unwrap();
                assert_eq!(balance.total, dec!(1000));
                assert_eq!(balance.available, dec!(0));
                assert_eq!(balance.locked, dec!(1000));
            })
        });
    }
    
    #[test]
    fn test_process_trade() {
        run_async(|| {
            Box::pin(async move {
                let service = AccountService::new();
                
                // Create buyer and seller accounts
                let buyer = service.create_account().await.unwrap();
                let seller = service.create_account().await.unwrap();
                
                // Deposit funds
                service.deposit(buyer.id, "USD", dec!(10000)).await.unwrap();
                service.deposit(seller.id, "BTC", dec!(1)).await.unwrap();
                
                // Create buy and sell orders
                let buy_order = Order {
                    id: Uuid::new_v4(),
                    user_id: buyer.id,
                    market: "BTC/USD".to_string(),
                    side: Side::Buy,
                    order_type: OrderType::Limit,
                    price: Some(dec!(10000)),
                    quantity: dec!(0.1), // 0.1 BTC
                    remaining_quantity: dec!(0.1),
                    filled_quantity: Quantity::ZERO,
                    status: Status::New,
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
                    price: Some(dec!(10000)),
                    quantity: dec!(0.1), // 0.1 BTC
                    remaining_quantity: dec!(0.1),
                    filled_quantity: Quantity::ZERO,
                    status: Status::New,
                    time_in_force: TimeInForce::GTC,
                    created_at: chrono::Utc::now(),
                    updated_at: chrono::Utc::now(),
                    average_fill_price: None,
                };
                
                // Lock funds
                service.reserve_for_order(&buy_order).await.unwrap();
                service.reserve_for_order(&sell_order).await.unwrap();
                
                // Create trade
                let trade = Trade {
                    id: Uuid::new_v4(),
                    market: "BTC/USD".to_string(),
                    buyer_id: buyer.id,
                    seller_id: seller.id,
                    buyer_order_id: buy_order.id,
                    seller_order_id: sell_order.id,
                    price: dec!(10000),
                    quantity: dec!(0.1),
                    amount: dec!(1000), // 0.1 BTC * 10000 USD
                    taker_side: Side::Buy,
                    created_at: chrono::Utc::now(),
                };
                
                // Process trade
                let result = service.process_trade(&trade).await;
                assert!(result.is_ok());
                
                // Check buyer balances
                let buyer_usd = service.get_balance(buyer.id, "USD").await.unwrap().unwrap();
                let buyer_btc = service.get_balance(buyer.id, "BTC").await.unwrap().unwrap();
                
                assert_eq!(buyer_usd.total, dec!(9000)); // 10000 - 1000 (trade)
                assert_eq!(buyer_usd.available, dec!(9000));
                assert_eq!(buyer_usd.locked, dec!(0));
                
                assert_eq!(buyer_btc.total, dec!(0.1));
                assert_eq!(buyer_btc.available, dec!(0.1));
                
                // Check seller balances
                let seller_usd = service.get_balance(seller.id, "USD").await.unwrap().unwrap();
                let seller_btc = service.get_balance(seller.id, "BTC").await.unwrap().unwrap();
                
                assert_eq!(seller_usd.total, dec!(1000)); // 0 + 1000 (trade)
                assert_eq!(seller_usd.available, dec!(1000));
                
                assert_eq!(seller_btc.total, dec!(0.9)); // 1.0 - 0.1 (trade)
                assert_eq!(seller_btc.available, dec!(0.9));
                assert_eq!(seller_btc.locked, dec!(0));
            })
        });
    }
}

// PostgreSQL repository tests
mod postgres_tests {
    use super::*;
    
    // Helper to create a PostgreSQL service
    async fn create_postgres_service() -> Result<AccountService, Error> {
        // Skip if postgres tests are disabled
        if SKIP_POSTGRES_TESTS {
            return Err(Error::Internal("PostgreSQL tests are disabled".to_string()));
        }
        
        // Check for TEST_DATABASE_URL
        let _db_url = match env::var("TEST_DATABASE_URL") {
            Ok(url) => url,
            Err(_) => {
                return Err(Error::Internal("TEST_DATABASE_URL not set".to_string()));
            }
        };
        
        // Create service with PostgreSQL repository
        AccountService::with_repository(RepositoryType::Postgres(None)).await
    }
    
    #[test]
    fn test_postgres_create_account() {
        run_async(|| {
            Box::pin(async move {
                let service = match create_postgres_service().await {
                    Ok(svc) => svc,
                    Err(_) => return, // Skip test
                };
                
                let account = service.create_account().await.unwrap();
                
                assert!(account.id != Uuid::nil());
                assert_eq!(account.created_at.date_naive(), chrono::Utc::now().date_naive());
            })
        });
    }
    
    #[test]
    fn test_postgres_deposit() {
        run_async(|| {
            Box::pin(async move {
                // Skipped if PostgreSQL tests are disabled
                let service = match create_postgres_service().await {
                    Ok(svc) => svc,
                    Err(_) => return,
                };
                
                let account = service.create_account().await.unwrap();
                let balance = service.deposit(account.id, "BTC", dec!(1)).await.unwrap();
                
                assert_eq!(balance.account_id, account.id);
                assert_eq!(balance.asset, "BTC");
                assert_eq!(balance.total, dec!(1));
                assert_eq!(balance.available, dec!(1));
            })
        });
    }
    
    #[test]
    fn test_postgres_order_and_trade() {
        run_async(|| {
            Box::pin(async move {
                // Skipped if PostgreSQL tests are disabled
                let service = match create_postgres_service().await {
                    Ok(svc) => svc,
                    Err(_) => return,
                };
                
                // Create test accounts
                let buyer = service.create_account().await.unwrap();
                let seller = service.create_account().await.unwrap();
                
                // Fund accounts
                service.deposit(buyer.id, "USD", dec!(10000)).await.unwrap();
                service.deposit(seller.id, "BTC", dec!(5)).await.unwrap();
                
                // Create orders
                let buy_order = Order {
                    id: Uuid::new_v4(),
                    user_id: buyer.id,
                    market: "BTC/USD".to_string(),
                    side: Side::Buy,
                    order_type: OrderType::Limit,
                    price: Some(dec!(10000)),
                    quantity: dec!(1),
                    remaining_quantity: dec!(1),
                    filled_quantity: Quantity::ZERO,
                    status: Status::New,
                    time_in_force: TimeInForce::GTC,
                    created_at: chrono::Utc::now(),
                    updated_at: chrono::Utc::now(),
                    average_fill_price: None,
                };
                
                // Reserve funds
                service.reserve_for_order(&buy_order).await.unwrap();
                
                // Verify balances
                let buyer_usd = service.get_balance(buyer.id, "USD").await.unwrap().unwrap();
                assert_eq!(buyer_usd.locked, dec!(10000));
                assert_eq!(buyer_usd.available, dec!(0));
            })
        });
    }
}