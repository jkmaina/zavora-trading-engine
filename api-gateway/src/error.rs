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
    /// Error code
    pub code: String,
    /// Error message
    pub message: String,
    /// Request ID for tracing
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_id: Option<String>,
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
        let status = match &self {
            ApiError::NotFound(_) => StatusCode::NOT_FOUND,
            ApiError::BadRequest(_) => StatusCode::BAD_REQUEST,
            ApiError::Unauthorized(_) => StatusCode::UNAUTHORIZED,
            ApiError::Forbidden(_) => StatusCode::FORBIDDEN,
            ApiError::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
            ApiError::Common(e) => match e {
                common::error::Error::InvalidOrder(_) => StatusCode::BAD_REQUEST,
                common::error::Error::InsufficientBalance(_) => StatusCode::BAD_REQUEST,
                common::error::Error::OrderNotFound(_) => StatusCode::NOT_FOUND,
                common::error::Error::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
                common::error::Error::Database(_) => StatusCode::INTERNAL_SERVER_ERROR,
                common::error::Error::Serialization(_) => StatusCode::INTERNAL_SERVER_ERROR,
                common::error::Error::Migration(_) => StatusCode::INTERNAL_SERVER_ERROR
            },
        };
        
        let code = match &self {
            ApiError::NotFound(_) => "not_found",
            ApiError::BadRequest(_) => "bad_request",
            ApiError::Unauthorized(_) => "unauthorized",
            ApiError::Forbidden(_) => "forbidden",
            ApiError::Internal(_) => "internal_error",
            ApiError::Common(e) => match e {
                common::error::Error::InvalidOrder(_) => "invalid_order",
                common::error::Error::InsufficientBalance(_) => "insufficient_balance",
                common::error::Error::OrderNotFound(_) => "order_not_found",
                common::error::Error::Internal(_) => "internal_error",
                common::error::Error::Database(_) => "database_error",
                common::error::Error::Serialization(_) => "serialization_error",
                common::error::Error::Migration(_) => "migration_error"
            },
        };
        
        let error_response = ErrorResponse {
            code: code.to_string(),
            message: self.to_string(),
            request_id: Some(Uuid::new_v4().to_string()),
        };
        
        (status, Json(error_response)).into_response()
    }
}