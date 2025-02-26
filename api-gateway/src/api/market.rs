//! Market API handlers

use std::sync::Arc;

use axum::{
    extract::{Path, Query, State},
    Json,
};
use market_data::CandleInterval;
use serde::{Deserialize, Serialize};

use crate::error::ApiError;
use crate::AppState;

/// Get markets response
#[derive(Debug, Serialize)]
pub struct GetMarketsResponse {
    /// Markets
    pub markets: Vec<common::model::market::Market>,
}

/// Get all markets
pub async fn get_markets(
    State(state): State<Arc<AppState>>,
) -> Result<Json<GetMarketsResponse>, ApiError> {
    Ok(Json(GetMarketsResponse {
        markets: state.markets.clone(),
    }))
}

/// Order book query parameters
#[derive(Debug, Deserialize)]
pub struct OrderBookQuery {
    /// Depth limit
    #[serde(default = "default_depth")]
    pub depth: usize,
}

fn default_depth() -> usize {
    10
}

/// Get order book response
#[derive(Debug, Serialize)]
pub struct GetOrderBookResponse {
    /// Market
    pub market: String,
    /// Bids (price, quantity)
    pub bids: Vec<(common::decimal::Price, common::decimal::Quantity)>,
    /// Asks (price, quantity)
    pub asks: Vec<(common::decimal::Price, common::decimal::Quantity)>,
}

/// Get order book
pub async fn get_order_book(
    State(state): State<Arc<AppState>>,
    Path(market): Path<String>,
    Query(query): Query<OrderBookQuery>,
) -> Result<Json<GetOrderBookResponse>, ApiError> {
    let (bids, asks) = state.matching_engine.get_market_depth(&market, query.depth)
        .map_err(ApiError::Common)?;
    
    Ok(Json(GetOrderBookResponse {
        market,
        bids,
        asks,
    }))
}

/// Get ticker response
#[derive(Debug, Serialize)]
pub struct GetTickerResponse {
    /// Ticker
    pub ticker: market_data::Ticker,
}

/// Get ticker
pub async fn get_ticker(
    State(state): State<Arc<AppState>>,
    Path(market): Path<String>,
) -> Result<Json<GetTickerResponse>, ApiError> {
    let ticker = state.market_data_service.get_ticker(&market)
        .ok_or_else(|| ApiError::NotFound(format!("Ticker not found for market: {}", market)))?;
    
    Ok(Json(GetTickerResponse { ticker }))
}

/// Get tickers response
#[derive(Debug, Serialize)]
pub struct GetTickersResponse {
    /// Tickers
    pub tickers: Vec<market_data::Ticker>,
}

/// Get all tickers
pub async fn get_tickers(
    State(state): State<Arc<AppState>>,
) -> Result<Json<GetTickersResponse>, ApiError> {
    let tickers = state.market_data_service.get_all_tickers();
    
    Ok(Json(GetTickersResponse { tickers }))
}

/// Trades query parameters
#[derive(Debug, Deserialize)]
pub struct TradesQuery {
    /// Limit
    #[serde(default = "default_trades_limit")]
    pub limit: usize,
}

fn default_trades_limit() -> usize {
    100
}

/// Get trades response
#[derive(Debug, Serialize)]
pub struct GetTradesResponse {
    /// Market
    pub market: String,
    /// Trades
    pub trades: Vec<market_data::TradeMessage>,
}

/// Get recent trades
pub async fn get_trades(
    State(state): State<Arc<AppState>>,
    Path(market): Path<String>,
    Query(query): Query<TradesQuery>,
) -> Result<Json<GetTradesResponse>, ApiError> {
    let trades = state.market_data_service.get_recent_trades(&market, query.limit);
    
    Ok(Json(GetTradesResponse { market, trades }))
}

/// Candles query parameters
#[derive(Debug, Deserialize)]
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

/// Get candles response
#[derive(Debug, Serialize)]
pub struct GetCandlesResponse {
    /// Market
    pub market: String,
    /// Interval
    pub interval: String,
    /// Candles
    pub candles: Vec<market_data::Candle>,
}

/// Get candles
pub async fn get_candles(
    State(state): State<Arc<AppState>>,
    Path(market): Path<String>,
    Query(query): Query<CandlesQuery>,
) -> Result<Json<GetCandlesResponse>, ApiError> {
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
    
    let candles = state.market_data_service.get_candles(&market, interval, query.limit);
    
    Ok(Json(GetCandlesResponse {
        market,
        interval: query.interval,
        candles,
    }))
}