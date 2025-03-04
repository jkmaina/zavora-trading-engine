//! Trading engine integration module

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH, Instant};

use clap::Parser;
use common::model::market::Market;
use dotenv::dotenv;
use rust_decimal_macros::dec;
use tokio::signal;
use tracing::{info, debug, Level};
use tracing_subscriber::{FmtSubscriber, EnvFilter, fmt::format::FmtSpan};
use account_service::AccountService;
use market_data::MarketDataService;
use matching_engine::MatchingEngine;
use uuid::Uuid;
use axum::extract::State;
use axum::response::IntoResponse;
use axum::Json;

/// Command line arguments
#[derive(Parser, Debug)]
#[clap(author, version, about)]
struct Args {
    /// Run with demo data
    #[clap(short, long)]
    demo: bool,
}

// Static variable to track service start time
static START_TIME: AtomicU64 = AtomicU64::new(0);

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load environment variables
    dotenv().ok();
    
    // Parse command line arguments
    let args = Args::parse();
    
    // Initialize tracing with debug level if DEBUG=1 in .env
    let env_debug = std::env::var("DEBUG").unwrap_or_else(|_| "0".to_string());
    let log_level = if env_debug == "1" { Level::DEBUG } else { Level::INFO };
    
    // Create an environment filter
    let env_filter = EnvFilter::builder()
        .with_default_directive(log_level.into())
        .parse("tower_http=debug,api_gateway=debug,market_data=debug,matching_engine=debug,account_service=debug")
        .unwrap();
    
    let subscriber = FmtSubscriber::builder()
        .with_env_filter(env_filter)
        .with_span_events(FmtSpan::NEW | FmtSpan::CLOSE)
        .finish();
    
    // Only set the global subscriber if it hasn't been set already
    if tracing::subscriber::set_global_default(subscriber).is_ok() {
        info!("Tracing initialized");
        if env_debug == "1" {
            debug!("Debug logging enabled");
        }
    }
    
    info!("Starting Zavora Trading Engine...");
    
    // Initialize service start time for uptime tracking
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    START_TIME.store(now, Ordering::Relaxed);
    
    // Initialize services
    let matching_engine = MatchingEngine::new();
    let account_service = Arc::new(AccountService::new());
    let market_data_service = Arc::new(MarketDataService::new());
    
    // Register markets
    let btc_usd = Market {
        symbol: "BTC/USD".to_string(),
        base_asset: "BTC".to_string(),
        quote_asset: "USD".to_string(),
        price_tick: dec!(0.01),
        quantity_step: dec!(0.0001),
        min_order_size: dec!(10.0),
        max_price_deviation: 10.0,
        trading_enabled: true,
    };
    
    matching_engine.register_market(btc_usd.symbol.clone());
    
    // Create app state
    let matching_engine = Arc::new(matching_engine);
    
    // Create demo data if requested
    if args.demo {
        info!("Creating demo data...");
        create_demo_data(
            matching_engine.clone(),
            account_service.clone(),
            market_data_service.clone(),
        ).await?;
    }
    
    // Start API server in a separate task
    let api_handle = {
        let matching_engine = matching_engine.clone();
        let account_service = account_service.clone();
        let market_data_service = market_data_service.clone();
        let btc_usd = btc_usd.clone();
        
        tokio::spawn(async move {
            // Create app state
            let state = Arc::new(api_gateway::AppState {
                matching_engine,
                account_service,
                market_data_service,
                markets: vec![btc_usd],
            });
            
            // Set up CORS
            let cors = tower_http::cors::CorsLayer::new()
                .allow_origin(tower_http::cors::Any)
                .allow_methods(tower_http::cors::Any)
                .allow_headers(tower_http::cors::Any);
            
            // Set up API routes
            let api_routes = axum::Router::new()
                //Health Check
                .route("/health", axum::routing::get(health_check))
                // Account routes
                .route("/accounts", axum::routing::post(api_gateway::api::account::create_account))
                .route("/accounts/:id", axum::routing::get(api_gateway::api::account::get_account))
                .route("/accounts/:id/balances", axum::routing::get(api_gateway::api::account::get_balances))
                .route("/accounts/:id/deposit", axum::routing::post(api_gateway::api::account::deposit))
                .route("/accounts/:id/withdraw", axum::routing::post(api_gateway::api::account::withdraw))
                
                // Market routes
                .route("/markets", axum::routing::get(api_gateway::api::market::get_markets))
                .route("/markets/:market/order-book", axum::routing::get(api_gateway::api::market::get_order_book))
                .route("/markets/:market/ticker", axum::routing::get(api_gateway::api::market::get_ticker))
                .route("/markets/:market/trades", axum::routing::get(api_gateway::api::market::get_trades))
                .route("/markets/:market/candles", axum::routing::get(api_gateway::api::market::get_candles))
                .route("/markets/tickers", axum::routing::get(api_gateway::api::market::get_tickers))
                
                // Order routes
                .route("/orders", axum::routing::post(api_gateway::api::order::place_order))
                .route("/orders/:id", axum::routing::get(api_gateway::api::order::get_order))
                .route("/orders/:id", axum::routing::post(api_gateway::api::order::cancel_order))
                .route("/accounts/:id/orders", axum::routing::get(api_gateway::api::order::get_orders));
            
            // Set up websocket route
            let ws_routes = axum::Router::new()
                .route("/ws", axum::routing::get(api_gateway::ws::handler::ws_handler));
            
            // Combine all routes
            let app = axum::Router::new()
                .nest("/api/v1", api_routes)
                .merge(ws_routes)
                .layer(cors)
                .layer(tower_http::trace::TraceLayer::new_for_http()
                    .make_span_with(tower_http::trace::DefaultMakeSpan::new().level(log_level))
                    .on_request(tower_http::trace::DefaultOnRequest::new().level(log_level))
                    .on_response(tower_http::trace::DefaultOnResponse::new().level(log_level)))
                .with_state(state);
            
            // Parse address to listen on
            let port = std::env::var("API_PORT").unwrap_or_else(|_| "8081".to_string());
            let port: u16 = port.parse().expect("Invalid API_PORT value");
            info!("Starting API server on 0.0.0.0:{}", port);
            let addr: std::net::SocketAddr = ([0, 0, 0, 0], port).into();
            
            // Start the server
            let listener = tokio::net::TcpListener::bind(&addr).await.expect("Failed to bind to address");
            axum::serve(listener, app).with_graceful_shutdown(shutdown_signal()).await.expect("Server error");
        })
    };
    
    // Wait for the API server to finish
    api_handle.await?;
    
    info!("Shutting down");
    Ok(())
}

// Health check endpoint
async fn health_check(
    State(state): State<Arc<api_gateway::AppState>>,
) -> impl IntoResponse {
    let start_time = Instant::now();
    
    // Initialize status for each service
    let mut matching_engine_status = "unknown";
    let mut account_service_status = "unknown";
    let mut market_data_status = "unknown";
    let mut matching_engine_latency = 0;
    let mut account_service_latency = 0;
    let mut market_data_latency = 0;
    
    // Check if matching engine is responsive
    let me_start = Instant::now();
    matching_engine_status = match state.matching_engine.get_market_depth("BTC/USD", 1) {
        Ok(_) => "up",
        Err(_) => "down",
    };
    matching_engine_latency = me_start.elapsed().as_millis() as u64;
    
    // Check if account service is responsive
    let as_start = Instant::now();
    account_service_status = match state.account_service.get_account(Uuid::nil()).await {
        // Any response means the service is working, even NotFound for a nil UUID
        Ok(_) => "up",
        Err(common::error::Error::AccountNotFound(_)) => "up",
        Err(_) => "down",
    };
    account_service_latency = as_start.elapsed().as_millis() as u64;
    
    // Check if market data service is responsive
    let md_start = Instant::now();
    market_data_status = if state.market_data_service.get_ticker("BTC/USD").is_some() ||
                           state.market_data_service.get_all_tickers().len() > 0 {
        "up"
    } else {
        "down"
    };
    market_data_latency = md_start.elapsed().as_millis() as u64;
    
    // Overall status depends on all services
    let overall_status = if matching_engine_status == "up" && 
                           account_service_status == "up" && 
                           market_data_status == "up" {
        "healthy"
    } else {
        "degraded"
    };
    
    // Count available markets
    let available_markets = state.markets.len();
    let active_markets = state.markets.iter()
        .filter(|m| m.trading_enabled)
        .count();
    
    // Get system metrics
    let memory_usage = get_memory_usage_mb();
    let uptime = get_uptime_seconds();
    
    // Total response time for this health check
    let total_latency = start_time.elapsed().as_millis() as u64;
    
    // Build the health information JSON
    let health_info = serde_json::json!({
        "status": overall_status,
        "version": env!("CARGO_PKG_VERSION"),
        "timestamp": chrono::Utc::now().to_rfc3339(),
        "uptime_seconds": uptime,
        "services": {
            "matching_engine": {
                "status": matching_engine_status,
                "latency_ms": matching_engine_latency
            },
            "account_service": {
                "status": account_service_status,
                "latency_ms": account_service_latency
            },
            "market_data_service": {
                "status": market_data_status,
                "latency_ms": market_data_latency
            }
        },
        "markets": {
            "total": available_markets,
            "active": active_markets
        },
        "system": {
            "memory_usage_mb": memory_usage,
        },
        "health_check_latency_ms": total_latency
    });
    
    if overall_status == "healthy" {
        (axum::http::StatusCode::OK, Json(health_info))
    } else {
        (axum::http::StatusCode::SERVICE_UNAVAILABLE, Json(health_info))
    }
}

// Helper function to get uptime in seconds
fn get_uptime_seconds() -> u64 {
    let current_start = START_TIME.load(Ordering::Relaxed);
    if current_start == 0 {
        // First call, initialize start time
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        START_TIME.store(now, Ordering::Relaxed);
        return 0;
    }
    
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    
    now.saturating_sub(current_start)
}

// Helper function to get memory usage in MB
fn get_memory_usage_mb() -> u64 {
    #[cfg(target_os = "linux")]
    {
        use std::fs::File;
        use std::io::Read;
        
        if let Ok(mut file) = File::open("/proc/self/status") {
            let mut contents = String::new();
            if let Ok(_) = file.read_to_string(&mut contents) {
                if let Some(line) = contents.lines().find(|l| l.starts_with("VmRSS:")) {
                    if let Some(kb_str) = line.split_whitespace().nth(1) {
                        if let Ok(kb) = kb_str.parse::<u64>() {
                            return kb / 1024; // Convert KB to MB
                        }
                    }
                }
            }
        }
    }
    
    // Default if we can't get the actual usage or not on Linux
    0
}

/// Create demo data for testing
async fn create_demo_data(
    matching_engine: Arc<MatchingEngine>,
    account_service: Arc<AccountService>,
    market_data_service: Arc<MarketDataService>,
) -> Result<(), Box<dyn std::error::Error>> {
    // Create two demo accounts
    let alice = account_service.create_account().await?;
    let bob = account_service.create_account().await?;
    
    info!("Created demo accounts: Alice = {}, Bob = {}", alice.id, bob.id);
    
    // Add some funds to the accounts
    account_service.deposit(alice.id, "USD", dec!(100000)).await?;
    account_service.deposit(alice.id, "BTC", dec!(10)).await?;
    
    account_service.deposit(bob.id, "USD", dec!(100000)).await?;
    account_service.deposit(bob.id, "BTC", dec!(10)).await?;
    
    info!("Added funds to demo accounts");
    
    // Place some initial orders
    use common::model::order::{Order, Side, TimeInForce};
    
    // Alice places some buy orders
    let orders = vec![
        Order::new_limit(
            alice.id,
            "BTC/USD".to_string(),
            Side::Buy,
            dec!(20000),
            dec!(1),
            TimeInForce::GTC,
        ),
        Order::new_limit(
            alice.id,
            "BTC/USD".to_string(),
            Side::Buy,
            dec!(19500),
            dec!(1),
            TimeInForce::GTC,
        ),
        Order::new_limit(
            alice.id,
            "BTC/USD".to_string(),
            Side::Buy,
            dec!(19000),
            dec!(1),
            TimeInForce::GTC,
        ),
    ];
    
    for order in orders {
        account_service.reserve_for_order(&order).await?;
        let result = matching_engine.place_order(order)?;
        
        for trade in &result.trades {
            account_service.process_trade(trade).await?;
            market_data_service.process_trade(trade).await?;
        }
    }
    
    // Bob places some sell orders
    let orders = vec![
        Order::new_limit(
            bob.id,
            "BTC/USD".to_string(),
            Side::Sell,
            dec!(21000),
            dec!(1),
            TimeInForce::GTC,
        ),
        Order::new_limit(
            bob.id,
            "BTC/USD".to_string(),
            Side::Sell,
            dec!(21500),
            dec!(1),
            TimeInForce::GTC,
        ),
        Order::new_limit(
            bob.id,
            "BTC/USD".to_string(),
            Side::Sell,
            dec!(22000),
            dec!(1),
            TimeInForce::GTC,
        ),
    ];
    
    for order in orders {
        account_service.reserve_for_order(&order).await?;
        let result = matching_engine.place_order(order)?;
        
        for trade in &result.trades {
            account_service.process_trade(trade).await?;
            market_data_service.process_trade(trade).await?;
        }
    }
    
    // Place a matching order to generate a trade
    let matching_order = Order::new_limit(
        alice.id,
        "BTC/USD".to_string(),
        Side::Buy,
        dec!(21000),
        dec!(0.5),
        TimeInForce::GTC,
    );
    
    account_service.reserve_for_order(&matching_order).await?;
    let result = matching_engine.place_order(matching_order)?;
    
    for trade in &result.trades {
        account_service.process_trade(trade).await?;
        market_data_service.process_trade(trade).await?;
    }
    
    info!("Generated {} trades", result.trades.len());
    
    // Update order book in market data service
    if let Ok((bids, asks)) = matching_engine.get_market_depth("BTC/USD", 10) {
        market_data_service.update_order_book("BTC/USD", bids, asks).await?;
    }
    
    info!("Demo data created successfully");
    Ok(())
}

/// Graceful shutdown signal handler
async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("Failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("Failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }

    info!("Shutdown signal received, starting graceful shutdown");
}