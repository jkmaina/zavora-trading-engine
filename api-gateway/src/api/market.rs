//! Market API handlers
//!
//! Handlers for market data endpoints including:
//! - List all markets
//! - Get order book data
//! - Get market ticker information
//! - Retrieve market trades
//! - Get OHLCV candles

use std::sync::Arc;

use axum::{
    extract::{Path, Query, State},
};
use market_data::{CandleInterval, Ticker, TradeMessage, Candle};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::error::ApiError;
use crate::AppState;
use crate::api::response::{ApiResponse, ApiListResponse};

/// Get all markets
#[utoipa::path(
    get,
    path = "/api/v1/markets",
    responses(
        (status = 200, description = "List of available markets retrieved successfully"),
        (status = 500, description = "Internal server error")
    ),
    tag = "market"
)]
pub async fn get_markets(
    State(state): State<Arc<AppState>>,
) -> Result<ApiListResponse<common::model::market::Market>, ApiError> {
    // Return a standardized list response with all markets
    Ok(ApiListResponse::new(state.markets.clone()))
}

/// Order book query parameters
#[derive(Debug, Deserialize, ToSchema)]
pub struct OrderBookQuery {
    /// Depth limit
    #[serde(default = "default_depth")]
    pub depth: usize,
}

fn default_depth() -> usize {
    10
}

/// Order book data structure
#[derive(Debug, Serialize, ToSchema)]
pub struct OrderBookData {
    /// Market symbol
    pub market: String,
    /// Bids (price, quantity)
    pub bids: Vec<(common::decimal::Price, common::decimal::Quantity)>,
    /// Asks (price, quantity)
    pub asks: Vec<(common::decimal::Price, common::decimal::Quantity)>,
}

/// Get order book
#[utoipa::path(
    get,
    path = "/api/v1/markets/{market}/order-book",
    params(
        ("market" = String, Path, description = "Market symbol"),
        ("depth" = Option<usize>, Query, description = "Order book depth")
    ),
    responses(
        (status = 200, description = "Order book retrieved successfully"),
        (status = 404, description = "Market not found"),
        (status = 500, description = "Internal server error")
    ),
    tag = "market"
)]
pub async fn get_order_book(
    State(state): State<Arc<AppState>>,
    Path(market): Path<String>,
    Query(query): Query<OrderBookQuery>,
) -> Result<ApiResponse<OrderBookData>, ApiError> {
    // Get market depth from matching engine
    let (bids, asks) = state.matching_engine.get_market_depth(&market, query.depth)
        .map_err(ApiError::Common)?;
    
    // Create order book data
    let order_book = OrderBookData {
        market,
        bids,
        asks,
    };
    
    // Return standardized response
    Ok(ApiResponse::new(order_book))
}

/// Get ticker for a market
#[utoipa::path(
    get,
    path = "/api/v1/markets/{market}/ticker",
    params(
        ("market" = String, Path, description = "Market symbol")
    ),
    responses(
        (status = 200, description = "Ticker retrieved successfully"),
        (status = 404, description = "Market not found"),
        (status = 500, description = "Internal server error")
    ),
    tag = "market"
)]
pub async fn get_ticker(
    State(state): State<Arc<AppState>>,
    Path(market): Path<String>,
) -> Result<ApiResponse<Ticker>, ApiError> {
    // Get ticker from market data service
    let ticker = state.market_data_service.get_ticker(&market)
        .ok_or_else(|| ApiError::NotFound(format!("Ticker not found for market: {}", market)))?;
    
    // Return standardized response
    Ok(ApiResponse::new(ticker))
}

/// Get all tickers
#[utoipa::path(
    get,
    path = "/api/v1/markets/tickers",
    responses(
        (status = 200, description = "All tickers retrieved successfully"),
        (status = 500, description = "Internal server error")
    ),
    tag = "market"
)]
pub async fn get_tickers(
    State(state): State<Arc<AppState>>,
) -> Result<ApiListResponse<Ticker>, ApiError> {
    // Get all tickers from market data service
    let tickers = state.market_data_service.get_all_tickers();
    
    // Return standardized list response
    Ok(ApiListResponse::new(tickers))
}

/// Trades query parameters
#[derive(Debug, Deserialize, ToSchema)]
pub struct TradesQuery {
    /// Limit
    #[serde(default = "default_trades_limit")]
    pub limit: usize,
}

fn default_trades_limit() -> usize {
    100
}

/// Trade data structure with market information
#[derive(Debug, Serialize, ToSchema)]
pub struct MarketTradesData {
    /// Market symbol
    pub market: String,
    /// List of trades
    pub trades: Vec<TradeMessage>,
}

/// Get recent trades
#[utoipa::path(
    get,
    path = "/api/v1/markets/{market}/trades",
    params(
        ("market" = String, Path, description = "Market symbol"),
        ("limit" = Option<usize>, Query, description = "Maximum number of trades to return")
    ),
    responses(
        (status = 200, description = "Trades retrieved successfully"),
        (status = 404, description = "Market not found"),
        (status = 500, description = "Internal server error")
    ),
    tag = "market"
)]
pub async fn get_trades(
    State(state): State<Arc<AppState>>,
    Path(market): Path<String>,
    Query(query): Query<TradesQuery>,
) -> Result<ApiResponse<MarketTradesData>, ApiError> {
    // Get recent trades from market data service
    let trades = state.market_data_service.get_recent_trades(&market, query.limit);
    
    // Create trade data with market info
    let trade_data = MarketTradesData {
        market,
        trades,
    };
    
    // Return standardized response
    Ok(ApiResponse::new(trade_data))
}

/// Candles query parameters
#[derive(Debug, Deserialize, ToSchema)]
pub struct CandlesQuery {
    /// Interval
    #[serde(default = "default_interval")]
    pub interval: String,
    /// Limit
    #[serde(default = "default_candles_limit")]
    pub limit: usize,
}

fn default_interval() -> String {
    "1m".to_string()
}

fn default_candles_limit() -> usize {
    100
}

/// Market candle data structure
#[derive(Debug, Serialize, ToSchema)]
pub struct MarketCandleData {
    /// Market symbol
    pub market: String,
    /// Time interval
    pub interval: String,
    /// List of candles
    pub candles: Vec<Candle>,
}

/// Get candles for a market
#[utoipa::path(
    get,
    path = "/api/v1/markets/{market}/candles",
    params(
        ("market" = String, Path, description = "Market symbol"),
        ("interval" = Option<String>, Query, description = "Candle interval (1m, 5m, 15m, 30m, 1h, 4h, 12h, 1d, 1w)"),
        ("limit" = Option<usize>, Query, description = "Maximum number of candles to return")
    ),
    responses(
        (status = 200, description = "Candles retrieved successfully"),
        (status = 404, description = "Market not found"),
        (status = 400, description = "Invalid interval"),
        (status = 500, description = "Internal server error")
    ),
    tag = "market"
)]
pub async fn get_candles(
    State(state): State<Arc<AppState>>,
    Path(market): Path<String>,
    Query(query): Query<CandlesQuery>,
) -> Result<ApiResponse<MarketCandleData>, ApiError> {
    // Parse the interval string
    let interval = match query.interval.as_str() {
        "1m" => CandleInterval::Minute1,
        "5m" => CandleInterval::Minute5,
        "15m" => CandleInterval::Minute15,
        "30m" => CandleInterval::Minute30,
        "1h" => CandleInterval::Hour1,
        "4h" => CandleInterval::Hour4,
        "12h" => CandleInterval::Hour12,
        "1d" => CandleInterval::Day1,
        "1w" => CandleInterval::Week1,
        _ => return Err(ApiError::BadRequest(format!("Invalid interval: {}", query.interval))),
    };
    
    // Get candles from market data service
    let candles = state.market_data_service.get_candles(&market, interval, query.limit);
    
    // Create candle data
    let candle_data = MarketCandleData {
        market,
        interval: query.interval,
        candles,
    };
    
    // Return standardized response
    Ok(ApiResponse::new(candle_data))
}