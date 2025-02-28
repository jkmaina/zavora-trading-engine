//! Order API handlers

use std::sync::Arc;

use axum::{
    extract::{Path, Query, State},
    Json,
};
use common::model::order::{Order, OrderType, Side, TimeInForce};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::ApiError;
use crate::AppState;

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

/// Place order response
#[derive(Debug, Serialize)]
pub struct PlaceOrderResponse {
    /// Order
    pub order: Order,
    /// Trades
    pub trades: Vec<common::model::trade::Trade>,
}

/// Place a new order
pub async fn place_order(
    State(state): State<Arc<AppState>>,
    Json(request): Json<PlaceOrderRequest>,
) -> Result<Json<PlaceOrderResponse>, ApiError> {
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
    
    // Return response
    Ok(Json(PlaceOrderResponse {
        order: result.taker_order.map(|o| o.as_ref().clone()).unwrap_or(order),
        trades: result.trades,
    }))
}

/// Cancel order response
#[derive(Debug, Serialize)]
pub struct CancelOrderResponse {
    /// Order
    pub order: Order,
}

/// Cancel an order
pub async fn cancel_order(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
) -> Result<Json<CancelOrderResponse>, ApiError> {
    // Add logging for debugging
    tracing::info!("Attempting to cancel order: {}", id);
    
    // Cancel the order with a timeout
    let order = state.matching_engine.cancel_order(id)
    .map_err(ApiError::Common)?;
    
    // Release reserved funds with a timeout
    state.account_service.release_reserved_funds(&order).await
        .map_err(ApiError::Common)?;
    
    // Update order book with a timeout
    if let Ok((bids, asks)) = state.matching_engine.get_market_depth(&order.market, 10) {
        state.market_data_service.update_order_book(&order.market, bids, asks)
            .await
            .map_err(ApiError::Common)?;
    }
    
    // Return response
    tracing::info!("Successfully canceled order: {}", id);
    Ok(Json(CancelOrderResponse {
        order: order.as_ref().clone(),
    }))
}

/// Get order response
#[derive(Debug, Serialize)]
pub struct GetOrderResponse {
    /// Order
    pub order: Order,
}

/// Get an order by ID
pub async fn get_order(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
) -> Result<Json<GetOrderResponse>, ApiError> {
    let order = state.matching_engine.get_order(id)
        .ok_or_else(|| ApiError::NotFound(format!("Order not found: {}", id)))?;
    
    Ok(Json(GetOrderResponse {
        order: order.as_ref().clone(),
    }))
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

/// Get orders response
#[derive(Debug, Serialize)]
pub struct GetOrdersResponse {
    /// Orders
    pub orders: Vec<Order>,
}

/// Get orders for a user
pub async fn get_orders(
    State(_state): State<Arc<AppState>>,
    Path(_user_id): Path<Uuid>,
    Query(_query): Query<OrdersQuery>,
) -> Result<Json<GetOrdersResponse>, ApiError> {
    // TODO: Implement get orders by user ID and market
    // This is just a placeholder for MVP
    Ok(Json(GetOrdersResponse {
        orders: Vec::new(),
    }))
}