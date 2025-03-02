//! Trade models and related types

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::decimal::{Price, Quantity, Amount};
use crate::model::order::Side;
#[cfg(feature = "utoipa")]
use crate::utoipa::ToSchema;

/// Trade model representing a matched order
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct Trade {
    /// Unique trade ID
    pub id: Uuid,
    /// Market symbol (e.g., "BTC/USD")
    pub market: String,
    /// Price at which the trade executed
    pub price: Price,
    /// Quantity traded
    pub quantity: Quantity,
    /// Total amount (price * quantity)
    pub amount: Amount,
    /// Buyer order ID
    pub buyer_order_id: Uuid,
    /// Seller order ID
    pub seller_order_id: Uuid,
    /// Buyer user ID
    pub buyer_id: Uuid,
    /// Seller user ID
    pub seller_id: Uuid,
    /// Side that was the taker (initiated the match)
    pub taker_side: Side,
    /// Timestamp when the trade occurred
    pub created_at: DateTime<Utc>,
}

impl Trade {
    /// Create a new trade from matched orders
    pub fn new(
        market: String,
        price: Price,
        quantity: Quantity,
        buyer_order_id: Uuid,
        seller_order_id: Uuid,
        buyer_id: Uuid,
        seller_id: Uuid,
        taker_side: Side,
    ) -> Self {
        let amount = price * quantity;
        Self {
            id: Uuid::new_v4(),
            market,
            price,
            quantity,
            amount,
            buyer_order_id,
            seller_order_id,
            buyer_id,
            seller_id,
            taker_side,
            created_at: Utc::now(),
        }
    }
}
