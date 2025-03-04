// api-gateway/src/lib.rs
pub mod api;
pub mod error;
pub mod config;
pub mod ws;

use std::sync::Arc;
use account_service::AccountService;
use market_data::MarketDataService;
use matching_engine::MatchingEngine;
use common::model::market::Market;

/// App state shared across handlers
pub struct AppState {
    /// Matching engine
    pub matching_engine: Arc<MatchingEngine>,
    /// Account service
    pub account_service: Arc<AccountService>,
    /// Market data service
    pub market_data_service: Arc<MarketDataService>,
    /// Available markets
    pub markets: Vec<Market>,
}