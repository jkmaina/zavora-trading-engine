// File: tests/test_helpers.rs

use std::process::{Command, Child};
use std::sync::{Arc, Mutex, Once};
use std::time::Duration;
use std::thread;
use std::env;

// Singleton to manage the engine process
lazy_static::lazy_static! {
    static ref ENGINE_PROCESS: Arc<Mutex<Option<Child>>> = Arc::new(Mutex::new(None));
    static ref DB_INITIALIZATION: Once = Once::new();
}

// Start the trading engine for tests
pub fn start_engine() -> Result<(), String> {
    let mut process_guard = ENGINE_PROCESS.lock().unwrap();
    
    // If the engine is already running, do nothing
    if process_guard.is_some() {
        return Ok(());
    }
    
    // Use a different port for tests to avoid conflicts
    let test_port = "8081";
    env::set_var("API_PORT", test_port);
    
    // Start the trading engine using the workspace target
    let process = Command::new("cargo")
        .args(["run", "-p", "trading-engine", "--", "--demo"])
        .spawn()
        .map_err(|e| format!("Failed to start engine: {}", e))?;
    
    *process_guard = Some(process);
    
    // Give the engine some time to start up
    drop(process_guard);
    thread::sleep(Duration::from_secs(2));
    
    // For now, just assume the process started if we got this far
    // In a real production environment, we would do more robust health checks    
    Ok(())
}

// Stop the trading engine after tests
pub fn stop_engine() -> Result<(), String> {
    let mut process_guard = ENGINE_PROCESS.lock().unwrap();
    
    if let Some(mut child) = process_guard.take() {
        child.kill()
            .map_err(|e| format!("Failed to kill engine process: {}", e))?;
        
        child.wait()
            .map_err(|e| format!("Failed to wait for engine process: {}", e))?;
    }
    
    Ok(())
}

// Automatically start and stop the engine for a test
pub struct EngineGuard;

impl EngineGuard {
    pub fn new() -> Result<Self, String> {
        start_engine()?;
        Ok(Self)
    }
}

impl Drop for EngineGuard {
    fn drop(&mut self) {
        let _ = stop_engine();
    }
}

// Database test utilities
#[cfg(feature = "db_tests")]
use sqlx::{postgres::PgPoolOptions, PgPool};

#[cfg(feature = "db_tests")]
pub struct DbTestContext {
    pub pool: PgPool,
}

#[cfg(feature = "db_tests")]
impl DbTestContext {
    // Create a new test database context
    pub async fn new() -> Self {
        // Use a test-specific database configuration
        let db_url = env::var("TEST_DATABASE_URL")
            .expect("TEST_DATABASE_URL must be set for database tests. Run ./create_test_db.sh to set it up.");
        
        // Connect to the test database
        let pool = PgPoolOptions::new()
            .max_connections(5)
            .connect(&db_url)
            .await
            .expect("Failed to connect to test database");
        
        // Run migrations to set up schema
        DB_INITIALIZATION.call_once(|| {
            // This block runs only once per test run
            println!("Initializing test database schema...");
            
            // Create a blocking runtime for migrations
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                // Run migrations to ensure schema is up to date
                common::db::run_migrations(&pool)
                    .await
                    .expect("Failed to run database migrations");
            });
        });
        
        Self { pool }
    }
    
    // Clean up test data after tests
    pub async fn cleanup(&self) {
        // Delete all test data, in the correct order to respect foreign key constraints
        sqlx::query("DELETE FROM trades")
            .execute(&self.pool)
            .await
            .expect("Failed to clean up trades table");
            
        sqlx::query("DELETE FROM orders")
            .execute(&self.pool)
            .await
            .expect("Failed to clean up orders table");
            
        sqlx::query("DELETE FROM market_summaries")
            .execute(&self.pool)
            .await
            .expect("Failed to clean up market_summaries table");
            
        sqlx::query("DELETE FROM order_books")
            .execute(&self.pool)
            .await
            .expect("Failed to clean up order_books table");
            
        sqlx::query("DELETE FROM balances")
            .execute(&self.pool)
            .await
            .expect("Failed to clean up balances table");
            
        sqlx::query("DELETE FROM accounts")
            .execute(&self.pool)
            .await
            .expect("Failed to clean up accounts table");
            
        sqlx::query("DELETE FROM markets")
            .execute(&self.pool)
            .await
            .expect("Failed to clean up markets table");
    }
}