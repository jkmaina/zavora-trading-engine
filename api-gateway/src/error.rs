//! Error handling for the API gateway

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// API error response
#[derive(Debug, Serialize, Deserialize)]
pub struct ErrorResponse {
    /// Error information
    pub error: ErrorInfo,
    /// Request ID for tracing
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_id: Option<String>,
}

/// Detailed error information
#[derive(Debug, Serialize, Deserialize)]
pub struct ErrorInfo {
    /// Error code (string identifier for the error type)
    pub code: String,
    /// Human-readable error message
    pub message: String,
    /// Optional additional error details
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<serde_json::Value>,
}

/// API errors
#[derive(Debug, thiserror::Error)]
#[allow(dead_code)]
pub enum ApiError {
    #[error("Not found: {0}")]
    NotFound(String),
    
    #[error("Invalid request: {0}")]
    BadRequest(String),
    
    #[error("Unauthorized: {0}")]
    Unauthorized(String),
    
    #[error("Forbidden: {0}")]
    Forbidden(String),
    
    #[error("Internal server error: {0}")]
    Internal(String),
    
    #[error("Common error: {0}")]
    Common(#[from] common::error::Error),
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        // Generate a request ID for tracking errors
        let request_id = Uuid::new_v4().to_string();
        
        // Log the error with request ID for backend tracing
        tracing::error!("API Error [{}]: {:?}", request_id, &self);
        
        let (status, code, details) = match &self {
            ApiError::NotFound(_) => (
                StatusCode::NOT_FOUND, 
                "not_found", 
                None
            ),
            ApiError::BadRequest(_) => (
                StatusCode::BAD_REQUEST, 
                "bad_request", 
                None
            ),
            ApiError::Unauthorized(_) => (
                StatusCode::UNAUTHORIZED, 
                "unauthorized", 
                None
            ),
            ApiError::Forbidden(_) => (
                StatusCode::FORBIDDEN, 
                "forbidden", 
                None
            ),
            ApiError::Internal(_) => (
                StatusCode::INTERNAL_SERVER_ERROR, 
                "internal_error", 
                None
            ),
            ApiError::Common(e) => match e {
                // Client errors (4xx)
                common::error::Error::InvalidOrder(_) => (
                    StatusCode::BAD_REQUEST, 
                    "invalid_order", 
                    None
                ),
                common::error::Error::InsufficientBalance(_) => (
                    StatusCode::BAD_REQUEST, 
                    "insufficient_balance", 
                    None
                ),
                common::error::Error::OrderNotFound(_) => (
                    StatusCode::NOT_FOUND, 
                    "order_not_found", 
                    None
                ),
                common::error::Error::MarketNotFound(_) => (
                    StatusCode::NOT_FOUND, 
                    "market_not_found", 
                    None
                ),
                common::error::Error::AccountNotFound(_) => (
                    StatusCode::NOT_FOUND, 
                    "account_not_found", 
                    None
                ),
                common::error::Error::ValidationError(_) => (
                    StatusCode::BAD_REQUEST, 
                    "validation_error", 
                    None
                ),
                common::error::Error::AuthorizationError(_) => (
                    StatusCode::FORBIDDEN, 
                    "authorization_error", 
                    None
                ),
                common::error::Error::RateLimitExceeded(_) => (
                    StatusCode::TOO_MANY_REQUESTS, 
                    "rate_limit_exceeded", 
                    None
                ),
                
                // Server errors (5xx)
                common::error::Error::ConfigurationError(_) => (
                    StatusCode::INTERNAL_SERVER_ERROR, 
                    "configuration_error", 
                    None
                ),
                common::error::Error::Internal(_) => (
                    StatusCode::INTERNAL_SERVER_ERROR, 
                    "internal_error", 
                    None
                ),
                common::error::Error::Database(e) => (
                    StatusCode::INTERNAL_SERVER_ERROR, 
                    "database_error", 
                    Some(serde_json::json!({
                        "db_error": e.to_string(),
                        "code": e.as_database_error().map(|dbe| dbe.code().map(|c| c.to_string())),
                    }))
                ),
                common::error::Error::Migration(_) => (
                    StatusCode::INTERNAL_SERVER_ERROR, 
                    "migration_error", 
                    None
                ),
                common::error::Error::Serialization(_) => (
                    StatusCode::INTERNAL_SERVER_ERROR, 
                    "serialization_error", 
                    None
                ),
                common::error::Error::DecimalError(_) => (
                    StatusCode::INTERNAL_SERVER_ERROR, 
                    "decimal_error", 
                    None
                ),
            },
        };
        
        // Create the error response with the new structure
        let error_response = ErrorResponse {
            error: ErrorInfo {
                code: code.to_string(),
                message: self.to_string(),
                details,
            },
            request_id: Some(request_id),
        };
        
        // Return the response with appropriate status code
        (status, Json(error_response)).into_response()
    }
}