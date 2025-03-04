//! WebSocket messages

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// WebSocket request message
#[derive(Debug, Deserialize)]
pub struct WsRequest {
    /// Request ID
    pub id: String,
    /// Method
    pub method: String,
    /// Params
    pub params: serde_json::Value,
}

/// WebSocket response message
#[derive(Debug, Serialize)]
pub struct WsResponse {
    /// Request ID
    pub id: String,
    /// Result
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<serde_json::Value>,
    /// Error
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<WsError>,
}

/// WebSocket error
#[derive(Debug, Serialize)]
pub struct WsError {
    /// Error code
    pub code: i32,
    /// Error message
    pub message: String,
}

/// WebSocket notification message
#[derive(Debug, Serialize)]
pub struct WsNotification {
    /// Method
    pub method: String,
    /// Params
    pub params: serde_json::Value,
}

/// WebSocket subscription
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Subscription {
    /// Channel
    pub channel: String,
    /// Market
    pub market: Option<String>,
    /// Subscription ID
    pub id: Uuid,
}