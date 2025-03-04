//! Order models and related types

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::decimal::{Price, Quantity};
#[cfg(feature = "utoipa")]
use crate::utoipa::ToSchema;

/// Order side (buy or sell)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub enum Side {
    Buy,
    Sell,
}

/// Order type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub enum OrderType {
    /// Market order to be executed immediately at the current market price
    Market,
    /// Limit order to be executed at specified price or better
    Limit,
}

/// Order time in force
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub enum TimeInForce {
    /// Good till cancelled
    GTC,
    /// Immediate or cancel
    IOC,
    /// Fill or kill
    FOK,
}

/// Order status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub enum Status {
    /// Order has been received but not yet processed
    New,
    /// Order is being processed
    PartiallyFilled,
    /// Order has been filled completely
    Filled,
    /// Order has been cancelled
    Cancelled,
    /// Order has been rejected
    Rejected,
}

/// Order model
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct Order {
    /// Unique order ID
    pub id: Uuid,
    /// User/account ID
    pub user_id: Uuid,
    /// Market symbol (e.g., "BTC/USD")
    pub market: String,
    /// Order side (buy or sell)
    pub side: Side,
    /// Order type
    pub order_type: OrderType,
    /// Price (for limit orders)
    pub price: Option<Price>,
    /// Original quantity
    pub quantity: Quantity,
    /// Remaining quantity
    pub remaining_quantity: Quantity,
    /// Cumulative matched quantity
    pub filled_quantity: Quantity,
    /// Average fill price
    pub average_fill_price: Option<Price>,
    /// Time in force
    pub time_in_force: TimeInForce,
    /// Current status
    pub status: Status,
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
    /// Last update timestamp
    pub updated_at: DateTime<Utc>,
}

impl Order {
    /// Create a new limit order
    pub fn new_limit(
        user_id: Uuid,
        market: String,
        side: Side,
        price: Price,
        quantity: Quantity,
        time_in_force: TimeInForce,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            user_id,
            market,
            side,
            order_type: OrderType::Limit,
            price: Some(price),
            quantity,
            remaining_quantity: quantity,
            filled_quantity: Quantity::ZERO,
            average_fill_price: None,
            time_in_force,
            status: Status::New,
            created_at: now,
            updated_at: now,
        }
    }
    
    /// Create a new market order
    pub fn new_market(
        user_id: Uuid,
        market: String,
        side: Side,
        quantity: Quantity,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            user_id,
            market,
            side,
            order_type: OrderType::Market,
            price: None,
            quantity,
            remaining_quantity: quantity,
            filled_quantity: Quantity::ZERO,
            average_fill_price: None,
            time_in_force: TimeInForce::IOC, // Market orders are IOC by default
            status: Status::New,
            created_at: now,
            updated_at: now,
        }
    }
    
    /// Check if the order is fully filled
    pub fn is_filled(&self) -> bool {
        self.remaining_quantity.is_zero() || self.status == Status::Filled
    }
    
    /// Check if the order is active (can be matched)
    pub fn is_active(&self) -> bool {
        matches!(self.status, Status::New | Status::PartiallyFilled)
    }
}
