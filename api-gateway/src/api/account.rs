//! Account API handlers

use std::sync::Arc;

use axum::{
    extract::{Path, State},
    Json,
};
use common::decimal::Quantity;
use common::model::account::{Account, Balance};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::ApiError;
use crate::AppState;

/// Create account request
#[derive(Debug, Deserialize)]
pub struct CreateAccountRequest {}

/// Create account response
#[derive(Debug, Serialize)]
pub struct CreateAccountResponse {
    /// Account
    pub account: Account,
}

/// Create a new account
pub async fn create_account(
    State(state): State<Arc<AppState>>,
    Json(_request): Json<CreateAccountRequest>,
) -> Result<Json<CreateAccountResponse>, ApiError> {
    let account = state.account_service.create_account();
    
    Ok(Json(CreateAccountResponse { account }))
}

/// Get account response
#[derive(Debug, Serialize)]
pub struct GetAccountResponse {
    /// Account
    pub account: Account,
}

/// Get an account by ID
pub async fn get_account(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
) -> Result<Json<GetAccountResponse>, ApiError> {
    let account = state.account_service.get_account(id)
        .ok_or_else(|| ApiError::NotFound(format!("Account not found: {}", id)))?;
    
    Ok(Json(GetAccountResponse { account }))
}

/// Get balances response
#[derive(Debug, Serialize)]
pub struct GetBalancesResponse {
    /// Balances
    pub balances: Vec<Balance>,
}

/// Get all balances for an account
pub async fn get_balances(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
) -> Result<Json<GetBalancesResponse>, ApiError> {
    let balances = state.account_service.get_balances(id);
    
    Ok(Json(GetBalancesResponse { balances }))
}

/// Deposit request
#[derive(Debug, Deserialize)]
pub struct DepositRequest {
    /// Asset
    pub asset: String,
    /// Amount
    pub amount: Quantity,
}

/// Deposit response
#[derive(Debug, Serialize)]
pub struct DepositResponse {
    /// Updated balance
    pub balance: Balance,
}

/// Deposit funds into an account
pub async fn deposit(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
    Json(request): Json<DepositRequest>,
) -> Result<Json<DepositResponse>, ApiError> {
    let balance = state.account_service.deposit(id, &request.asset, request.amount)
        .map_err(ApiError::Common)?;
    
    Ok(Json(DepositResponse { balance }))
}

/// Withdraw request
#[derive(Debug, Deserialize)]
pub struct WithdrawRequest {
    /// Asset
    pub asset: String,
    /// Amount
    pub amount: Quantity,
}

/// Withdraw response
#[derive(Debug, Serialize)]
pub struct WithdrawResponse {
    /// Updated balance
    pub balance: Balance,
}

/// Withdraw funds from an account
pub async fn withdraw(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
    Json(request): Json<WithdrawRequest>,
) -> Result<Json<WithdrawResponse>, ApiError> {
    let balance = state.account_service.withdraw(id, &request.asset, request.amount)
        .map_err(ApiError::Common)?;
    
    Ok(Json(WithdrawResponse { balance }))
}