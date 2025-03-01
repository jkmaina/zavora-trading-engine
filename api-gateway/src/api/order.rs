//! Order API handlers
//!
//! Handlers for order management endpoints including:
//! - Place new orders
//! - Cancel existing orders
//! - Get order details
//! - List orders by user

use std::sync::Arc;

use axum::{
    extract::{Path, Query, State},
    Json,
};
use common::model::order::{Order, OrderType, Side, TimeInForce};
use common::model::trade::Trade;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::ApiError;
use crate::AppState;
use crate::api::response::{ApiResponse, ApiListResponse};

/// Place order request
#[derive(Debug, Deserialize)]
pub struct PlaceOrderRequest {
    /// User ID
    pub user_id: Uuid,
    /// Market
    pub market: String,
    /// Side
    pub side: Side,
    /// Order type
    pub order_type: OrderType,
    /// Price (for limit orders)
    pub price: Option<common::decimal::Price>,
    /// Quantity
    pub quantity: common::decimal::Quantity,
    /// Time in force
    #[serde(default = "default_time_in_force")]
    pub time_in_force: TimeInForce,
}

fn default_time_in_force() -> TimeInForce {
    TimeInForce::GTC
}

/// Order placement result
#[derive(Debug, Serialize)]
pub struct OrderPlacementResult {
    /// The placed order
    pub order: Order,
    /// Trades that were generated
    pub trades: Vec<Trade>,
}

/// Place a new order
pub async fn place_order(
    State(state): State<Arc<AppState>>,
    Json(request): Json<PlaceOrderRequest>,
) -> Result<ApiResponse<OrderPlacementResult>, ApiError> {
    // Create order from request
    let order = match request.order_type {
        OrderType::Limit => {
            let price = request.price.ok_or_else(|| {
                ApiError::BadRequest("Limit orders must have a price".to_string())
            })?;
            
            Order::new_limit(
                request.user_id,
                request.market,
                request.side,
                price,
                request.quantity,
                request.time_in_force,
            )
        },
        OrderType::Market => {
            Order::new_market(
                request.user_id,
                request.market,
                request.side,
                request.quantity,
            )
        },
    };
    
    // Reserve funds for the order
    state.account_service.reserve_for_order(&order).await
        .map_err(ApiError::Common)?;
    
    // Place the order
    let result = state.matching_engine.place_order(order.clone())
        .map_err(ApiError::Common)?;
    
    // Process trades
    for trade in &result.trades {
        state.account_service.process_trade(trade).await
            .map_err(ApiError::Common)?;
        
        state.market_data_service.process_trade(trade)
            .await
            .map_err(ApiError::Common)?;
    }
    
    // Update order book
    let market = order.market.clone();
    if let Ok((bids, asks)) = state.matching_engine.get_market_depth(&market, 10) {
        state.market_data_service.update_order_book(&market, bids, asks)
            .await
            .map_err(ApiError::Common)?;
    }
    
    // Create placement result
    let placement_result = OrderPlacementResult {
        order: result.taker_order.map(|o| o.as_ref().clone()).unwrap_or(order),
        trades: result.trades,
    };
    
    // Return standardized response
    Ok(ApiResponse::new(placement_result))
}

/// Cancel an order
pub async fn cancel_order(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
) -> Result<ApiResponse<Order>, ApiError> {
    // Add logging for debugging
    tracing::info!("Attempting to cancel order: {}", id);
    
    // Cancel the order
    let order = state.matching_engine.cancel_order(id)
        .map_err(ApiError::Common)?;
    
    // Release reserved funds
    state.account_service.release_reserved_funds(&order).await
        .map_err(ApiError::Common)?;
    
    // Update order book
    if let Ok((bids, asks)) = state.matching_engine.get_market_depth(&order.market, 10) {
        state.market_data_service.update_order_book(&order.market, bids, asks)
            .await
            .map_err(ApiError::Common)?;
    }
    
    // Log success
    tracing::info!("Successfully canceled order: {}", id);
    
    // Return standardized response with the canceled order
    Ok(ApiResponse::new(order.as_ref().clone()))
}

/// Get an order by ID
pub async fn get_order(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
) -> Result<ApiResponse<Order>, ApiError> {
    // Get order from matching engine
    let order = state.matching_engine.get_order(id)
        .ok_or_else(|| ApiError::NotFound(format!("Order not found: {}", id)))?;
    
    // Return standardized response with the order
    Ok(ApiResponse::new(order.as_ref().clone()))
}

/// Orders query parameters
#[derive(Debug, Deserialize)]
pub struct OrdersQuery {
    /// Market
    #[allow(dead_code)]
    pub market: Option<String>,
    /// Limit    
    #[serde(default = "default_orders_limit")]
    #[allow(dead_code)]
    pub limit: usize,
}

fn default_orders_limit() -> usize {
    100
}

/// Get orders for a user
pub async fn get_orders(
    State(_state): State<Arc<AppState>>,
    Path(_user_id): Path<Uuid>,
    Query(_query): Query<OrdersQuery>,
) -> Result<ApiListResponse<Order>, ApiError> {
    // TODO: Implement get orders by user ID and market
    // This is just a placeholder for MVP
    
    // Return empty list with standardized response format
    Ok(ApiListResponse::new(Vec::new()))
}