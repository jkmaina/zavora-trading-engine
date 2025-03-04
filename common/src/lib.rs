//! Common types and utilities for the trading engine
//!
//! This library contains shared types, utilities, and abstractions used across
//! all microservices in the trading platform. It provides a unified approach to
//! error handling, database access, and domain models.

pub mod error;
pub mod model;
pub mod decimal;
pub mod db;

/// Re-export important types
pub use error::{Error, Result, ErrorExt, IntoError};
pub use decimal::*;

// Re-export database types
pub use db::transaction::{DBTransaction, TransactionManager};

// Re-export utoipa for use in model ToSchema derives
#[cfg(feature = "utoipa")]
pub use utoipa;
