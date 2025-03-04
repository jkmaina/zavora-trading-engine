//! API Gateway for the trading engine

mod api;
mod error;
mod ws;
mod config;

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH, Instant};

use axum::{
    routing::{get, post},
    Router,
    extract::State,
    response::IntoResponse,
    Json,
};
use clap::Parser;
use common::model::market::Market;
use dotenv::dotenv;
use tokio::net::TcpListener;
use tokio::signal;
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::{TraceLayer, DefaultMakeSpan, DefaultOnRequest, DefaultOnResponse};
use tracing::{info, Level, debug};
use tracing_subscriber::{EnvFilter, FmtSubscriber, fmt::format::FmtSpan};
use uuid::Uuid;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

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

/// API documentation
#[derive(OpenApi)]
#[openapi(
    paths(
        // Account routes
        api::account::create_account,
        api::account::get_account,
        api::account::get_balances,
        api::account::deposit,
        api::account::withdraw,
        // Market routes
        api::market::get_markets,
        api::market::get_order_book,
        api::market::get_ticker,
        api::market::get_tickers,
        api::market::get_trades,
        api::market::get_candles,
        // Order routes
        api::order::place_order,
        api::order::cancel_order,
        api::order::get_order,
        api::order::get_orders,
    ),
    components(
        schemas(
            // Account API
            api::account::CreateAccountRequest,
            api::account::DepositRequest,
            api::account::WithdrawRequest,
            common::model::account::Account,
            common::model::account::Balance,
            
            // Order API
            api::order::PlaceOrderRequest,
            api::order::OrderPlacementResult,
            api::order::OrdersQuery,
            common::model::order::Order,
            common::model::order::TimeInForce,
            common::model::order::Side,
            common::model::order::OrderType,
            common::model::trade::Trade,
            
            // Market API
            api::market::OrderBookQuery,
            api::market::OrderBookData,
            api::market::TradesQuery,
            api::market::MarketTradesData,
            api::market::CandlesQuery,
            api::market::MarketCandleData,
            market_data::Ticker,
            market_data::Candle,
            market_data::CandleInterval,
            common::model::market::Market,
            
            // Response models
            api::response::ApiResponse<common::model::account::Account>,
            api::response::ApiResponse<common::model::order::Order>, 
            api::response::ApiResponse<api::order::OrderPlacementResult>,
            api::response::ApiListResponse<common::model::market::Market>,
            api::response::ApiListResponse<common::model::order::Order>,
            api::response::ApiListResponse<common::model::account::Balance>,
            api::response::ApiListResponse<market_data::Ticker>,
            api::response::ResponseMetadata,
            api::response::PaginationMetadata
        )
    ),
    tags(
        (name = "account", description = "Account management endpoints"),
        (name = "market", description = "Market data endpoints"),
        (name = "order", description = "Order management endpoints"),
        (name = "system", description = "System endpoints")
    ),
    info(
        title = "Trading Engine API",
        version = "1.0.0",
        description = "API for the trading engine allowing account management, order placement, and market data access"
    )
)]
struct ApiDoc;

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
    
    // Initialize logging with debug level when DEBUG=1 env var is set
    let env = std::env::var("DEBUG").unwrap_or_else(|_| "0".to_string());
    let log_level = if env == "1" { Level::DEBUG } else { Level::INFO };
    
    let env_filter = EnvFilter::builder()
        .with_default_directive(log_level.into())
        .parse("tower_http=debug,api_gateway=debug")
        .unwrap();
    
    let subscriber = FmtSubscriber::builder()
        .with_env_filter(env_filter)
        .with_span_events(FmtSpan::NEW | FmtSpan::CLOSE)
        .finish();
        
    tracing::subscriber::set_global_default(subscriber)
        .expect("Failed to set tracing subscriber");
        
    debug!("Debug logging enabled");
    
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
    
    // Initialize service start time for uptime tracking
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    START_TIME.store(now, Ordering::Relaxed);
    
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
        // Health check endpoint
        .route("/health", get(health_check))
        
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
    
    // Set up Swagger UI
    let swagger_ui = SwaggerUi::new("/swagger-ui")
        .url("/api-docs/openapi.json", ApiDoc::openapi());
    
    // Combine all routes
    let app = Router::new()
        .nest("/api/v1", api_routes)
        .merge(ws_routes)
        .merge(swagger_ui)
        .layer(cors)
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(
                    DefaultMakeSpan::new()
                        .level(log_level)
                )
                .on_request(DefaultOnRequest::new().level(log_level))
                .on_response(DefaultOnResponse::new().level(log_level))
        )
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

// Static variable to track service start time
static START_TIME: AtomicU64 = AtomicU64::new(0);

// Health check endpoint
async fn health_check(
    State(state): State<Arc<AppState>>,
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