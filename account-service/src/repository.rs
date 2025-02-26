//! Repository for account data


use chrono::Utc;
use common::error::Result;
use common::model::account::{Account, Balance};
use dashmap::DashMap;
use uuid::Uuid;

/// In-memory repository for account data
pub struct InMemoryAccountRepository {
    /// Accounts by ID
    accounts: DashMap<Uuid, Account>,
    /// Balances by account ID and asset
    balances: DashMap<(Uuid, String), Balance>,
}

impl InMemoryAccountRepository {
    /// Create a new in-memory account repository
    pub fn new() -> Self {
        Self {
            accounts: DashMap::new(),
            balances: DashMap::new(),
        }
    }
    
    /// Create a new account
    pub fn create_account(&self) -> Account {
        let now = Utc::now();
        let account = Account {
            id: Uuid::new_v4(),
            created_at: now,
            updated_at: now,
        };
        
        self.accounts.insert(account.id, account.clone());
        account
    }
    
    /// Get an account by ID
    pub fn get_account(&self, id: Uuid) -> Option<Account> {
        self.accounts.get(&id).map(|a| a.clone())
    }
    
    /// Get a balance
    pub fn get_balance(&self, account_id: Uuid, asset: &str) -> Option<Balance> {
        self.balances.get(&(account_id, asset.to_string())).map(|b| b.clone())
    }
    
    /// Get all balances for an account
    pub fn get_balances(&self, account_id: Uuid) -> Vec<Balance> {
        self.balances
            .iter()
            .filter_map(|entry| {
                let ((acc_id, _), balance) = entry.pair();
                if *acc_id == account_id {
                    Some(balance.clone())
                } else {
                    None
                }
            })
            .collect()
    }
    
    /// Create or update a balance
    pub fn update_balance(&self, balance: Balance) -> Result<Balance> {
        let key = (balance.account_id, balance.asset.clone());
        self.balances.insert(key, balance.clone());
        Ok(balance)
    }
    
    /// Ensure a balance exists, creating it if necessary
    pub fn ensure_balance(&self, account_id: Uuid, asset: &str) -> Balance {
        let key = (account_id, asset.to_string());
        
        if let Some(balance) = self.balances.get(&key) {
            balance.clone()
        } else {
            let balance = Balance::new(account_id, asset.to_string());
            self.balances.insert(key, balance.clone());
            balance
        }
    }
}