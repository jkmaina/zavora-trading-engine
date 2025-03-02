//! Application configuration

use std::env;

/// Application configuration
#[allow(dead_code)]
pub struct AppConfig {
    /// API port
    pub port: u16,
    /// Database URL
    pub database_url: Option<String>,
    /// JWT secret
    pub jwt_secret: Option<String>,
}

impl AppConfig {
    /// Create a new configuration from environment variables
    pub fn new() -> Self {
        Self {
            port: env::var("PORT")
                .ok()
                .and_then(|p| p.parse().ok())
                .unwrap_or(8080),
            database_url: env::var("DATABASE_URL").ok(),
            jwt_secret: env::var("JWT_SECRET").ok(),
        }
    }
}