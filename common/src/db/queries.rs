use uuid::Uuid;
use sqlx::PgPool;
use rust_decimal::Decimal;

use crate::error::Result;
use crate::model::account::Account;
use crate::model::market::Market;
use crate::model::order::{Order, Side, OrderType};
use crate::model::trade::Trade;
use chrono::Utc;

// Account Queries

pub async fn create_account(_pool: &PgPool, _user_id: &str) -> Result<Account> {
    let id = Uuid::new_v4();
    let now = Utc::now();
    
    // Mock implementation
    Ok(Account {
        id,
        created_at: now,
        updated_at: now,
    })
}

pub async fn get_account_by_id(_pool: &PgPool, id: Uuid) -> Result<Option<Account>> {
    let now = Utc::now();
    
    // Mock implementation
    Ok(Some(Account {
        id,
        created_at: now,
        updated_at: now,
    }))
}

// Balance Queries

use crate::model::account::Balance;

pub async fn get_balance(_pool: &PgPool, account_id: Uuid, asset: &str) -> Result<Option<Balance>> {
    let now = Utc::now();
    
    // Mock implementation
    Ok(Some(Balance {
        account_id,
        asset: asset.to_string(),
        total: Decimal::from(100),
        available: Decimal::from(100),
        locked: Decimal::ZERO,
        updated_at: now,
    }))
}

pub async fn get_balances(_pool: &PgPool, account_id: Uuid) -> Result<Vec<Balance>> {
    let now = Utc::now();
    
    // Mock implementation
    Ok(vec![
        Balance {
            account_id,
            asset: "USD".to_string(),
            total: Decimal::from(1000),
            available: Decimal::from(1000),
            locked: Decimal::ZERO,
            updated_at: now,
        },
        Balance {
            account_id,
            asset: "BTC".to_string(),
            total: Decimal::from(5),
            available: Decimal::from(5),
            locked: Decimal::ZERO,
            updated_at: now,
        }
    ])
}

pub async fn update_balance(
    _pool: &PgPool, 
    account_id: Uuid, 
    asset: &str, 
    total: Decimal,
    available: Decimal,
    locked: Decimal
) -> Result<Balance> {
    let now = Utc::now();
    
    // Mock implementation
    Ok(Balance {
        account_id,
        asset: asset.to_string(),
        total,
        available,
        locked,
        updated_at: now,
    })
}

// Market Queries

pub async fn create_market(
    _pool: &PgPool, 
    symbol: &str,
    base_asset: &str,
    quote_asset: &str,
    _min_price: Decimal,
    _max_price: Decimal,
    price_tick: Decimal,
    min_quantity: Decimal,
    _max_quantity: Decimal,
    quantity_step: Decimal
) -> Result<Market> {
    // Mock implementation
    Ok(Market {
        symbol: symbol.to_string(),
        base_asset: base_asset.to_string(),
        quote_asset: quote_asset.to_string(),
        price_tick,
        quantity_step,
        min_order_size: min_quantity,
        max_price_deviation: 0.05,  // Default 5% max deviation
        trading_enabled: true,
    })
}

// Order Queries

pub async fn create_order(
    _pool: &PgPool,
    user_id: Uuid,
    market: &str,
    side: Side,
    order_type: OrderType,
    price: Option<Decimal>,
    quantity: Decimal
) -> Result<Order> {
    let now = Utc::now();
    
    // Mock implementation
    Ok(Order {
        id: Uuid::new_v4(),
        user_id,
        market: market.to_string(),
        side,
        order_type,
        price,
        quantity,
        filled_quantity: Decimal::ZERO,
        remaining_quantity: quantity,
        average_fill_price: None,
        time_in_force: crate::model::order::TimeInForce::GTC, // Default
        status: crate::model::order::Status::New,
        created_at: now,
        updated_at: now,
    })
}

// Trade Queries

pub async fn create_trade(
    _pool: &PgPool,
    market: &str,
    buyer_order_id: Uuid,
    seller_order_id: Uuid,
    buyer_id: Uuid,
    seller_id: Uuid,
    price: Decimal,
    quantity: Decimal,
    taker_side: Side
) -> Result<Trade> {
    let amount = price * quantity;
    let now = Utc::now();
    
    // Mock implementation
    Ok(Trade {
        id: Uuid::new_v4(),
        market: market.to_string(),
        price,
        quantity,
        amount,
        buyer_order_id,
        seller_order_id,
        buyer_id,
        seller_id,
        taker_side,
        created_at: now,
    })
}