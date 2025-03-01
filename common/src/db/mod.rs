use std::env;
use sqlx::{postgres::PgPoolOptions, PgPool, Pool, Postgres};

use crate::error::Result;

pub mod models;
pub mod queries;
pub mod transaction;

// Re-export transaction types
pub use transaction::{
    DBTransaction, TransactionManager,
    PgTransaction, PgTransactionManager,
    InMemoryTransaction, InMemoryTransactionManager
};

/// Database pool type
pub type DbPool = Pool<Postgres>;

/// Initialize the database connection pool
pub async fn init_db_pool() -> Result<DbPool> {
    let database_url = env::var("DATABASE_URL")
        .expect("DATABASE_URL must be set");
    
    let pool = PgPoolOptions::new()
        .max_connections(50)
        .connect(&database_url)
        .await?;
    
    Ok(pool)
}

/// Run migrations on the database
pub async fn run_migrations(pool: &PgPool) -> Result<()> {
    let migrations_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("migrations");

    sqlx::migrate::Migrator::new(migrations_path)
        .await?
        .run(pool)
        .await?;
    
    Ok(())
}