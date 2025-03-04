//! Market data models

use chrono::{DateTime, Utc};
use common::decimal::{Price, Quantity};
use common::model::trade::Trade;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[cfg(feature = "utoipa")]
use utoipa::ToSchema;

/// Market depth (order book)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketDepth {
    /// Market symbol
    pub market: String,
    /// Timestamp
    pub timestamp: DateTime<Utc>,
    /// Bid side (price, quantity) sorted by price in descending order
    pub bids: Vec<PriceLevel>,
    /// Ask side (price, quantity) sorted by price in ascending order
    pub asks: Vec<PriceLevel>,
}

/// Order book update message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderBookUpdate {
    /// Market symbol
    pub market: String,
    /// Timestamp
    pub timestamp: DateTime<Utc>,
    /// Bid updates (price, quantity) - quantity of 0 means remove level
    pub bids: Vec<PriceLevel>,
    /// Ask updates (price, quantity) - quantity of 0 means remove level
    pub asks: Vec<PriceLevel>,
}

/// Price level in order book
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PriceLevel {
    /// Price
    pub price: Price,
    /// Quantity
    pub quantity: Quantity,
}

/// Trade message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradeMessage {
    /// Unique trade ID
    pub id: Uuid,
    /// Market symbol
    pub market: String,
    /// Price
    pub price: Price,
    /// Quantity
    pub quantity: Quantity,
    /// Side that was the taker (initiated the match)
    pub taker_side: String, // "buy" or "sell"
    /// Timestamp
    pub timestamp: DateTime<Utc>,
}

impl From<&Trade> for TradeMessage {
    fn from(trade: &Trade) -> Self {
        Self {
            id: trade.id,
            market: trade.market.clone(),
            price: trade.price,
            quantity: trade.quantity,
            taker_side: match trade.taker_side {
                common::model::order::Side::Buy => "buy".to_string(),
                common::model::order::Side::Sell => "sell".to_string(),
            },
            timestamp: trade.created_at,
        }
    }
}

/// Market ticker
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct Ticker {
    /// Market symbol
    pub market: String,
    /// Best bid price
    pub bid: Option<Price>,
    /// Best ask price
    pub ask: Option<Price>,
    /// Last trade price
    pub last: Option<Price>,
    /// 24h price change
    pub change_24h: Option<Price>,
    /// 24h price change percentage
    pub change_24h_percent: Option<f64>,
    /// 24h high price
    pub high_24h: Option<Price>,
    /// 24h low price
    pub low_24h: Option<Price>,
    /// 24h volume in base asset
    pub volume_24h: Option<Quantity>,
    /// 24h volume in quote asset
    pub quote_volume_24h: Option<Quantity>,
    /// Timestamp
    pub timestamp: DateTime<Utc>,
}

/// Market summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketSummary {
    /// Market symbol
    pub market: String,
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
    /// Current best bid
    pub bid: Option<Price>,
    /// Current best ask
    pub ask: Option<Price>,
    /// Timestamp
    pub timestamp: DateTime<Utc>,
}

/// Candle interval
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub enum CandleInterval {
    /// 1 minute
    Minute1,
    /// 5 minutes
    Minute5,
    /// 15 minutes
    Minute15,
    /// 30 minutes
    Minute30,
    /// 1 hour
    Hour1,
    /// 4 hours
    Hour4,
    /// 12 hours
    Hour12,
    /// 1 day
    Day1,
    /// 1 week
    Week1,
}

impl CandleInterval {
    /// Get the duration in seconds
    pub fn duration_secs(&self) -> i64 {
        match self {
            CandleInterval::Minute1 => 60,
            CandleInterval::Minute5 => 300,
            CandleInterval::Minute15 => 900,
            CandleInterval::Minute30 => 1800,
            CandleInterval::Hour1 => 3600,
            CandleInterval::Hour4 => 14400,
            CandleInterval::Hour12 => 43200,
            CandleInterval::Day1 => 86400,
            CandleInterval::Week1 => 604800,
        }
    }
}

/// OHLCV candle
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct Candle {
    /// Market symbol
    pub market: String,
    /// Interval
    pub interval: CandleInterval,
    /// Open time
    pub open_time: DateTime<Utc>,
    /// Close time
    pub close_time: DateTime<Utc>,
    /// Open price
    pub open: Price,
    /// High price
    pub high: Price,
    /// Low price
    pub low: Price,
    /// Close price
    pub close: Price,
    /// Volume in base asset
    pub volume: Quantity,
    /// Volume in quote asset
    pub quote_volume: Quantity,
    /// Number of trades
    pub trades: u64,
}