// Database persistence tests - simplified version

#[cfg(test)]
mod db_persistence_tests {
    use sqlx::Row;
    use std::env;
    use tokio::runtime::Runtime;
    use sqlx::{postgres::PgPoolOptions, PgPool};
    
    // Helper function to run async tests
    fn run_db_test<F>(test: F)
    where
        F: FnOnce(PgPool) -> futures::future::BoxFuture<'static, ()> + Send + 'static,
    {
        // Skip test if TEST_DATABASE_URL is not set
        let db_url = match env::var("TEST_DATABASE_URL") {
            Ok(url) => url,
            Err(_) => {
                println!("Skipping database test: TEST_DATABASE_URL not set");
                println!("Run ./create_test_db.sh to set up the test database");
                return;
            }
        };
            
        // Create runtime
        let rt = Runtime::new().unwrap();
        
        // Run the test
        rt.block_on(async {
            // Create database connection
            let pool = match PgPoolOptions::new()
                .max_connections(5)
                .connect(&db_url)
                .await
            {
                Ok(pool) => pool,
                Err(err) => {
                    println!("Skipping database test: could not connect to database: {}", err);
                    println!("Run ./create_test_db.sh to set up the test database");
                    return;
                }
            };
                
            // Run the test
            test(pool).await;
        });
    }

    // Simple test to write and read data from a temp table
    #[test]
    #[ignore = "Requires test database, run with RUST_TEST_THREADS=1 cargo test -- --ignored"]
    fn test_basic_db_operations() {
        run_db_test(|pool| {
            Box::pin(async move {
                // Create a simple test table
                sqlx::query("
                    CREATE TABLE IF NOT EXISTS test_db_persistence (
                        id SERIAL PRIMARY KEY,
                        name TEXT NOT NULL,
                        value INTEGER NOT NULL
                    )
                ")
                .execute(&pool)
                .await
                .expect("Failed to create temporary table");
                
                // Insert data
                let name = "test_value";
                let value = 42;
                
                sqlx::query("
                    INSERT INTO test_db_persistence (name, value) VALUES ($1, $2)
                ")
                .bind(name)
                .bind(value)
                .execute(&pool)
                .await
                .expect("Failed to insert data");
                
                // Read data back
                let row = sqlx::query("
                    SELECT name, value FROM test_db_persistence WHERE name = $1
                ")
                .bind(name)
                .fetch_one(&pool)
                .await
                .expect("Failed to read data");
                
                // Verify data
                assert_eq!(row.get::<&str, _>("name"), name);
                assert_eq!(row.get::<i32, _>("value"), value);
                
                // Clean up
                sqlx::query("DROP TABLE IF EXISTS test_db_persistence")
                    .execute(&pool)
                    .await
                    .expect("Failed to drop table");
            })
        });
    }

    // Test with multiple rows
    #[test]
    #[ignore = "Requires test database, run with RUST_TEST_THREADS=1 cargo test -- --ignored"]
    fn test_multiple_rows() {
        run_db_test(|pool| {
            Box::pin(async move {
                // Create a simple test table
                sqlx::query("
                    CREATE TABLE IF NOT EXISTS test_multi_rows (
                        id SERIAL PRIMARY KEY,
                        name TEXT NOT NULL,
                        value INTEGER NOT NULL
                    )
                ")
                .execute(&pool)
                .await
                .expect("Failed to create temporary table");
                
                // Insert multiple rows
                for i in 1..=5 {
                    let name = format!("value_{}", i);
                    let value = i * 10;
                    
                    sqlx::query("
                        INSERT INTO test_multi_rows (name, value) VALUES ($1, $2)
                    ")
                    .bind(&name)
                    .bind(value)
                    .execute(&pool)
                    .await
                    .expect("Failed to insert data");
                }
                
                // Read all rows
                let rows = sqlx::query("
                    SELECT name, value FROM test_multi_rows ORDER BY id
                ")
                .fetch_all(&pool)
                .await
                .expect("Failed to read data");
                
                // Verify data
                assert_eq!(rows.len(), 5);
                
                for (i, row) in rows.iter().enumerate() {
                    let expected_name = format!("value_{}", i+1);
                    let expected_value = (i+1) * 10;
                    
                    assert_eq!(row.get::<&str, _>("name"), expected_name);
                    assert_eq!(row.get::<i32, _>("value"), expected_value as i32);
                }
                
                // Clean up
                sqlx::query("DROP TABLE IF EXISTS test_multi_rows")
                    .execute(&pool)
                    .await
                    .expect("Failed to drop table");
            })
        });
    }
    
    // Test transactions
    #[test]
    #[ignore = "Requires test database, run with RUST_TEST_THREADS=1 cargo test -- --ignored"]
    fn test_transactions() {
        run_db_test(|pool| {
            Box::pin(async move {
                // Create a simple test table
                sqlx::query("
                    CREATE TABLE IF NOT EXISTS test_transactions (
                        id SERIAL PRIMARY KEY,
                        name TEXT NOT NULL,
                        value INTEGER NOT NULL
                    )
                ")
                .execute(&pool)
                .await
                .expect("Failed to create temporary table");
                
                // Start a transaction
                let mut tx = pool.begin().await.expect("Failed to start transaction");
                
                // Insert data in transaction
                sqlx::query("
                    INSERT INTO test_transactions (name, value) VALUES ($1, $2)
                ")
                .bind("tx_value_1")
                .bind(100)
                .execute(&mut *tx)
                .await
                .expect("Failed to insert data");
                
                // Insert more data
                sqlx::query("
                    INSERT INTO test_transactions (name, value) VALUES ($1, $2)
                ")
                .bind("tx_value_2")
                .bind(200)
                .execute(&mut *tx)
                .await
                .expect("Failed to insert data");
                
                // Commit transaction
                tx.commit().await.expect("Failed to commit transaction");
                
                // Verify data
                let rows = sqlx::query("
                    SELECT name, value FROM test_transactions ORDER BY id
                ")
                .fetch_all(&pool)
                .await
                .expect("Failed to read data");
                
                assert_eq!(rows.len(), 2);
                assert_eq!(rows[0].get::<&str, _>("name"), "tx_value_1");
                assert_eq!(rows[0].get::<i32, _>("value"), 100);
                assert_eq!(rows[1].get::<&str, _>("name"), "tx_value_2");
                assert_eq!(rows[1].get::<i32, _>("value"), 200);
                
                // Test rollback
                let mut tx = pool.begin().await.expect("Failed to start transaction");
                
                // Insert data that will be rolled back
                sqlx::query("
                    INSERT INTO test_transactions (name, value) VALUES ($1, $2)
                ")
                .bind("rollback_value")
                .bind(300)
                .execute(&mut *tx)
                .await
                .expect("Failed to insert data");
                
                // Rollback instead of commit
                tx.rollback().await.expect("Failed to rollback transaction");
                
                // Verify rollback data wasn't persisted
                let rows = sqlx::query("
                    SELECT name, value FROM test_transactions WHERE name = $1
                ")
                .bind("rollback_value")
                .fetch_all(&pool)
                .await
                .expect("Failed to read data");
                
                assert_eq!(rows.len(), 0, "Rollback should have prevented data from being persisted");
                
                // Clean up
                sqlx::query("DROP TABLE IF EXISTS test_transactions")
                    .execute(&pool)
                    .await
                    .expect("Failed to drop table");
            })
        });
    }
}