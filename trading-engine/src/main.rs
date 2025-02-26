//! Trading engine integration module

use std::sync::Arc;

use clap::Parser;
use common::model::market::Market;
use dotenv::dotenv;
use rust_decimal_macros::dec;
use tokio::signal;
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;
use account_service::AccountService;
use market_data::MarketDataService;
use matching_engine::MatchingEngine;

/// Command line arguments
#[derive(Parser, Debug)]
#[clap(author, version, about)]
struct Args {
    /// Run with demo data
    #[clap(short, long)]
    demo: bool,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::fmt::init();
    
    info!("Starting Zavora Trading Engine...");

    // Load environment variables
    dotenv().ok();
    
    // Parse command line arguments
    let args = Args::parse();
    
    // Initialize logging
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .finish();
    tracing::subscriber::set_global_default(subscriber)
        .expect("Failed to set tracing subscriber");
    
    // Initialize services
    let mut matching_engine = MatchingEngine::new();
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
                .layer(tower_http::trace::TraceLayer::new_for_http())
                .with_state(state);
            
            // Parse address to listen on
            info!("Starting API server on 0.0.0.0:8080");
            let addr: std::net::SocketAddr = ([0, 0, 0, 0], 8080).into();
            
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
async fn health_check() -> impl axum::response::IntoResponse {
    axum::http::StatusCode::OK
}

/// Create demo data for testing
async fn create_demo_data(
    matching_engine: Arc<MatchingEngine>,
    account_service: Arc<AccountService>,
    market_data_service: Arc<MarketDataService>,
) -> Result<(), Box<dyn std::error::Error>> {
    // Create two demo accounts
    let alice = account_service.create_account();
    let bob = account_service.create_account();
    
    info!("Created demo accounts: Alice = {}, Bob = {}", alice.id, bob.id);
    
    // Add some funds to the accounts
    account_service.deposit(alice.id, "USD", dec!(100000))?;
    account_service.deposit(alice.id, "BTC", dec!(10))?;
    
    account_service.deposit(bob.id, "USD", dec!(100000))?;
    account_service.deposit(bob.id, "BTC", dec!(10))?;
    
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
        account_service.reserve_for_order(&order)?;
        let result = matching_engine.place_order(order)?;
        
        for trade in &result.trades {
            account_service.process_trade(trade)?;
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
        account_service.reserve_for_order(&order)?;
        let result = matching_engine.place_order(order)?;
        
        for trade in &result.trades {
            account_service.process_trade(trade)?;
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
    
    account_service.reserve_for_order(&matching_order)?;
    let result = matching_engine.place_order(matching_order)?;
    
    for trade in &result.trades {
        account_service.process_trade(trade)?;
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