//! API Gateway for the trading engine

mod api;
mod error;
mod ws;
mod config;

use std::sync::Arc;

use axum::{
    routing::{get, post},
    Router,
};
use clap::Parser;
use common::model::market::Market;
use dotenv::dotenv;
use tokio::net::TcpListener;
use tokio::signal;
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;

use account_service::AccountService;
use market_data::MarketDataService;
use matching_engine::MatchingEngine;

use crate::api::{
    account::{create_account, get_account, get_balances, deposit, withdraw},
    market::{get_markets, get_order_book, get_ticker, get_tickers, get_trades, get_candles},
    order::{place_order, cancel_order, get_order, get_orders},
};
use crate::config::AppConfig;
use crate::ws::handler::ws_handler;

/// Trading engine API server
#[derive(Parser, Debug)]
#[clap(author, version, about)]
struct Args {
    /// Listening address
    #[clap(short, long, default_value = "127.0.0.1:8080")]
    addr: String,
}

#[tokio::main]
async fn main() -> std::io::Result<()> {
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
    let _config = AppConfig::new();
    let mut matching_engine = MatchingEngine::new();
    let account_service = Arc::new(AccountService::new());
    let market_data_service = Arc::new(MarketDataService::new());
    
    // Register markets
    let btc_usd = Market {
        symbol: "BTC/USD".to_string(),
        base_asset: "BTC".to_string(),
        quote_asset: "USD".to_string(),
        price_tick: rust_decimal_macros::dec!(0.01),
        quantity_step: rust_decimal_macros::dec!(0.0001),
        min_order_size: rust_decimal_macros::dec!(10.0),
        max_price_deviation: 10.0,
        trading_enabled: true,
    };
    
    matching_engine.register_market(btc_usd.symbol.clone());
    
    // Create app state
    let state = Arc::new(AppState {
        matching_engine: Arc::new(matching_engine),
        account_service,
        market_data_service,
        markets: vec![btc_usd],
    });
    
    // Set up CORS
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);
    
    // Set up API routes
    let api_routes = Router::new()
        // Account routes
        .route("/accounts", post(create_account))
        .route("/accounts/:id", get(get_account))
        .route("/accounts/:id/balances", get(get_balances))
        .route("/accounts/:id/deposit", post(deposit))
        .route("/accounts/:id/withdraw", post(withdraw))
        
        // Market routes
        .route("/markets", get(get_markets))
        .route("/markets/:market/order-book", get(get_order_book))
        .route("/markets/:market/ticker", get(get_ticker))
        .route("/markets/:market/trades", get(get_trades))
        .route("/markets/:market/candles", get(get_candles))
        .route("/markets/tickers", get(get_tickers))        
        
        // Order routes
        .route("/orders", post(place_order))
        .route("/orders/:id", get(get_order))
        .route("/orders/:id", post(cancel_order))
        .route("/accounts/:id/orders", get(get_orders));
    
    
    // Set up websocket route
    let ws_routes = Router::new()
        .route("/ws", get(ws_handler));
    
    // Combine all routes
    let app = Router::new()
        .nest("/api/v1", api_routes)
        .merge(ws_routes)
        .layer(cors)
        .layer(TraceLayer::new_for_http())
        .with_state(state);
    
    // Start the server
    let addr: std::net::SocketAddr = args.addr.parse().expect("Invalid address");
    let listener = TcpListener::bind(addr).await?;
    info!("Listening on {}", addr);
    
    // Run until interrupt signal
    axum::serve(listener, app).with_graceful_shutdown(shutdown_signal()).await?;
    
    Ok(())
}

/// App state shared across handlers
pub struct AppState {
    /// Matching engine
    pub matching_engine: Arc<MatchingEngine>,
    /// Account service
    pub account_service: Arc<AccountService>,
    /// Market data service
    pub market_data_service: Arc<MarketDataService>,
    /// Available markets
    pub markets: Vec<Market>,
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