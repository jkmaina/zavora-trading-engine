use chrono::{DateTime, Utc};
use sqlx::FromRow;
use uuid::Uuid;
use rust_decimal::Decimal;

use crate::model::order::{Side, Status as OrderStatus, OrderType};

/// Database model for Account table
#[derive(Debug, Clone, FromRow)]
pub struct DbAccount {
    pub id: Uuid,
    pub user_id: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Database model for Balance table
#[derive(Debug, Clone, FromRow)]
pub struct DbBalance {
    pub id: Uuid,
    pub account_id: Uuid,
    pub asset: String,
    pub total: Decimal,
    pub available: Decimal,
    pub locked: Decimal,
    pub updated_at: DateTime<Utc>,
}

/// Database model for Order table
#[derive(Debug, Clone, FromRow)]
pub struct DbOrder {
    pub id: Uuid,
    pub account_id: Uuid,
    pub market_id: String,
    pub side: Side,
    pub order_type: OrderType,
    pub price: Option<Decimal>,
    pub quantity: Decimal,
    pub filled_quantity: Decimal,
    pub status: OrderStatus,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Database model for Market table
#[derive(Debug, Clone, FromRow)]
pub struct DbMarket {
    pub id: String,
    pub base_asset: String,
    pub quote_asset: String,
    pub min_price: Decimal,
    pub max_price: Decimal,
    pub tick_size: Decimal,
    pub min_quantity: Decimal,
    pub max_quantity: Decimal,
    pub step_size: Decimal,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Database model for Trade table
#[derive(Debug, Clone, FromRow)]
pub struct DbTrade {
    pub id: Uuid,
    pub market_id: String,
    pub maker_order_id: Uuid,
    pub taker_order_id: Uuid,
    pub price: Decimal,
    pub quantity: Decimal,
    pub executed_at: DateTime<Utc>,
}