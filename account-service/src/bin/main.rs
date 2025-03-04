use account_service::{AccountService, AccountServiceConfig};
use clap::{Parser, Subcommand};
use tokio::signal;
use tracing::{info, error};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

/// Account Service CLI
#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// Set the log level
    #[arg(short, long, default_value = "info")]
    log_level: String,
    
    /// Commands
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Start the account service
    Start {
        /// Database URL
        #[arg(short, long)]
        database_url: Option<String>,
        
        /// Database pool size
        #[arg(short, long)]
        pool_size: Option<u32>,
        
        /// Enable transaction logging
        #[arg(short, long)]
        transaction_logging: bool,
    },
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Parse command line arguments
    let cli = Cli::parse();
    
    // Initialize logging
    let _log_level = cli.log_level.parse::<tracing::Level>().unwrap_or(tracing::Level::INFO);
    
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(format!("account_service={}", cli.log_level)))
        .with(tracing_subscriber::fmt::layer())
        .init();
    
    // Process commands
    match cli.command {
        Commands::Start { database_url, pool_size, transaction_logging } => {
            // Create config using provided values or env vars
            let config = if let Some(url) = database_url {
                let pool_size = pool_size.unwrap_or(5);
                AccountServiceConfig::new(url, pool_size, transaction_logging)
            } else {
                AccountServiceConfig::from_env()
            };
            
            // Print config (except database password)
            info!(
                "Starting account service with database pool size: {}, transaction logging: {}",
                config.db_pool_size, config.transaction_logging
            );
            
            // Initialize service
            let _service = AccountService::with_config(&config).await?;
            
            // Wait for ctrl-c
            info!("Account service started. Press Ctrl+C to stop.");
            match signal::ctrl_c().await {
                Ok(()) => {
                    info!("Shutting down account service...");
                },
                Err(err) => {
                    error!("Error waiting for Ctrl+C: {}", err);
                }
            }
        }
    }
    
    Ok(())
}