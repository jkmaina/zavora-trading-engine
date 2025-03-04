//! Account service implementation

use std::sync::Arc;

use common::decimal::Quantity;
use common::error::{Error, Result, ErrorExt};
use common::model::account::{Account, Balance};
use common::model::order::{Order, Side};
use common::model::trade::Trade;
use tracing::{debug, info, error};
use uuid::Uuid;

use crate::repository::{AccountRepository, InMemoryAccountRepository, PostgresAccountRepository};

// Not used currently but might be useful in the future
#[allow(dead_code)]
type TransactionResult = std::result::Result<(), Error>;

/// Account service for managing user balances and positions
pub struct AccountService {
    /// Repository for account data
    repo: Arc<dyn AccountRepository>,
}

/// Repository Type
pub enum RepositoryType {
    /// In-memory repository
    InMemory,
    /// PostgreSQL repository
    Postgres(Option<String>),
}

impl AccountService {
    /// Create a new account service
    pub fn new() -> Self {
        Self {
            repo: Arc::new(InMemoryAccountRepository::new()),
        }
    }
    
    /// Create a new account service with a specific repository type
    pub async fn with_repository(repo_type: RepositoryType) -> Result<Self> {
        let repo: Arc<dyn AccountRepository> = match repo_type {
            RepositoryType::InMemory => {
                Arc::new(InMemoryAccountRepository::new())
            },
            RepositoryType::Postgres(database_url) => {
                Arc::new(PostgresAccountRepository::new(database_url).await?)
            }
        };
        
        Ok(Self { repo })
    }
    
    /// Create a new account service with a configuration
    pub async fn with_config(config: &crate::config::AccountServiceConfig) -> Result<Self> {
        let repo: Arc<dyn AccountRepository> = Arc::new(
            PostgresAccountRepository::with_config(config).await?
        );
        
        Ok(Self { repo })
    }
    
    /// Create a new account
    pub async fn create_account(&self) -> Result<Account> {
        info!("Creating new account");
        self.repo.create_account().await
    }
    
    /// Get an account by ID
    pub async fn get_account(&self, id: Uuid) -> Result<Option<Account>> {
        self.repo.get_account(id).await
    }
    
    /// Get a balance
    pub async fn get_balance(&self, account_id: Uuid, asset: &str) -> Result<Option<Balance>> {
        self.repo.get_balance(account_id, asset).await
    }
    
    /// Get all balances for an account
    pub async fn get_balances(&self, account_id: Uuid) -> Result<Vec<Balance>> {
        self.repo.get_balances(account_id).await
    }
    
    /// Deposit funds into an account
    pub async fn deposit(&self, account_id: Uuid, asset: &str, amount: Quantity) -> Result<Balance> {
        info!("Depositing {} {} to account {}", amount, asset, account_id);
        
        // Ensure the account exists
        let _account = self.repo.get_account(account_id).await
            .with_context(|| format!("Failed to retrieve account {}", account_id))?
            .ok_or_else(|| Error::AccountNotFound(format!("Account not found: {}", account_id)))?;
        
        // Get or create balance
        let mut balance = self.repo.ensure_balance(account_id, asset).await
            .with_context(|| format!("Failed to ensure balance for account {}, asset {}", account_id, asset))?;
        
        // Update balance
        balance.deposit(amount);
        
        // Save and return
        self.repo.update_balance(balance).await
            .with_context(|| format!("Failed to update balance after deposit for account {}, asset {}", account_id, asset))
    }
    
    /// Withdraw funds from an account
    pub async fn withdraw(&self, account_id: Uuid, asset: &str, amount: Quantity) -> Result<Balance> {
        info!("Withdrawing {} {} from account {}", amount, asset, account_id);
        
        // Ensure the account exists
        let _account = self.repo.get_account(account_id).await
            .with_context(|| format!("Failed to retrieve account {}", account_id))?
            .ok_or_else(|| Error::AccountNotFound(format!("Account not found: {}", account_id)))?;
        
        // Get balance
        let mut balance = self.repo.get_balance(account_id, asset).await
            .with_context(|| format!("Failed to retrieve balance for account {}, asset {}", account_id, asset))?
            .ok_or_else(|| Error::InsufficientBalance(format!("No balance found for {} in account {}", asset, account_id)))?;
        
        // Update balance
        balance.withdraw(amount).map_err(|e| {
            Error::InsufficientBalance(format!("Cannot withdraw {} {}: {}", amount, asset, e))
        })?;
        
        // Save and return
        self.repo.update_balance(balance).await
            .with_context(|| format!("Failed to update balance after withdrawal for account {}, asset {}", account_id, asset))
    }
    
    /// Reserve funds for an order
    pub async fn reserve_for_order(&self, order: &Order) -> Result<()> {
        // For buy orders, we need to lock quote currency
        // For sell orders, we need to lock base currency
        let (asset, amount) = match order.side {
            Side::Buy => {
                let market_parts: Vec<&str> = order.market.split('/').collect();
                if market_parts.len() != 2 {
                    return Err(Error::Internal(format!("Invalid market format: {}", order.market)));
                }
                
                let quote_asset = market_parts[1];
                let price = order.price.ok_or_else(|| {
                    Error::InvalidOrder("Buy limit order must have a price".to_string())
                })?;
                
                (quote_asset, price * order.quantity)
            },
            Side::Sell => {
                let market_parts: Vec<&str> = order.market.split('/').collect();
                if market_parts.len() != 2 {
                    return Err(Error::Internal(format!("Invalid market format: {}", order.market)));
                }
                
                let base_asset = market_parts[0];
                (base_asset, order.quantity)
            }
        };
        
        debug!("Reserving {} {} for order {}", amount, asset, order.id);
        
        // Get balance
        let mut balance = self.repo.get_balance(order.user_id, asset).await?
            .ok_or_else(|| Error::InsufficientBalance(format!("No balance found for {} in account {}", asset, order.user_id)))?;
        
        // Lock funds
        balance.lock(amount).map_err(|e| {
            Error::InsufficientBalance(e)
        })?;
        
        // Save balance
        self.repo.update_balance(balance).await?;
        
        Ok(())
    }
    
    /// Release funds when an order is canceled
    pub async fn release_reserved_funds(&self, order: &Order) -> Result<()> {
        // Calculate remaining locked amount
        let (asset, amount) = match order.side {
            Side::Buy => {
                let market_parts: Vec<&str> = order.market.split('/').collect();
                if market_parts.len() != 2 {
                    return Err(Error::Internal(format!("Invalid market format: {}", order.market)));
                }
                
                let quote_asset = market_parts[1];
                let price = order.price.ok_or_else(|| {
                    Error::InvalidOrder("Buy limit order must have a price".to_string())
                })?;
                
                (quote_asset, price * order.remaining_quantity)
            },
            Side::Sell => {
                let market_parts: Vec<&str> = order.market.split('/').collect();
                if market_parts.len() != 2 {
                    return Err(Error::Internal(format!("Invalid market format: {}", order.market)));
                }
                
                let base_asset = market_parts[0];
                (base_asset, order.remaining_quantity)
            }
        };
        
        debug!("Releasing {} {} for canceled order {}", amount, asset, order.id);
        
        // Get balance
        let mut balance = self.repo.get_balance(order.user_id, asset).await?
            .ok_or_else(|| Error::Internal(format!("No balance found for {} in account {}", asset, order.user_id)))?;
        
        // Unlock funds
        balance.unlock(amount);
        
        // Save balance
        self.repo.update_balance(balance).await?;
        
        Ok(())
    }
    
    /// Process a trade, updating balances for both parties with database transaction
    pub async fn process_trade(&self, trade: &Trade) -> Result<()> {
        debug!("Processing trade: {}", trade.id);
        
        // Market components
        let market_parts: Vec<&str> = trade.market.split('/').collect();
        if market_parts.len() != 2 {
            return Err(Error::ValidationError(format!("Invalid market format: {}", trade.market)));
        }
        
        let base_asset = market_parts[0];
        let quote_asset = market_parts[1];
        
        // Trade amount
        let base_amount = trade.quantity;
        let quote_amount = trade.price * trade.quantity;
        
        // Start a database transaction
        let transaction = self.repo.begin_transaction().await
            .with_context(|| format!("Failed to start transaction for trade {}", trade.id))?;
        
        // Use a closure for the transaction work to handle errors consistently
        let transaction_result = async {
            // Get all balances first to avoid deadlocks
            let buyer_quote_balance_result = self.repo.get_balance(trade.buyer_id, quote_asset).await
                .with_context(|| format!("Failed to get buyer's quote balance ({}) for trade {}", quote_asset, trade.id))?;
                
            let buyer_base_balance_result = self.repo.get_balance(trade.buyer_id, base_asset).await
                .with_context(|| format!("Failed to get buyer's base balance ({}) for trade {}", base_asset, trade.id))?;
                
            let seller_base_balance_result = self.repo.get_balance(trade.seller_id, base_asset).await
                .with_context(|| format!("Failed to get seller's base balance ({}) for trade {}", base_asset, trade.id))?;
                
            let seller_quote_balance_result = self.repo.get_balance(trade.seller_id, quote_asset).await
                .with_context(|| format!("Failed to get seller's quote balance ({}) for trade {}", quote_asset, trade.id))?;
            
            // Validate and prepare balances
            let mut buyer_quote_balance = buyer_quote_balance_result
                .ok_or_else(|| Error::InsufficientBalance(
                    format!("No {} balance found for buyer {}", quote_asset, trade.buyer_id)
                ))?;
            
            let mut buyer_base_balance = match buyer_base_balance_result {
                Some(balance) => balance,
                None => self.repo.ensure_balance(trade.buyer_id, base_asset).await
                    .with_context(|| format!("Failed to create base balance for buyer"))?,
            };
            
            let mut seller_base_balance = seller_base_balance_result
                .ok_or_else(|| Error::InsufficientBalance(
                    format!("No {} balance found for seller {}", base_asset, trade.seller_id)
                ))?;
            
            let mut seller_quote_balance = match seller_quote_balance_result {
                Some(balance) => balance,
                None => self.repo.ensure_balance(trade.seller_id, quote_asset).await
                    .with_context(|| format!("Failed to create quote balance for seller"))?,
            };
            
            // Validate locked funds
            if buyer_quote_balance.locked < quote_amount {
                return Err(Error::InsufficientBalance(format!(
                    "Buyer has insufficient locked funds: {} < {}", buyer_quote_balance.locked, quote_amount
                )));
            }
            
            if seller_base_balance.locked < base_amount {
                return Err(Error::InsufficientBalance(format!(
                    "Seller has insufficient locked funds: {} < {}", seller_base_balance.locked, base_amount
                )));
            }
            
            // Update buyer balances
            buyer_quote_balance.locked -= quote_amount;
            buyer_quote_balance.total -= quote_amount;
            buyer_base_balance.total += base_amount;
            buyer_base_balance.available += base_amount;
            
            // Update seller balances
            seller_base_balance.locked -= base_amount;
            seller_base_balance.total -= base_amount;
            seller_quote_balance.total += quote_amount;
            seller_quote_balance.available += quote_amount;
            
            // Update all balances
            self.repo.update_balance(buyer_quote_balance).await
                .with_context(|| "Failed to update buyer quote balance")?;
                
            self.repo.update_balance(buyer_base_balance).await
                .with_context(|| "Failed to update buyer base balance")?;
                
            self.repo.update_balance(seller_base_balance).await
                .with_context(|| "Failed to update seller base balance")?;
                
            self.repo.update_balance(seller_quote_balance).await
                .with_context(|| "Failed to update seller quote balance")?;
            
            Ok(())
        }.await;
        
        // Handle transaction result
        match transaction_result {
            Ok(_) => {
                // Commit the transaction
                transaction.commit().await
                    .with_context(|| format!("Failed to commit transaction for trade {}", trade.id))?;
                    
                info!("Successfully processed trade: {}", trade.id);
                Ok(())
            },
            Err(e) => {
                // Log the error and roll back
                error!("Error processing trade {}: {}", trade.id, e);
                
                // Roll back the transaction
                if let Err(rollback_err) = transaction.rollback().await {
                    // Log rollback failure but return the original error
                    error!("Failed to roll back transaction: {}", rollback_err);
                }
                
                // Return the original error
                Err(e)
            }
        }
    }
}