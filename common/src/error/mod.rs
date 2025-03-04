//! Error types for the trading engine
//!
//! This module provides a unified error handling system for all microservices
//! in the trading platform. It defines standard error types that can be used
//! across service boundaries and provides consistent error conversion.

use std::fmt::Display;
use thiserror::Error;

/// Trading engine error type
#[derive(Debug, Error)]
pub enum Error {
    /// Error related to order validation or processing
    #[error("Invalid order: {0}")]
    InvalidOrder(String),
    
    /// Error when an account has insufficient funds
    #[error("Insufficient balance: {0}")]
    InsufficientBalance(String),
    
    /// Error when an order cannot be found
    #[error("Order not found: {0}")]
    OrderNotFound(String),
    
    /// Error when a market cannot be found
    #[error("Market not found: {0}")]
    MarketNotFound(String),
    
    /// Error when an account cannot be found
    #[error("Account not found: {0}")]
    AccountNotFound(String),
    
    /// Generic validation error
    #[error("Validation error: {0}")]
    ValidationError(String),
    
    /// Configuration error
    #[error("Configuration error: {0}")]
    ConfigurationError(String),
    
    /// Authorization error
    #[error("Authorization error: {0}")]
    AuthorizationError(String),
    
    /// Rate limit error
    #[error("Rate limit exceeded: {0}")]
    RateLimitExceeded(String),
    
    /// Internal server error
    #[error("Internal error: {0}")]
    Internal(String),
    
    /// Database error
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),
    
    /// Database migration error
    #[error("Migration error: {0}")]
    Migration(#[from] sqlx::migrate::MigrateError),
    
    /// Serialization/deserialization error
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    
    /// Decimal conversion error
    #[error("Decimal conversion error: {0}")]
    DecimalError(String),
}

/// Result type alias
pub type Result<T> = std::result::Result<T, Error>;

/// Extension trait to add context to error results
pub trait ErrorExt<T> {
    /// Add context information to an error
    fn with_context<C, F>(self, context_fn: F) -> Result<T>
    where
        F: FnOnce() -> C,
        C: Display;
}

impl<T> ErrorExt<T> for Result<T> {
    fn with_context<C, F>(self, context_fn: F) -> Result<T>
    where
        F: FnOnce() -> C,
        C: Display,
    {
        self.map_err(|e| {
            let context = context_fn().to_string();
            match e {
                Error::Internal(msg) => Error::Internal(format!("{}: {}", context, msg)),
                Error::InvalidOrder(msg) => Error::InvalidOrder(format!("{}: {}", context, msg)),
                Error::InsufficientBalance(msg) => Error::InsufficientBalance(format!("{}: {}", context, msg)),
                Error::OrderNotFound(msg) => Error::OrderNotFound(format!("{}: {}", context, msg)),
                Error::MarketNotFound(msg) => Error::MarketNotFound(format!("{}: {}", context, msg)),
                Error::AccountNotFound(msg) => Error::AccountNotFound(format!("{}: {}", context, msg)),
                Error::ValidationError(msg) => Error::ValidationError(format!("{}: {}", context, msg)),
                Error::ConfigurationError(msg) => Error::ConfigurationError(format!("{}: {}", context, msg)),
                Error::AuthorizationError(msg) => Error::AuthorizationError(format!("{}: {}", context, msg)),
                Error::RateLimitExceeded(msg) => Error::RateLimitExceeded(format!("{}: {}", context, msg)),
                Error::Database(e) => Error::Database(e),
                Error::Migration(e) => Error::Migration(e),
                Error::Serialization(e) => Error::Serialization(e),
                Error::DecimalError(msg) => Error::DecimalError(format!("{}: {}", context, msg)),
            }
        })
    }
}

/// Trait for converting other error types to our Error type
pub trait IntoError {
    /// Convert to Error
    fn into_error(self, message: &str) -> Error;
}

impl<E: std::error::Error> IntoError for E {
    fn into_error(self, message: &str) -> Error {
        Error::Internal(format!("{}: {}", message, self))
    }
}

/// Convert string messages into an error
impl From<String> for Error {
    fn from(message: String) -> Self {
        Error::Internal(message)
    }
}

/// Convert static string references into an error
impl From<&str> for Error {
    fn from(message: &str) -> Self {
        Error::Internal(message.to_string())
    }
}

/// From rust_decimal::Error
impl From<rust_decimal::Error> for Error {
    fn from(err: rust_decimal::Error) -> Self {
        Error::DecimalError(err.to_string())
    }
}
