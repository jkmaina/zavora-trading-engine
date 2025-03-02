//! Transaction handling for database operations
//!
//! This module provides a standardized approach to database transactions
//! across all services. It defines traits for transaction management
//! and concrete implementations for PostgreSQL.

use async_trait::async_trait;
use sqlx::{PgPool, Transaction as SqlxTransaction, Postgres};

use crate::error::{Error, Result};

/// Transaction enum that can be either PostgreSQL or in-memory
pub enum DBTransaction {
    /// PostgreSQL transaction
    Postgres(PgTransaction),
    /// In-memory transaction
    InMemory(InMemoryTransaction),
}

/// Transaction interface methods
impl DBTransaction {
    /// Commit the transaction
    pub async fn commit(self) -> Result<()> {
        match self {
            DBTransaction::Postgres(tx) => tx.commit().await,
            DBTransaction::InMemory(tx) => tx.commit().await,
        }
    }
    
    /// Rollback the transaction
    pub async fn rollback(self) -> Result<()> {
        match self {
            DBTransaction::Postgres(tx) => tx.rollback().await,
            DBTransaction::InMemory(tx) => tx.rollback().await,
        }
    }
    
    /// Execute a query against the transaction
    pub async fn execute<'a, E>(&mut self, query: E) -> Result<u64> 
    where
        E: sqlx::Execute<'a, Postgres> + Send + 'a,
    {
        match self {
            DBTransaction::Postgres(tx) => tx.execute(query).await,
            DBTransaction::InMemory(tx) => tx.execute(query).await,
        }
    }
}

/// A PostgreSQL transaction implementation
pub struct PgTransaction {
    tx: SqlxTransaction<'static, Postgres>,
}

impl PgTransaction {
    /// Create a new PgTransaction
    pub fn new(tx: SqlxTransaction<'static, Postgres>) -> Self {
        Self { tx }
    }
    
    /// Execute a query within this transaction 
    pub async fn execute<'a, E>(&mut self, query: E) -> Result<u64>
    where
        E: sqlx::Execute<'a, Postgres> + Send + 'a,
    {
        use sqlx::Executor;
        self.tx.execute(query).await
            .map(|r| r.rows_affected())
            .map_err(Error::Database)
    }
    
    /// Commit the transaction
    pub async fn commit(self) -> Result<()> {
        self.tx.commit().await.map_err(Error::Database)
    }
    
    /// Rollback the transaction
    pub async fn rollback(self) -> Result<()> {
        self.tx.rollback().await.map_err(Error::Database)
    }
}

/// Transaction manager trait for creating and managing transactions
#[async_trait]
pub trait TransactionManager: Send + Sync {
    /// Begin a new transaction
    async fn begin_transaction(&self) -> Result<DBTransaction>;
}

/// A PostgreSQL transaction manager implementation
pub struct PgTransactionManager {
    pool: PgPool,
}

impl PgTransactionManager {
    /// Create a new PgTransactionManager
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl TransactionManager for PgTransactionManager {
    async fn begin_transaction(&self) -> Result<DBTransaction> {
        let tx = self.pool.begin().await.map_err(Error::Database)?;
        Ok(DBTransaction::Postgres(PgTransaction::new(tx)))
    }
}

/// In-memory transaction for testing
pub struct InMemoryTransaction {
    committed: bool,
    rolled_back: bool,
}

impl InMemoryTransaction {
    /// Create a new in-memory transaction
    pub fn new() -> Self {
        Self {
            committed: false,
            rolled_back: false,
        }
    }
    
    /// Check if this transaction was committed
    pub fn is_committed(&self) -> bool {
        self.committed
    }
    
    /// Check if this transaction was rolled back
    pub fn is_rolled_back(&self) -> bool {
        self.rolled_back
    }

    /// Execute a query (in-memory implementation)
    pub async fn execute<'a, E>(&mut self, _query: E) -> Result<u64>
    where
        E: Send + 'a,
    {
        // In-memory implementation just returns success with 1 row affected
        Ok(1)
    }
    
    /// Commit the transaction
    pub async fn commit(mut self) -> Result<()> {
        self.committed = true;
        Ok(())
    }
    
    /// Rollback the transaction
    pub async fn rollback(mut self) -> Result<()> {
        self.rolled_back = true;
        Ok(())
    }
}

/// In-memory transaction manager for testing
pub struct InMemoryTransactionManager;

impl InMemoryTransactionManager {
    /// Create a new in-memory transaction manager
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl TransactionManager for InMemoryTransactionManager {
    async fn begin_transaction(&self) -> Result<DBTransaction> {
        Ok(DBTransaction::InMemory(InMemoryTransaction::new()))
    }
}