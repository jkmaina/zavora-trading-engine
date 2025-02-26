//! Account service implementation

use std::sync::Arc;

use common::decimal::Quantity;
use common::error::{Error, Result};
use common::model::account::{Account, Balance};
use common::model::order::{Order, Side};
use common::model::trade::Trade;
use tracing::{debug, info};
use uuid::Uuid;

use crate::repository::InMemoryAccountRepository;

/// Account service for managing user balances and positions
pub struct AccountService {
    /// Repository for account data
    repo: Arc<InMemoryAccountRepository>,
}

impl AccountService {
    /// Create a new account service
    pub fn new() -> Self {
        Self {
            repo: Arc::new(InMemoryAccountRepository::new()),
        }
    }
    
    /// Create a new account
    pub fn create_account(&self) -> Account {
        info!("Creating new account");
        self.repo.create_account()
    }
    
    /// Get an account by ID
    pub fn get_account(&self, id: Uuid) -> Option<Account> {
        self.repo.get_account(id)
    }
    
    /// Get a balance
    pub fn get_balance(&self, account_id: Uuid, asset: &str) -> Option<Balance> {
        self.repo.get_balance(account_id, asset)
    }
    
    /// Get all balances for an account
    pub fn get_balances(&self, account_id: Uuid) -> Vec<Balance> {
        self.repo.get_balances(account_id)
    }
    
    /// Deposit funds into an account
    pub fn deposit(&self, account_id: Uuid, asset: &str, amount: Quantity) -> Result<Balance> {
        info!("Depositing {} {} to account {}", amount, asset, account_id);
        
        // Ensure the account exists
        let _account = self.repo.get_account(account_id).ok_or_else(|| {
            Error::Internal(format!("Account not found: {}", account_id))
        })?;
        
        // Get or create balance
        let mut balance = self.repo.ensure_balance(account_id, asset);
        
        // Update balance
        balance.deposit(amount);
        
        // Save and return
        self.repo.update_balance(balance)
    }
    
    /// Withdraw funds from an account
    pub fn withdraw(&self, account_id: Uuid, asset: &str, amount: Quantity) -> Result<Balance> {
        info!("Withdrawing {} {} from account {}", amount, asset, account_id);
        
        // Ensure the account exists
        let _account = self.repo.get_account(account_id).ok_or_else(|| {
            Error::Internal(format!("Account not found: {}", account_id))
        })?;
        
        // Get balance
        let mut balance = self.repo.get_balance(account_id, asset).ok_or_else(|| {
            Error::InsufficientBalance(format!("No balance found for {} in account {}", asset, account_id))
        })?;
        
        // Update balance
        balance.withdraw(amount).map_err(|e| {
            Error::InsufficientBalance(e)
        })?;
        
        // Save and return
        self.repo.update_balance(balance)
    }
    
    /// Reserve funds for an order
    pub fn reserve_for_order(&self, order: &Order) -> Result<()> {
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
        let mut balance = self.repo.get_balance(order.user_id, asset).ok_or_else(|| {
            Error::InsufficientBalance(format!("No balance found for {} in account {}", asset, order.user_id))
        })?;
        
        // Lock funds
        balance.lock(amount).map_err(|e| {
            Error::InsufficientBalance(e)
        })?;
        
        // Save balance
        self.repo.update_balance(balance)?;
        
        Ok(())
    }
    
    /// Release funds when an order is canceled
    pub fn release_reserved_funds(&self, order: &Order) -> Result<()> {
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
        let mut balance = self.repo.get_balance(order.user_id, asset).ok_or_else(|| {
            Error::Internal(format!("No balance found for {} in account {}", asset, order.user_id))
        })?;
        
        // Unlock funds
        balance.unlock(amount);
        
        // Save balance
        self.repo.update_balance(balance)?;
        
        Ok(())
    }
    
    /// Process a trade, updating balances for both parties
    pub fn process_trade(&self, trade: &Trade) -> Result<()> {
        debug!("Processing trade: {}", trade.id);
        
        // Market components
        let market_parts: Vec<&str> = trade.market.split('/').collect();
        if market_parts.len() != 2 {
            return Err(Error::Internal(format!("Invalid market format: {}", trade.market)));
        }
        
        let base_asset = market_parts[0];
        let quote_asset = market_parts[1];
        
        // Trade amount
        let base_amount = trade.quantity;
        let quote_amount = trade.price * trade.quantity;
        
        // Update buyer: add base asset, reduce locked quote asset
        let mut buyer_quote_balance = self.repo.get_balance(trade.buyer_id, quote_asset).ok_or_else(|| {
            Error::Internal(format!("No balance found for {} in account {}", quote_asset, trade.buyer_id))
        })?;
        
        let mut buyer_base_balance = self.repo.ensure_balance(trade.buyer_id, base_asset);
        
        // Buyer pays quote asset (which was already locked during order placement)
        buyer_quote_balance.locked -= quote_amount;
        buyer_quote_balance.total -= quote_amount;
        
        // Buyer receives base asset
        buyer_base_balance.total += base_amount;
        buyer_base_balance.available += base_amount;
        
        // Update seller: add quote asset, reduce locked base asset
        let mut seller_base_balance = self.repo.get_balance(trade.seller_id, base_asset).ok_or_else(|| {
            Error::Internal(format!("No balance found for {} in account {}", base_asset, trade.seller_id))
        })?;
        
        let mut seller_quote_balance = self.repo.ensure_balance(trade.seller_id, quote_asset);
        
        // Seller pays base asset (which was already locked during order placement)
        seller_base_balance.locked -= base_amount;
        seller_base_balance.total -= base_amount;
        
        // Seller receives quote asset
        seller_quote_balance.total += quote_amount;
        seller_quote_balance.available += quote_amount;
        
        // Save all balances
        self.repo.update_balance(buyer_quote_balance)?;
        self.repo.update_balance(buyer_base_balance)?;
        self.repo.update_balance(seller_base_balance)?;
        self.repo.update_balance(seller_quote_balance)?;
        
        Ok(())
    }
}