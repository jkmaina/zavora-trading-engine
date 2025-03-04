//! Market data service for providing real-time market data

mod service;
mod models;
pub mod channel;

pub use service::MarketDataService;
pub use models::{
    MarketDepth, OrderBookUpdate, PriceLevel, TradeMessage, 
    Ticker, MarketSummary, Candle, CandleInterval,
};