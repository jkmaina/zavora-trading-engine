# Account Service

The Account Service is a microservice responsible for managing user accounts, balances, and financial transactions within the Zavora Trading Engine. It provides essential functionality for secure asset custody and accurate balance tracking.

## Features

- **Account Management**: Create and retrieve user accounts
- **Multi-Asset Balances**: Support for multiple digital assets per account 
- **Transaction Management**: Deposit and withdraw assets
- **Order Funding**: Reserve and release funds for orders
- **Trade Settlement**: Process completed trades with ACID transaction guarantees
- **Extensible Storage**: In-memory implementation for testing and PostgreSQL for production

## Getting Started

### Prerequisites

- Rust 1.70 or newer
- PostgreSQL database (for production usage)

### Configuration

The Account Service can be configured using environment variables or command-line arguments:

```
DATABASE_URL=postgres://user:password@localhost:5432/zavora
DB_POOL_SIZE=5
TRANSACTION_LOGGING=false
```

### Running the Service

```bash
# Run with default configuration (uses .env file)
cargo run -p account-service -- start

# Run with custom database URL
cargo run -p account-service -- start --database-url postgres://user:pass@localhost:5432/zavora

# Run with custom pool size and transaction logging
cargo run -p account-service -- start --pool-size 10 --transaction-logging
```

## API Usage

The Account Service implements these main operations:

### Create Account

Creates a new user account with a unique identifier.

```rust
let account = service.create_account().await?;
println!("Created account with ID: {}", account.id);
```

### Get Account

Retrieves account information by ID.

```rust
let account = service.get_account(account_id).await?;
```

### Deposit Funds

Increases an account's balance for a specified asset.

```rust
let balance = service.deposit(account_id, "BTC", dec!(1.5)).await?;
println!("New balance: {} {}", balance.total, balance.asset);
```

### Withdraw Funds

Decreases an account's balance for a specified asset.

```rust
let balance = service.withdraw(account_id, "BTC", dec!(0.5)).await?;
```

### Reserve Funds for Orders

Locks funds when a new order is placed, ensuring they can't be withdrawn.

```rust
service.reserve_for_order(&order).await?;
```

### Release Reserved Funds

Unlocks funds when an order is canceled.

```rust
service.release_reserved_funds(&order).await?;
```

### Process Trades

Updates balances for both parties when a trade is executed.

```rust
service.process_trade(&trade).await?;
```

## Architecture

### Repository Pattern

The Account Service follows the repository pattern to abstract storage operations:

- `AccountRepository` trait defines the interface for account data access
- `InMemoryAccountRepository` provides a fast in-memory implementation for testing
- `PostgresAccountRepository` provides a persistent implementation for production

```rust
// Repository trait defining storage operations
#[async_trait]
pub trait AccountRepository: Send + Sync {
    async fn create_account(&self) -> Result<Account>;
    async fn get_account(&self, id: Uuid) -> Result<Option<Account>>;
    async fn get_balance(&self, account_id: Uuid, asset: &str) -> Result<Option<Balance>>;
    async fn get_balances(&self, account_id: Uuid) -> Result<Vec<Balance>>;
    async fn update_balance(&self, balance: Balance) -> Result<Balance>;
    async fn ensure_balance(&self, account_id: Uuid, asset: &str) -> Result<Balance>;
    async fn begin_transaction(&self) -> Result<Box<dyn Transaction>>;
}
```

### Dependency Injection

The service uses dependency injection to support different repository implementations:

```rust
// Create with in-memory repository (for testing)
let service = AccountService::new();

// Create with PostgreSQL repository (for production)
let service = AccountService::with_repository(
    RepositoryType::Postgres(Some("postgres://user:pass@localhost/db".to_string()))
).await?;

// Create with configuration
let config = AccountServiceConfig::from_env();
let service = AccountService::with_config(&config).await?;
```

### Transaction Management

The service supports database transactions to ensure consistency:

- `Transaction` trait for abstracting transaction operations
- ACID guarantees for critical operations like trade processing
- Automatic rollback on errors
- Optimistic concurrency control

```rust
// Example of transaction use in trade processing
async fn process_trade(&self, trade: &Trade) -> Result<()> {
    // Start a database transaction
    let transaction = self.repo.begin_transaction().await?;
    
    // Get balances (within transaction context)
    let buyer_quote_balance = self.repo.get_balance(trade.buyer_id, quote_asset).await?;
    let buyer_base_balance = self.repo.get_balance(trade.buyer_id, base_asset).await?;
    // ...
    
    // Update balances (within transaction context)
    self.repo.update_balance(buyer_quote_balance).await?;
    self.repo.update_balance(buyer_base_balance).await?;
    // ...
    
    // Commit the transaction
    transaction.commit().await?;
    
    Ok(())
}
```

### Asynchronous Design

The Account Service is fully asynchronous using Tokio runtime:

- Non-blocking database operations
- Async/await syntax for readable code
- Future-based error handling
- Efficient resource utilization

## Testing

The Account Service has comprehensive test coverage through multiple test categories:

### Unit Tests

Unit tests verify individual functions and components in isolation:

```bash
# Run unit tests
cargo test -p account-service

# Run tests with output
cargo test -p account-service -- --nocapture

# Run specific test
cargo test -p account-service balance_operations
```

### In-Memory Tests

These tests verify the service behavior using the in-memory repository implementation:

```bash
# Run in-memory service tests
cargo test -p account-service --test in_memory_tests
```

Key in-memory tests include:
- Account creation and retrieval
- Balance operations (deposit, withdraw)
- Order funding (reserve and release)
- Trade settlement with multiple assets

### PostgreSQL Integration Tests

These tests verify persistence behavior with a real PostgreSQL database:

```bash
# Setup test database
./create_test_db.sh

# Run database tests with proper environment variables
source .env.test && cargo test -p account-service --test account_postgres_tests -- --ignored
```

Key PostgreSQL tests include:
- Database connection and pooling
- Transaction handling (commit and rollback)
- Concurrent operations
- Error handling and recovery

### Minimal Example Tests

Simple examples showing basic usage of the account service:

```bash
# Run minimal example tests
cargo test -p account-service --test minimal_test
```

## Error Handling

The Account Service provides comprehensive error handling through a custom error type hierarchy:

```rust
pub enum Error {
    InsufficientBalance(String),
    InvalidOrder(String),
    OrderNotFound(String),
    Database(sqlx::Error),
    Migration(sqlx::migrate::MigrateError),
    Serialization(String),
    Internal(String),
}
```

### Error Categories

- **Validation Errors**
  - `Error::InsufficientBalance` - Not enough available funds for withdrawal or order
  - `Error::InvalidOrder` - Order parameters are invalid or inconsistent

- **Resource Errors**
  - `Error::OrderNotFound` - Referenced order doesn't exist
  - `Error::Internal` - Referenced account or balance doesn't exist

- **Infrastructure Errors**
  - `Error::Database` - Database operation failed
  - `Error::Migration` - Schema migration failed
  - `Error::Serialization` - Data serialization/deserialization failed

### Error Handling Examples

**Handling Insufficient Balance:**
```rust
match service.withdraw(account_id, "BTC", amount).await {
    Ok(balance) => { /* Success case */ },
    Err(Error::InsufficientBalance(msg)) => {
        // Handle insufficient funds case
        println!("Cannot withdraw: {}", msg);
    },
    Err(e) => { /* Handle other errors */ }
}
```

**Transaction Rollback:**
```rust
// If any operation within process_trade fails, the transaction will
// automatically roll back, keeping the database in a consistent state
match service.process_trade(&trade).await {
    Ok(_) => println!("Trade processed successfully"),
    Err(e) => {
        // Transaction rolled back automatically
        println!("Trade processing failed: {}", e);
    }
}
```

## Performance Considerations

The Account Service is designed for high performance:

- **Connection Pooling**: Minimizes connection overhead
- **Prepared Statements**: Reduces query parsing time
- **Efficient Locking**: Granular locking only when needed
- **Optimized Queries**: Careful design of database access patterns
- **Batch Processing**: Support for processing multiple operations
- **Asynchronous I/O**: Non-blocking database operations

## Future Extensions

Planned improvements to the Account Service include:

- **Multi-tenancy support**: Isolation between different organizations
- **Audit logging**: Comprehensive tracking of all financial operations
- **Rate limiting**: Protection against excessive operations
- **Scheduled operations**: Support for time-based transfers and settlements
- **Reporting**: Advanced reporting and analytics capabilities