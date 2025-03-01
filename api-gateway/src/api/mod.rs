//! API handlers
//!
//! This module contains all the API endpoint handlers organized by resource.
//! Each handler follows a consistent pattern:
//! - Extract state and parameters using Axum extractors
//! - Validate input parameters
//! - Call the appropriate service methods
//! - Map the result to a standardized response format

pub mod account;
pub mod market;
pub mod order;
pub mod response;

// Re-export the response module for easy access
pub use response::{ApiResponse, PaginatedResponse, ApiListResponse};