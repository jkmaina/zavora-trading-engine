mod postgres;

use std::sync::Arc;
use async_trait::async_trait;
use sqlx::PgPool;

use common::decimal::Decimal;
use common::error::Result;
use common::model::{Market, OrderBook, Trade};

use crate::models::MarketSummary;

#[async_trait]
pub trait MarketRepository: Send + Sync {
    async fn create_market(&self, market: Market) -> Result<Market>;
    async fn get_market(&self, market_id: &str) -> Result<Option<Market>>;
    async fn list_markets(&self) -> Result<Vec<Market>>;
    async fn update_market_summary(&self, market_id: &str, summary: &MarketSummary) -> Result<()>;
    async fn get_market_summary(&self, market_id: &str) -> Result<Option<MarketSummary>>;
    async fn save_trade(&self, trade: &Trade) -> Result<()>;
    async fn get_recent_trades(&self, market_id: &str, limit: usize) -> Result<Vec<Trade>>;
    async fn save_order_book(&self, market_id: &str, order_book: &OrderBook) -> Result<()>;
    async fn get_order_book(&self, market_id: &str) -> Result<Option<OrderBook>>;
}

pub fn create_repository(pool: PgPool) -> Arc<dyn MarketRepository> {
    Arc::new(postgres::PostgresMarketRepository::new(pool))
}