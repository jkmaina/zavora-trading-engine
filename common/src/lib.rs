//! Common types and utilities for the trading engine

pub mod error;
pub mod model;
pub mod decimal;

/// Re-export important types
pub use error::Error;
pub use decimal::*;
