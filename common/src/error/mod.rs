//! Error types for the trading engine

use thiserror::Error;

/// Trading engine error type
#[derive(Debug, Error)]
pub enum Error {
    #[error("Invalid order: {0}")]
    InvalidOrder(String),
    
    #[error("Insufficient balance: {0}")]
    InsufficientBalance(String),
    
    #[error("Order not found: {0}")]
    OrderNotFound(String),
    
    #[error("Internal error: {0}")]
    Internal(String),
    
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),
    
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
}

/// Result type alias
pub type Result<T> = std::result::Result<T, Error>;
