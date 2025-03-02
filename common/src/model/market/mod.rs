//! Market models and related types

use serde::{Deserialize, Serialize};

use crate::decimal::{Price, Quantity};
#[cfg(feature = "utoipa")]
use crate::utoipa::ToSchema;

/// Market configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct Market {
    /// Market symbol (e.g., "BTC/USD")
    pub symbol: String,
    /// Base asset (e.g., "BTC")
    pub base_asset: String,
    /// Quote asset (e.g., "USD")
    pub quote_asset: String,
    /// Minimum price change (tick size)
    pub price_tick: Price,
    /// Minimum quantity (lot size)
    pub quantity_step: Quantity,
    /// Minimum order size in quote currency
    pub min_order_size: Quantity,
    /// Maximum price deviation for market orders (in percent)
    pub max_price_deviation: f64,
    /// Whether trading is enabled
    pub trading_enabled: bool,
}

/// Market summary information
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct MarketSummary {
    /// Market symbol
    pub symbol: String,
    /// Last trade price
    pub last_price: Option<Price>,
    /// 24h price change
    pub price_change_24h: Option<Price>,
    /// 24h price change percent
    pub price_change_percent_24h: Option<f64>,
    /// 24h high price
    pub high_24h: Option<Price>,
    /// 24h low price
    pub low_24h: Option<Price>,
    /// 24h volume in base asset
    pub volume_24h: Option<Quantity>,
    /// 24h volume in quote asset
    pub quote_volume_24h: Option<Quantity>,
}
