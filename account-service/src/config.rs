//! Configuration for the account service

use std::env;

/// Configuration for the account service
#[derive(Debug, Clone)]
pub struct AccountServiceConfig {
    /// Database URL
    pub database_url: String,
    /// Database connection pool size
    pub db_pool_size: u32,
    /// Enable transaction logging
    pub transaction_logging: bool,
}

impl Default for AccountServiceConfig {
    fn default() -> Self {
        Self {
            database_url: env::var("DATABASE_URL")
                .unwrap_or_else(|_| "postgres://postgres:postgres@localhost:5432/zavora".to_string()),
            db_pool_size: env::var("DB_POOL_SIZE")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(5),
            transaction_logging: env::var("TRANSACTION_LOGGING")
                .map(|v| v == "true" || v == "1")
                .unwrap_or(false),
        }
    }
}

impl AccountServiceConfig {
    /// Create a new configuration using environment variables
    pub fn from_env() -> Self {
        Self::default()
    }
    
    /// Create a new configuration with custom values
    pub fn new(database_url: String, db_pool_size: u32, transaction_logging: bool) -> Self {
        Self {
            database_url,
            db_pool_size,
            transaction_logging,
        }
    }
}