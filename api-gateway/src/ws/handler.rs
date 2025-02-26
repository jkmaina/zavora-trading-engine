//! WebSocket handler implementation

use std::collections::HashSet;
use std::sync::Arc;

use axum::{
    extract::{State, WebSocketUpgrade},
    response::IntoResponse,
};
use futures::{SinkExt, StreamExt};
use market_data::channel::Topic;
use serde_json::json;
use tokio::sync::{mpsc, Mutex};
use tracing::{debug, error, info};
use uuid::Uuid;

use crate::AppState;
use crate::ws::message::{Subscription, WsError, WsNotification, WsRequest, WsResponse};

/// Handle WebSocket connection
pub async fn ws_handler(
    State(state): State<Arc<AppState>>,
    ws: WebSocketUpgrade,
) -> impl IntoResponse {
    ws.on_upgrade(|socket| handle_socket(socket, state))
}

/// Handle WebSocket connection
async fn handle_socket(
    socket: axum::extract::ws::WebSocket,
    state: Arc<AppState>,
) {
    // Client state
    let client_id = Uuid::new_v4();
    let subscriptions: Arc<Mutex<HashSet<Subscription>>> = Arc::new(Mutex::new(HashSet::new()));
    
    info!("New WebSocket connection: {}", client_id);

    // Get the market data channel
    let market_data_channel = state.market_data_service.channel();
    
    // Create a channel for sending messages to the client
    let (tx, mut rx) = mpsc::channel(100);
    
    // Split the WebSocket
    let (mut ws_sender, mut ws_receiver) = socket.split();
    
    // Spawn a task that forwards messages from the channel to the WebSocket
    let send_task = tokio::spawn(async move {
        while let Some(message) = rx.recv().await {
            if let Err(e) = ws_sender.send(axum::extract::ws::Message::Text(message)).await {
                error!("Error sending message: {}", e);
                break;
            }
        }
        
        // If the channel is closed or an error occurs, close the WebSocket
        let _ = ws_sender.close().await;
    });
    
    // Clone the sender for use in subscription handlers
    let tx_clone = tx.clone();
    
    // Handle incoming messages
    while let Some(result) = ws_receiver.next().await {
        match result {
            Ok(axum::extract::ws::Message::Text(text)) => {
                debug!("Received text message: {}", text);
                
                // Parse the message
                let request: WsRequest = match serde_json::from_str(&text) {
                    Ok(req) => req,
                    Err(e) => {
                        // Send error response
                        let response = WsResponse {
                            id: "0".to_string(),
                            result: None,
                            error: Some(WsError {
                                code: 400,
                                message: format!("Invalid request: {}", e),
                            }),
                        };
                        
                        if let Err(e) = tx.send(serde_json::to_string(&response).unwrap()).await {
                            error!("Error sending error response: {}", e);
                            break;
                        }
                        
                        continue;
                    }
                };
                
                // Handle the request
                match request.method.as_str() {
                    "subscribe" => {
                        // Extract channel and market
                        let channel = match request.params.get("channel") {
                            Some(serde_json::Value::String(channel)) => channel.clone(),
                            _ => {
                                // Send error response
                                let response = WsResponse {
                                    id: request.id,
                                    result: None,
                                    error: Some(WsError {
                                        code: 400,
                                        message: "Missing or invalid channel parameter".to_string(),
                                    }),
                                };
                                
                                if let Err(e) = tx.send(serde_json::to_string(&response).unwrap()).await {
                                    error!("Error sending error response: {}", e);
                                    break;
                                }
                                
                                continue;
                            }
                        };
                        
                        let market = request.params.get("market").and_then(|m| {
                            if let serde_json::Value::String(market) = m {
                                Some(market.clone())
                            } else {
                                None
                            }
                        });
                        
                        // Create subscription
                        let subscription_id = Uuid::new_v4();
                        let subscription = Subscription {
                            channel: channel.clone(),
                            market: market.clone(),
                            id: subscription_id,
                        };
                        
                        // Map to topic
                        let topic = match (channel.as_str(), market.clone()) {
                            ("orderbook", Some(market)) => Topic::OrderBook(market),
                            ("trades", Some(market)) => Topic::Trades(market),
                            ("ticker", Some(market)) => Topic::Ticker(market),
                            ("orderbook", None) => Topic::AllOrderBooks,
                            ("trades", None) => Topic::AllTrades,
                            ("ticker", None) => Topic::AllTickers,
                            _ => {
                                // Send error response
                                let response = WsResponse {
                                    id: request.id,
                                    result: None,
                                    error: Some(WsError {
                                        code: 400,
                                        message: format!("Invalid channel: {}", channel),
                                    }),
                                };
                                
                                if let Err(e) = tx.send(serde_json::to_string(&response).unwrap()).await {
                                    error!("Error sending error response: {}", e);
                                    break;
                                }
                                
                                continue;
                            }
                        };
                        
                        // Subscribe to the topic
                        let receiver = market_data_channel.subscribe::<serde_json::Value>(topic.clone()).await;
                        
                        // Set up subscription handler in a separate task
                        let sub_tx = tx_clone.clone();
                        let topic_clone = topic.clone();
                        let market_clone = market.clone();
                        
                        tokio::spawn(async move {
                            while let Ok(message) = receiver.recv() {
                                if let Some(any_ref) = message.downcast_ref::<serde_json::Value>() {
                                    // Create notification based on topic
                                    let notification = match topic_clone {
                                        Topic::OrderBook(_) => WsNotification {
                                            method: "orderbook".to_string(),
                                            params: json!({
                                                "market": market_clone,
                                                "data": any_ref,
                                                "subscription_id": subscription_id.to_string(),
                                            }),
                                        },
                                        Topic::Trades(_) => WsNotification {
                                            method: "trades".to_string(),
                                            params: json!({
                                                "market": market_clone,
                                                "data": any_ref,
                                                "subscription_id": subscription_id.to_string(),
                                            }),
                                        },
                                        Topic::Ticker(_) => WsNotification {
                                            method: "ticker".to_string(),
                                            params: json!({
                                                "market": market_clone,
                                                "data": any_ref,
                                                "subscription_id": subscription_id.to_string(),
                                            }),
                                        },
                                        _ => WsNotification {
                                            method: "update".to_string(),
                                            params: json!({
                                                "data": any_ref,
                                                "subscription_id": subscription_id.to_string(),
                                            }),
                                        },
                                    };
                                    
                                    // Send notification
                                    if let Err(e) = sub_tx.send(
                                        serde_json::to_string(&notification).unwrap()
                                    ).await {
                                        error!("Error sending notification: {}", e);
                                        break;
                                    }
                                }
                            }
                            
                            debug!("Subscription handler for {} exited", subscription_id);
                        });
                        
                        // Store subscription
                        {
                            let mut subs = subscriptions.lock().await;
                            subs.insert(subscription.clone());
                        }
                        
                        // Send success response
                        let response = WsResponse {
                            id: request.id,
                            result: Some(json!({
                                "subscriptionId": subscription_id,
                                "channel": channel,
                                "market": market,
                            })),
                            error: None,
                        };
                        
                        if let Err(e) = tx.send(serde_json::to_string(&response).unwrap()).await {
                            error!("Error sending success response: {}", e);
                            break;
                        }
                    },
                    "unsubscribe" => {
                        // Extract subscription ID
                        let subscription_id = match request.params.get("subscriptionId") {
                            Some(serde_json::Value::String(id)) => {
                                match Uuid::parse_str(id) {
                                    Ok(uuid) => uuid,
                                    Err(_) => {
                                        // Send error response
                                        let response = WsResponse {
                                            id: request.id,
                                            result: None,
                                            error: Some(WsError {
                                                code: 400,
                                                message: "Invalid subscription ID".to_string(),
                                            }),
                                        };
                                        
                                        if let Err(e) = tx.send(serde_json::to_string(&response).unwrap()).await {
                                            error!("Error sending error response: {}", e);
                                            break;
                                        }
                                        
                                        continue;
                                    }
                                }
                            },
                            _ => {
                                // Send error response
                                let response = WsResponse {
                                    id: request.id,
                                    result: None,
                                    error: Some(WsError {
                                        code: 400,
                                        message: "Missing or invalid subscriptionId parameter".to_string(),
                                    }),
                                };
                                
                                if let Err(e) = tx.send(serde_json::to_string(&response).unwrap()).await {
                                    error!("Error sending error response: {}", e);
                                    break;
                                }
                                
                                continue;
                            }
                        };
                        
                        // Find subscription details
                        let found_subscription = {
                            let subs = subscriptions.lock().await;
                            subs.iter()
                                .find(|s| s.id == subscription_id)
                                .cloned()
                        };
                        
                        match found_subscription {
                            Some(subscription) => {
                                // Remove subscription from our tracking
                                {
                                    let mut subs = subscriptions.lock().await;
                                    subs.remove(&subscription);
                                }
                                
                                // The unsubscribe operation doesn't need to tell the market_data_channel
                                // because the receiver will be dropped when the subscription handler task completes
                                
                                // Send success response
                                let response = WsResponse {
                                    id: request.id,
                                    result: Some(json!({
                                        "unsubscribed": true,
                                    })),
                                    error: None,
                                };
                                
                                if let Err(e) = tx.send(serde_json::to_string(&response).unwrap()).await {
                                    error!("Error sending success response: {}", e);
                                    break;
                                }
                            },
                            None => {
                                // Send error response
                                let response = WsResponse {
                                    id: request.id,
                                    result: None,
                                    error: Some(WsError {
                                        code: 404,
                                        message: "Subscription not found".to_string(),
                                    }),
                                };
                                
                                if let Err(e) = tx.send(serde_json::to_string(&response).unwrap()).await {
                                    error!("Error sending error response: {}", e);
                                    break;
                                }
                            }
                        }
                    },
                    "getOrderBook" => {
                        // Extract market
                        let market = match request.params.get("market") {
                            Some(serde_json::Value::String(market)) => market.clone(),
                            _ => {
                                // Send error response
                                let response = WsResponse {
                                    id: request.id,
                                    result: None,
                                    error: Some(WsError {
                                        code: 400,
                                        message: "Missing or invalid market parameter".to_string(),
                                    }),
                                };
                                
                                if let Err(e) = tx.send(serde_json::to_string(&response).unwrap()).await {
                                    error!("Error sending error response: {}", e);
                                    break;
                                }
                                
                                continue;
                            }
                        };
                        
                        // Get depth
                        let depth = request.params.get("depth")
                            .and_then(|d| d.as_u64())
                            .unwrap_or(10) as usize;
                        
                        // Get order book data
                        match state.matching_engine.get_market_depth(&market, depth) {
                            Ok((bids, asks)) => {
                                // Convert to JSON-friendly format
                                let bids_json: Vec<Vec<String>> = bids.iter()
                                    .map(|(price, quantity)| vec![
                                        price.to_string(),
                                        quantity.to_string(),
                                    ])
                                    .collect();
                                
                                let asks_json: Vec<Vec<String>> = asks.iter()
                                    .map(|(price, quantity)| vec![
                                        price.to_string(),
                                        quantity.to_string(),
                                    ])
                                    .collect();
                                
                                // Send response
                                let response = WsResponse {
                                    id: request.id,
                                    result: Some(json!({
                                        "market": market,
                                        "bids": bids_json,
                                        "asks": asks_json,
                                        "timestamp": chrono::Utc::now().to_rfc3339(),
                                    })),
                                    error: None,
                                };
                                
                                if let Err(e) = tx.send(serde_json::to_string(&response).unwrap()).await {
                                    error!("Error sending order book response: {}", e);
                                    break;
                                }
                            },
                            Err(e) => {
                                // Send error response
                                let response = WsResponse {
                                    id: request.id,
                                    result: None,
                                    error: Some(WsError {
                                        code: 500,
                                        message: format!("Error getting order book: {}", e),
                                    }),
                                };
                                
                                if let Err(e) = tx.send(serde_json::to_string(&response).unwrap()).await {
                                    error!("Error sending error response: {}", e);
                                    break;
                                }
                            }
                        }
                    },
                    "getTrades" => {
                        // Extract market
                        let market = match request.params.get("market") {
                            Some(serde_json::Value::String(market)) => market.clone(),
                            _ => {
                                // Send error response
                                let response = WsResponse {
                                    id: request.id,
                                    result: None,
                                    error: Some(WsError {
                                        code: 400,
                                        message: "Missing or invalid market parameter".to_string(),
                                    }),
                                };
                                
                                if let Err(e) = tx.send(serde_json::to_string(&response).unwrap()).await {
                                    error!("Error sending error response: {}", e);
                                    break;
                                }
                                
                                continue;
                            }
                        };
                        
                        // Get limit
                        let limit = request.params.get("limit")
                            .and_then(|l| l.as_u64())
                            .unwrap_or(100) as usize;
                        
                        // Get recent trades
                        let trades = state.market_data_service.get_recent_trades(&market, limit);
                        
                        // Send response
                        let response = WsResponse {
                            id: request.id,
                            result: Some(json!({
                                "market": market,
                                "trades": trades,
                            })),
                            error: None,
                        };
                        
                        if let Err(e) = tx.send(serde_json::to_string(&response).unwrap()).await {
                            error!("Error sending trades response: {}", e);
                            break;
                        }
                    },
                    "getTicker" => {
                        // Extract market
                        let market = match request.params.get("market") {
                            Some(serde_json::Value::String(market)) => market.clone(),
                            _ => {
                                // Send error response
                                let response = WsResponse {
                                    id: request.id,
                                    result: None,
                                    error: Some(WsError {
                                        code: 400,
                                        message: "Missing or invalid market parameter".to_string(),
                                    }),
                                };
                                
                                if let Err(e) = tx.send(serde_json::to_string(&response).unwrap()).await {
                                    error!("Error sending error response: {}", e);
                                    break;
                                }
                                
                                continue;
                            }
                        };
                        
                        // Get ticker
                        let ticker = state.market_data_service.get_ticker(&market);
                        
                        // Send response
                        let response = WsResponse {
                            id: request.id,
                            result: Some(json!({
                                "market": market,
                                "ticker": ticker,
                            })),
                            error: None,
                        };
                        
                        if let Err(e) = tx.send(serde_json::to_string(&response).unwrap()).await {
                            error!("Error sending ticker response: {}", e);
                            break;
                        }
                    },
                    "ping" => {
                        // Send pong response
                        let response = WsResponse {
                            id: request.id,
                            result: Some(json!({
                                "pong": chrono::Utc::now().to_rfc3339(),
                            })),
                            error: None,
                        };
                        
                        if let Err(e) = tx.send(serde_json::to_string(&response).unwrap()).await {
                            error!("Error sending pong response: {}", e);
                            break;
                        }
                    },
                    _ => {
                        // Send error for unknown method
                        let response = WsResponse {
                            id: request.id,
                            result: None,
                            error: Some(WsError {
                                code: 400,
                                message: format!("Unknown method: {}", request.method),
                            }),
                        };
                        
                        if let Err(e) = tx.send(serde_json::to_string(&response).unwrap()).await {
                            error!("Error sending error response: {}", e);
                            break;
                        }
                    }
                }
            },
            Ok(axum::extract::ws::Message::Ping(_bytes)) => {
                // Forward the pong through the dedicated sender task
                if let Err(e) = tx.send(serde_json::to_string(&"PONG").unwrap()).await {
                    error!("Error sending pong: {}", e);
                    break;
                }
            },
            Ok(axum::extract::ws::Message::Close(_)) => {
                debug!("Received close message");
                break;
            },
            Err(e) => {
                error!("Error receiving message: {}", e);
                break;
            },
            _ => {}
        }
    }
    
    // Connection closed, clean up
    info!("WebSocket connection closed: {}", client_id);
    
    // Cancel send task
    send_task.abort();
    
    // Clean up subscriptions
    {
        let mut subs = subscriptions.lock().await;
        subs.clear();
    }
}