//! Repository for account data

use async_trait::async_trait;
use chrono::Utc;
use common::decimal::Quantity;
use common::error::{Error, Result};
use common::model::account::{Account, Balance};
use common::{DBTransaction, TransactionManager};
use common::db::{PgTransactionManager, InMemoryTransactionManager};
use dashmap::DashMap;
use sqlx::{PgPool, postgres::PgPoolOptions, Row};
use tracing::{debug, info};
use uuid::Uuid;

/// Account repository trait defining the interface for account data storage
#[async_trait]
pub trait AccountRepository: Send + Sync {
    /// Get the transaction manager
    fn transaction_manager(&self) -> &dyn TransactionManager;

    /// Create a new account
    async fn create_account(&self) -> Result<Account>;
    
    /// Get an account by ID
    async fn get_account(&self, id: Uuid) -> Result<Option<Account>>;
    
    /// Get a balance
    async fn get_balance(&self, account_id: Uuid, asset: &str) -> Result<Option<Balance>>;
    
    /// Get all balances for an account
    async fn get_balances(&self, account_id: Uuid) -> Result<Vec<Balance>>;
    
    /// Create or update a balance
    async fn update_balance(&self, balance: Balance) -> Result<Balance>;
    
    /// Ensure a balance exists, creating it if necessary
    async fn ensure_balance(&self, account_id: Uuid, asset: &str) -> Result<Balance>;
    
    /// Begin a database transaction
    async fn begin_transaction(&self) -> Result<DBTransaction> {
        self.transaction_manager().begin_transaction().await
    }
}

/// In-memory repository for account data
pub struct InMemoryAccountRepository {
    /// Accounts by ID
    pub accounts: DashMap<Uuid, Account>,
    /// Balances by account ID and asset
    pub balances: DashMap<(Uuid, String), Balance>,
    /// Transaction manager
    transaction_manager: InMemoryTransactionManager,
}

impl InMemoryAccountRepository {
    /// Create a new in-memory account repository
    pub fn new() -> Self {
        Self {
            accounts: DashMap::new(),
            balances: DashMap::new(),
            transaction_manager: InMemoryTransactionManager::new(),
        }
    }
}

#[async_trait]
impl AccountRepository for InMemoryAccountRepository {
    fn transaction_manager(&self) -> &dyn TransactionManager {
        &self.transaction_manager
    }
    
    /// Create a new account
    async fn create_account(&self) -> Result<Account> {
        let now = Utc::now();
        let account = Account {
            id: Uuid::new_v4(),
            created_at: now,
            updated_at: now,
        };
        
        self.accounts.insert(account.id, account.clone());
        Ok(account)
    }
    
    /// Get an account by ID
    async fn get_account(&self, id: Uuid) -> Result<Option<Account>> {
        Ok(self.accounts.get(&id).map(|a| a.clone()))
    }
    
    /// Get a balance
    async fn get_balance(&self, account_id: Uuid, asset: &str) -> Result<Option<Balance>> {
        Ok(self.balances.get(&(account_id, asset.to_string())).map(|b| b.clone()))
    }
    
    /// Get all balances for an account
    async fn get_balances(&self, account_id: Uuid) -> Result<Vec<Balance>> {
        let balances = self.balances
            .iter()
            .filter_map(|entry| {
                let ((acc_id, _), balance) = entry.pair();
                if *acc_id == account_id {
                    Some(balance.clone())
                } else {
                    None
                }
            })
            .collect();
        
        Ok(balances)
    }
    
    /// Create or update a balance
    async fn update_balance(&self, balance: Balance) -> Result<Balance> {
        let key = (balance.account_id, balance.asset.clone());
        self.balances.insert(key, balance.clone());
        Ok(balance)
    }
    
    /// Ensure a balance exists, creating it if necessary
    async fn ensure_balance(&self, account_id: Uuid, asset: &str) -> Result<Balance> {
        let key = (account_id, asset.to_string());
        
        if let Some(balance) = self.balances.get(&key) {
            Ok(balance.clone())
        } else {
            // Check if the account exists
            if !self.accounts.contains_key(&account_id) {
                return Err(Error::AccountNotFound(format!("Account not found: {}", account_id)));
            }
            
            let balance = Balance::new(account_id, asset.to_string());
            self.balances.insert(key, balance.clone());
            Ok(balance)
        }
    }
}

/// PostgreSQL repository for account data
pub struct PostgresAccountRepository {
    /// Database connection pool
    pool: PgPool,
    /// Transaction manager 
    transaction_manager: PgTransactionManager,
    /// Enable transaction logging
    #[allow(dead_code)]
    transaction_logging: bool,
}

impl PostgresAccountRepository {
    /// Create a new PostgreSQL account repository
    pub async fn new(database_url: Option<String>) -> Result<Self> {
        let pool = match database_url {
            Some(url) => {
                let pool = PgPoolOptions::new()
                    .max_connections(5)
                    .connect(&url)
                    .await
                    .map_err(|e| Error::Database(e))?;
                pool
            },
            None => {
                let database_url = std::env::var("DATABASE_URL")
                    .map_err(|_| Error::ConfigurationError("DATABASE_URL must be set".to_string()))?;
                
                PgPoolOptions::new()
                    .max_connections(5)
                    .connect(&database_url)
                    .await
                    .map_err(|e| Error::Database(e))?
            },
        };
        
        info!("Connected to PostgreSQL database");
        
        Ok(Self { 
            transaction_manager: PgTransactionManager::new(pool.clone()),
            pool,
            transaction_logging: false
        })
    }
    
    /// Create a new PostgreSQL account repository with configuration
    pub async fn with_config(config: &crate::config::AccountServiceConfig) -> Result<Self> {
        info!("Connecting to PostgreSQL database with pool size: {}", config.db_pool_size);
        
        let pool = PgPoolOptions::new()
            .max_connections(config.db_pool_size)
            .connect(&config.database_url)
            .await
            .map_err(|e| Error::Database(e))?;
        
        info!("Connected to PostgreSQL database");
        
        Ok(Self { 
            transaction_manager: PgTransactionManager::new(pool.clone()),
            pool,
            transaction_logging: config.transaction_logging
        })
    }
}

#[async_trait]
impl AccountRepository for PostgresAccountRepository {
    fn transaction_manager(&self) -> &dyn TransactionManager {
        &self.transaction_manager
    }
    /// Create a new account
    async fn create_account(&self) -> Result<Account> {
        debug!("Creating new account in database");
        
        // Create a unique ID for the new account
        let id = Uuid::new_v4();
        let now = Utc::now();
        
        // Insert the account using a manual query rather than sqlx::query_as macro
        sqlx::query(
            "INSERT INTO accounts (id, user_id) VALUES ($1, $2)"
        )
        .bind(id)
        .bind("user") // Default user ID
        .execute(&self.pool)
        .await?;
        
        // Return the new account
        let account = Account {
            id,
            created_at: now,
            updated_at: now,
        };
        
        Ok(account)
    }
    
    /// Get an account by ID
    async fn get_account(&self, id: Uuid) -> Result<Option<Account>> {
        debug!("Getting account from database: {}", id);
        
        // Query the account using manual query rather than sqlx::query_as macro
        let row = sqlx::query(
            "SELECT id, created_at, updated_at FROM accounts WHERE id = $1"
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;
        
        // Convert the row to Account if found
        match row {
            Some(row) => {
                let account = Account {
                    id: row.get("id"),
                    created_at: row.get("created_at"),
                    updated_at: row.get("updated_at"),
                };
                Ok(Some(account))
            },
            None => Ok(None),
        }
    }
    
    /// Get a balance for an account and asset
    async fn get_balance(&self, account_id: Uuid, asset: &str) -> Result<Option<Balance>> {
        debug!("Getting balance from database: {} for {}", asset, account_id);
        
        // Query the balances table
        let row = sqlx::query(
            "SELECT account_id, asset, total, available, locked, updated_at 
             FROM balances 
             WHERE account_id = $1 AND asset = $2"
        )
        .bind(account_id)
        .bind(asset)
        .fetch_optional(&self.pool)
        .await?;
        
        // Convert the row to Balance if found
        match row {
            Some(row) => {
                let total_str: String = row.get("total");
                let available_str: String = row.get("available");
                let locked_str: String = row.get("locked");
                
                // Convert the balance strings to Quantity
                let total = total_str.parse::<Quantity>()
                    .map_err(|e| Error::Internal(format!("Invalid total balance format: {}", e)))?;
                let available = available_str.parse::<Quantity>()
                    .map_err(|e| Error::Internal(format!("Invalid available balance format: {}", e)))?;
                let locked = locked_str.parse::<Quantity>()
                    .map_err(|e| Error::Internal(format!("Invalid locked balance format: {}", e)))?;
                
                let balance = Balance {
                    account_id,
                    asset: asset.to_string(),
                    total,
                    available,
                    locked,
                    updated_at: row.get("updated_at"),
                };
                
                Ok(Some(balance))
            },
            None => Ok(None),
        }
    }
    
    /// Get all balances for an account
    async fn get_balances(&self, account_id: Uuid) -> Result<Vec<Balance>> {
        debug!("Getting all balances for account: {}", account_id);
        
        // Query all balances for the account
        let rows = sqlx::query(
            "SELECT account_id, asset, total, available, locked, updated_at 
             FROM balances 
             WHERE account_id = $1"
        )
        .bind(account_id)
        .fetch_all(&self.pool)
        .await?;
        
        // Convert the rows to a Vec<Balance>
        let mut balances = Vec::with_capacity(rows.len());
        
        for row in rows {
            let total_str: String = row.get("total");
            let available_str: String = row.get("available");
            let locked_str: String = row.get("locked");
            
            // Convert the balance strings to Quantity
            let total = total_str.parse::<Quantity>()
                .map_err(|e| Error::Internal(format!("Invalid total balance format: {}", e)))?;
            let available = available_str.parse::<Quantity>()
                .map_err(|e| Error::Internal(format!("Invalid available balance format: {}", e)))?;
            let locked = locked_str.parse::<Quantity>()
                .map_err(|e| Error::Internal(format!("Invalid locked balance format: {}", e)))?;
            
            let balance = Balance {
                account_id,
                asset: row.get("asset"),
                total,
                available,
                locked,
                updated_at: row.get("updated_at"),
            };
            
            balances.push(balance);
        }
        
        Ok(balances)
    }
    
    /// Update a balance
    async fn update_balance(&self, balance: Balance) -> Result<Balance> {
        debug!("Updating balance in database: {} {}", balance.asset, balance.account_id);
        
        // Try to update an existing balance
        let result = sqlx::query(
            "INSERT INTO balances (account_id, asset, total, available, locked) 
             VALUES ($1, $2, $3, $4, $5)
             ON CONFLICT (account_id, asset) 
             DO UPDATE SET 
                total = $3, 
                available = $4, 
                locked = $5"
        )
        .bind(balance.account_id)
        .bind(&balance.asset)
        .bind(balance.total.to_string())
        .bind(balance.available.to_string())
        .bind(balance.locked.to_string())
        .execute(&self.pool)
        .await?;
        
        if result.rows_affected() == 0 {
            return Err(Error::Internal(format!("Failed to update balance for account: {}, asset: {}", 
                                               balance.account_id, balance.asset)));
        }
        
        Ok(balance)
    }
    
    /// Ensure a balance exists, creating it if necessary
    async fn ensure_balance(&self, account_id: Uuid, asset: &str) -> Result<Balance> {
        debug!("Ensuring balance exists: {} for {}", asset, account_id);
        
        // First check if the account exists
        let account_exists = sqlx::query("SELECT 1 FROM accounts WHERE id = $1")
            .bind(account_id)
            .fetch_optional(&self.pool)
            .await?
            .is_some();
        
        if !account_exists {
            return Err(Error::Internal(format!("Account not found: {}", account_id)));
        }
        
        // Then check if the balance exists
        if let Some(balance) = self.get_balance(account_id, asset).await? {
            return Ok(balance);
        }
        
        // Create a new zero balance
        let balance = Balance::new(account_id, asset.to_string());
        
        // Insert the new balance
        sqlx::query(
            "INSERT INTO balances (account_id, asset, total, available, locked) 
             VALUES ($1, $2, $3, $4, $5)"
        )
        .bind(account_id)
        .bind(asset)
        .bind(balance.total.to_string())
        .bind(balance.available.to_string())
        .bind(balance.locked.to_string())
        .execute(&self.pool)
        .await?;
        
        Ok(balance)
    }
}