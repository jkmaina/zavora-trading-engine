//! Account API handlers
//!
//! Handles endpoints related to account management:
//! - Create account
//! - Get account details
//! - Get account balances
//! - Deposit and withdraw funds

use std::sync::Arc;

use axum::{
    extract::{Path, State},
    Json,
};
use common::decimal::Quantity;
use common::model::account::{Account, Balance};
use serde::Deserialize;
use uuid::Uuid;

use crate::error::ApiError;
use crate::AppState;
use crate::api::response::{ApiResponse, ApiListResponse};

/// Create account request
#[derive(Debug, Deserialize)]
pub struct CreateAccountRequest {}

/// Create a new account
pub async fn create_account(
    State(state): State<Arc<AppState>>,
    Json(_request): Json<CreateAccountRequest>,
) -> Result<ApiResponse<Account>, ApiError> {
    let account = state.account_service.create_account().await
        .map_err(ApiError::Common)?;
    
    // Create a standardized response
    let response = ApiResponse::new(account);
    Ok(response)
}

/// Get an account by ID
pub async fn get_account(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
) -> Result<ApiResponse<Account>, ApiError> {
    // Request the account from the service
    let account = state.account_service.get_account(id).await
        .map_err(ApiError::Common)?
        .ok_or_else(|| ApiError::NotFound(format!("Account not found: {}", id)))?;
    
    // Return a standardized response
    Ok(ApiResponse::new(account))
}

/// Get all balances for an account
pub async fn get_balances(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
) -> Result<ApiListResponse<Balance>, ApiError> {
    // Verify the account exists before fetching balances
    let _ = state.account_service.get_account(id).await
        .map_err(ApiError::Common)?
        .ok_or_else(|| ApiError::NotFound(format!("Account not found: {}", id)))?;

    // Get balances from the service
    let balances = state.account_service.get_balances(id).await
        .map_err(ApiError::Common)?;
    
    // Return a standardized list response
    Ok(ApiListResponse::new(balances))
}

/// Deposit request
#[derive(Debug, Deserialize)]
pub struct DepositRequest {
    /// Asset
    pub asset: String,
    /// Amount
    pub amount: Quantity,
}

/// Deposit funds into an account
pub async fn deposit(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
    Json(request): Json<DepositRequest>,
) -> Result<ApiResponse<Balance>, ApiError> {
    // Call the service to deposit funds
    let balance = state.account_service.deposit(id, &request.asset, request.amount).await
        .map_err(ApiError::Common)?;
    
    // Return a standardized response with the updated balance
    Ok(ApiResponse::new(balance))
}

/// Withdraw request
#[derive(Debug, Deserialize)]
pub struct WithdrawRequest {
    /// Asset
    pub asset: String,
    /// Amount
    pub amount: Quantity,
}

/// Withdraw funds from an account
pub async fn withdraw(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
    Json(request): Json<WithdrawRequest>,
) -> Result<ApiResponse<Balance>, ApiError> {
    // Call the service to withdraw funds
    let balance = state.account_service.withdraw(id, &request.asset, request.amount).await
        .map_err(ApiError::Common)?;
    
    // Return a standardized response with the updated balance
    Ok(ApiResponse::new(balance))
}