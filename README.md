# Zavora Trading Engine

## Overview
The Zavora Trading Engine is a high-performance, low-latency trading system implemented in Rust. This engine is designed to provide robust order matching, market data processing, and trade execution with minimal overhead. The system follows a microservices architecture with asynchronous processing for maximum throughput and scalability.

## Features
- High-throughput order processing with asynchronous execution
- Low-latency matching algorithm using price-time priority
- Memory-safe implementation leveraging Rust's ownership model
- Concurrent processing of market data streams
- Comprehensive risk management and balance tracking
- Fully dockerized database setup for development and testing
- REST API and WebSocket endpoints for real-time data access
- Modular design allowing for easy component replacement or extension

## System Architecture

The Zavora Trading Engine is built on a modular microservices architecture for flexibility, scalability, and maintainability:

### Core Components

#### Matching Engine (`matching-engine/`)
- Maintains order books for all markets
- Processes limit and market orders
- Implements price-time priority matching algorithm
- Generates trades when orders match
- Supports order cancellation and modification

#### Account Service (`account-service/`)
- Manages user accounts and balances
- Handles deposits and withdrawals
- Reserves and releases funds for orders
- Processes trades to update balances
- Supports both in-memory and PostgreSQL persistence
- Provides transaction processing with ACID guarantees

#### Market Data Service (`market-data/`)
- Maintains market statistics and price information
- Processes and aggregates trade data
- Provides order book snapshots and updates
- Supports both real-time and historical data

#### API Gateway (`api-gateway/`)
- RESTful HTTP API for all services
- WebSocket support for real-time updates
- Request validation and error handling
- Authentication and authorization (planned)
- Rate limiting and throttling protection (planned)

#### Common Utilities (`common/`)
- Shared data models and structures
- Standardized error handling system with domain-specific error types
- Unified transaction system with consistent rollback
- Database access abstractions
- Decimal number handling for currency
- Utility functions and helpers

### Communication Flow
1. External requests enter through the API Gateway
2. API Gateway routes requests to appropriate services
3. Services communicate asynchronously using Tokio
4. Each service is responsible for its own data persistence
5. Services maintain consistency through transaction patterns

### Persistence Layer
- PostgreSQL database for all persistent data
- Separation of read and write operations
- Optimized queries for high-throughput operations
- Database connection pooling for efficiency

## Getting Started

> **IMPORTANT**: If you encounter any issues with the build process or running Docker commands, see the [Troubleshooting](#troubleshooting) section below.

### Prerequisites
- [Rust](https://www.rust-lang.org/tools/install) (1.70+)
  ```bash
  # Install Rust using rustup
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
  
  # Add recommended components
  rustup component add rustfmt clippy
  
  # For enhanced development (recommended)
  sudo apt update
  sudo apt install build-essential -y
  sudo apt install pkg-config libssl-dev -y
  cargo install cargo-watch cargo-expand cargo-edit
  ```
  
- [Docker](https://docs.docker.com/get-docker/) and [Docker Compose](https://docs.docker.com/compose/install/)
  ```bash
  # Verify installation
  docker --version
  docker-compose --version
  
  # On Linux, add your user to the docker group (REQUIRED)
  sudo groupadd docker
  sudo usermod -aG docker $USER
  newgrp docker  # Apply group changes to current shell
  
  # Verify docker permissions
  docker ps
  ```
  
- [PostgreSQL client](https://www.postgresql.org/download/) (recommended for development)
  ```bash
  # On Ubuntu/Debian
  sudo apt-get install postgresql-client
  
  # On macOS with Homebrew
  brew install libpq
  brew link --force libpq
  ```
  
- Development tools (recommended)
  ```bash
  # jq for API testing
  sudo apt-get install jq  # or brew install jq on macOS
  
  # SQLx CLI for database management
  cargo install sqlx-cli
  
  # HTTP testing utility
  cargo install httpie
  
  # Node.js for WebSocket testing
  curl -fsSL https://deb.nodesource.com/setup_20.x | sudo -E bash -
  sudo apt-get install -y nodejs
  npm install ws
  ```

### Building the Project
```bash
# Clone the repository
git clone https://github.com/jkmaina/zavora-trading-engine.git

# Build the project
cd zavora-trading-engine
cargo build --release
```

### Database Setup

#### Production Database
```bash
# Start the production PostgreSQL database on port 5435
docker compose up -d postgres

# Run database setup and test script
./api_test.sh
```

#### Test Database
```bash
# Start the test PostgreSQL database on port 5434
./create_test_db.sh

# Run database tests
source .env.test && cargo test --test db_tests -- --ignored
```

### Environment Variables
The trading engine uses the following environment variables:

- `DATABASE_URL`: PostgreSQL connection string for the main database
  - Example: `postgres://viabtc:viabtc@localhost:5435/viabtc`
- `TEST_DATABASE_URL`: PostgreSQL connection string for the test database
  - Example: `postgres://viabtc:viabtc@localhost:5434/viabtc_test`
- `API_PORT`: Port for the API server (default: 8081)
- `DEBUG`: Set to "1" to enable detailed request/response logging
- `RUST_LOG`: Logging level (e.g., info, debug, trace)

You can create a `.env` file in the project root for development:

```bash
# Create .env file for development
cat > .env << EOF
DATABASE_URL=postgres://viabtc:viabtc@localhost:5435/viabtc
RUST_LOG=info,sqlx=warn
API_PORT=8081
DEBUG=0
EOF

# Create .env.test file for testing
cat > .env.test << EOF
export TEST_DATABASE_URL=postgres://viabtc:viabtc@localhost:5434/viabtc_test
EOF
```

### Running Tests
```bash
# Run all non-database tests
cargo test

# Run database integration tests (requires test database)
source .env.test && cargo test --test db_tests -- --ignored

# Test account service
cargo test -p account-service

# Run account service PostgreSQL tests
source .env.test && cargo test -p account-service --test account_postgres_tests -- --ignored
```

### Running the Services

There are multiple ways to run the trading engine services:

#### Running the Complete Trading Engine
```bash
# Start the PostgreSQL database
docker compose up -d postgres

# Run the trading engine with demo data
cargo run -p trading-engine -- --demo
```

The trading engine will:
1. Start all required services
2. Create demo accounts and market data
3. Start an API server on port 8081 (configurable via API_PORT env var)

#### Running Individual Services
```bash
# Start the account service
cargo run -p account-service -- start

# Start the API gateway separately (requires all services to be running)
cargo run --bin api-gateway

# In a separate terminal, test the API with curl commands:
curl -s -X GET "http://localhost:8081/api/v1/health"
curl -s -X GET "http://localhost:8081/api/v1/markets"
```

#### Testing the API
A test script is provided to verify API functionality:
```bash
# Make the script executable
chmod +x test_api.sh

# Run the API test script
./test_api.sh
```

## Database Architecture

The system uses PostgreSQL for persistence with the following main tables:
- `accounts` - User accounts and balances
- `markets` - Available trading pairs and their parameters
- `orders` - Open and historical orders
- `trades` - Executed trades
- `market_summaries` - Market statistics and price data

See `migrations/20240227000000_initial_schema.sql` for the complete schema.

## API Documentation

### Swagger UI

The API Gateway includes Swagger UI for interactive API exploration and testing. You can access the Swagger documentation at:

```
http://localhost:8080/swagger-ui/
```

Key features of the Swagger documentation:
- Interactive API explorer
- Request/response examples
- Schema definitions for all data models
- Try-it-out functionality to test API calls directly from the browser
- OpenAPI specification available at `/openapi.json`

### REST API Endpoints

The API Gateway exposes the following RESTful endpoints:

#### Health Check
- `GET /api/v1/health` - Check API server health

#### Account Management
- `POST /api/v1/accounts` - Create a new account
- `GET /api/v1/accounts/:id` - Get account details
- `GET /api/v1/accounts/:id/balances` - Get account balances
- `POST /api/v1/accounts/:id/deposit` - Deposit funds
- `POST /api/v1/accounts/:id/withdraw` - Withdraw funds

#### Market Data
- `GET /api/v1/markets` - List all markets
- `GET /api/v1/markets/:market/order-book` - Get market order book
- `GET /api/v1/markets/:market/ticker` - Get market ticker
- `GET /api/v1/markets/:market/trades` - Get recent trades
- `GET /api/v1/markets/:market/candles` - Get OHLCV candles
- `GET /api/v1/markets/tickers` - Get all market tickers

#### Order Management
- `POST /api/v1/orders` - Place a new order
- `GET /api/v1/orders/:id` - Get order details
- `POST /api/v1/orders/:id` - Cancel an order
- `GET /api/v1/accounts/:id/orders` - List account orders

### WebSocket API

The WebSocket API provides real-time updates through a single endpoint:

- `WebSocket /ws` - Real-time data stream

WebSocket messages are JSON objects with the following format:
```json
{
  "op": "subscribe",
  "channel": "orderbook",
  "market": "BTC/USD"
}
```

Supported channels:
- `orderbook` - Order book updates
- `trades` - Real-time trade updates
- `ticker` - Ticker updates

## Performance

The Zavora Trading Engine demonstrates excellent performance characteristics:

- **Order Processing**: Sub-microsecond latency under typical market conditions
- **Throughput**: Can handle thousands of orders per second per market
- **Memory Efficiency**: Low memory overhead due to Rust's ownership model
- **Scalability**: Independent services can be scaled horizontally as needed

## Testing Strategy

The project implements a comprehensive testing strategy:

- **Unit Tests**: Cover individual components and functions
- **Integration Tests**: Verify interactions between components
- **Database Tests**: Ensure persistence works correctly with real database
- **API Tests**: Validate HTTP endpoints behavior and consistency of error responses
- **Performance Tests**: Benchmark critical paths for performance regression

See [TESTING_DB.md](TESTING_DB.md) for detailed database testing information.

## Development Recommendations

### IDE Setup

For the best development experience with Rust, we recommend:

- **VS Code** with the following extensions:
  - rust-analyzer (essential)
  - CodeLLDB (for debugging)
  - crates (for dependency management)
  - Better TOML (for Cargo.toml files)
  - SQLx (for database operations)

- **IntelliJ IDEA / CLion** with:
  - Rust plugin
  - Database navigator

### Code Quality Tools

Maintain code quality with these commands:

```bash
# Run all linting checks
cargo clippy -- -D warnings

# Format all code
cargo fmt --all

# Check for unused dependencies
cargo audit

# Run all tests (excluding database tests)
cargo test --all-features
```

### Debugging

For debugging, use either:

1. VS Code with CodeLLDB extension:
   - Create a `.vscode/launch.json` with appropriate configurations
   - Use the Run/Debug panel to launch services

2. Command line with `rust-gdb` or `rust-lldb`:
   ```bash
   rust-gdb target/debug/trading-engine
   ```

3. Enable detailed API request/response logging:
   ```bash
   # Set DEBUG=1 in your .env file or
   DEBUG=1 cargo run -p trading-engine -- --demo
   
   # This will log detailed HTTP request/response information
   # You'll see headers, bodies, timing information, and more
   ```

### Service Development

When developing a specific service:

```bash
# Watch for changes and automatically rebuild/test
cargo watch -x "run -p account-service -- start"

# Test a specific service
cargo test -p matching-engine

# Load database schema for offline SQLx macros
cargo sqlx prepare --database-url "postgres://viabtc:viabtc@localhost:5435/viabtc"
```

## Release Management

The project follows semantic versioning and maintains a [RELEASE_NOTES.md](./RELEASE_NOTES.md) file with details of each release. GitHub releases are created to align with the release notes.

For detailed information on our deployment and release process, see the [deployment guide](./deploy/DEPLOYMENT.md).

## Contributing

Contributions are welcome! To contribute to the Zavora Trading Engine:

1. Fork the repository
2. Create a feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m 'Add some amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request

## Troubleshooting

### Docker Permission Issues

If you encounter `permission denied` errors when running Docker commands:

```
permission denied while trying to connect to the Docker daemon socket at unix:///var/run/docker.sock
```

Solution:
1. Add your user to the docker group:
   ```bash
   sudo groupadd docker
   sudo usermod -aG docker $USER
   ```
2. Apply the changes to your current shell:
   ```bash
   newgrp docker
   ```
   OR log out and log back in

3. Verify you can run Docker without sudo:
   ```bash
   docker ps
   ```

### Build Errors

If you encounter build errors related to missing files:

```
error: couldn't read src/lib.rs: No such file or directory (os error 2)
```

Solution:
1. Make sure you have the latest code from the repository:
   ```bash
   git pull
   ```
2. Try cleaning the build cache:
   ```bash
   cargo clean
   cargo build --release
   ```

### API Connectivity Issues

If API tests don't work after starting the Docker containers:

1. Check if the PostgreSQL container is running:
   ```bash
   docker ps
   ```
2. Ensure the API server is running on the right port (default: 8081):
   ```bash
   netstat -tulpn | grep 8081
   ```
3. Check Docker logs for any errors:
   ```bash
   docker logs zavora-trading-engine-postgres-1
   ```

### WebSocket Test Failures

If you encounter WebSocket test failures like:

```
WebSocket test failed: "Failed to execute Node.js script: No such file or directory (os error 2)"
```

Install Node.js and npm:

```bash
# On Ubuntu/Debian
curl -fsSL https://deb.nodesource.com/setup_20.x | sudo -E bash -
sudo apt-get install -y nodejs

# Verify installation
node --version
npm --version

# Install WebSocket dependencies
npm install ws
```

## License

This project is licensed under the MIT License - see the LICENSE file for details.

## Acknowledgments

- The Rust community for their excellent tooling and libraries
- Contributors and reviewers who have helped improve this project