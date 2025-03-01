use std::str::FromStr;
use async_trait::async_trait;
use chrono::Utc;
use sqlx::{PgPool, postgres::PgQueryResult};
use uuid::Uuid;

use common::decimal::Decimal;
use common::error::{Error, Result};
use common::model::{Market, OrderBook, OrderBookEntry, Trade};

use crate::models::MarketSummary;
use super::MarketRepository;

pub struct PostgresMarketRepository {
    pool: PgPool,
}

impl PostgresMarketRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl MarketRepository for PostgresMarketRepository {
    async fn create_market(&self, market: Market) -> Result<Market> {
        let result = sqlx::query!(
            r#"
            INSERT INTO markets (
                id, base_asset, quote_asset, min_price, max_price, 
                tick_size, min_quantity, max_quantity, step_size
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
            RETURNING id, base_asset, quote_asset, min_price, max_price, 
                    tick_size, min_quantity, max_quantity, step_size,
                    created_at, updated_at
            "#,
            market.id,
            market.base_asset,
            market.quote_asset,
            market.min_price.to_string(),
            market.max_price.to_string(),
            market.tick_size.to_string(),
            market.min_quantity.to_string(),
            market.max_quantity.to_string(),
            market.step_size.to_string()
        )
        .fetch_one(&self.pool)
        .await?;
        
        Ok(Market {
            id: result.id,
            base_asset: result.base_asset,
            quote_asset: result.quote_asset,
            min_price: Decimal::from_str(&result.min_price)?,
            max_price: Decimal::from_str(&result.max_price)?,
            tick_size: Decimal::from_str(&result.tick_size)?,
            min_quantity: Decimal::from_str(&result.min_quantity)?,
            max_quantity: Decimal::from_str(&result.max_quantity)?,
            step_size: Decimal::from_str(&result.step_size)?,
            created_at: result.created_at,
            updated_at: result.updated_at,
        })
    }

    async fn get_market(&self, market_id: &str) -> Result<Option<Market>> {
        let result = sqlx::query!(
            r#"
            SELECT id, base_asset, quote_asset, min_price, max_price, 
                 tick_size, min_quantity, max_quantity, step_size,
                 created_at, updated_at
            FROM markets
            WHERE id = $1
            "#,
            market_id
        )
        .fetch_optional(&self.pool)
        .await?;
        
        Ok(result.map(|r| Market {
            id: r.id,
            base_asset: r.base_asset,
            quote_asset: r.quote_asset,
            min_price: Decimal::from_str(&r.min_price).unwrap_or_default(),
            max_price: Decimal::from_str(&r.max_price).unwrap_or_default(),
            tick_size: Decimal::from_str(&r.tick_size).unwrap_or_default(),
            min_quantity: Decimal::from_str(&r.min_quantity).unwrap_or_default(),
            max_quantity: Decimal::from_str(&r.max_quantity).unwrap_or_default(),
            step_size: Decimal::from_str(&r.step_size).unwrap_or_default(),
            created_at: r.created_at,
            updated_at: r.updated_at,
        }))
    }

    async fn list_markets(&self) -> Result<Vec<Market>> {
        let results = sqlx::query!(
            r#"
            SELECT id, base_asset, quote_asset, min_price, max_price, 
                 tick_size, min_quantity, max_quantity, step_size,
                 created_at, updated_at
            FROM markets
            ORDER BY id
            "#
        )
        .fetch_all(&self.pool)
        .await?;
        
        Ok(results
            .into_iter()
            .map(|r| Market {
                id: r.id,
                base_asset: r.base_asset,
                quote_asset: r.quote_asset,
                min_price: Decimal::from_str(&r.min_price).unwrap_or_default(),
                max_price: Decimal::from_str(&r.max_price).unwrap_or_default(),
                tick_size: Decimal::from_str(&r.tick_size).unwrap_or_default(),
                min_quantity: Decimal::from_str(&r.min_quantity).unwrap_or_default(),
                max_quantity: Decimal::from_str(&r.max_quantity).unwrap_or_default(),
                step_size: Decimal::from_str(&r.step_size).unwrap_or_default(),
                created_at: r.created_at,
                updated_at: r.updated_at,
            })
            .collect())
    }

    async fn update_market_summary(&self, market_id: &str, summary: &MarketSummary) -> Result<()> {
        sqlx::query!(
            r#"
            INSERT INTO market_summaries (
                market_id, open_price, high_price, low_price, close_price,
                volume, updated_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            ON CONFLICT (market_id) 
            DO UPDATE SET
                open_price = $2,
                high_price = $3,
                low_price = $4,
                close_price = $5,
                volume = $6,
                updated_at = $7
            "#,
            market_id,
            summary.open_price.to_string(),
            summary.high_price.to_string(),
            summary.low_price.to_string(),
            summary.close_price.to_string(),
            summary.volume.to_string(),
            Utc::now()
        )
        .execute(&self.pool)
        .await?;
        
        Ok(())
    }

    async fn get_market_summary(&self, market_id: &str) -> Result<Option<MarketSummary>> {
        let result = sqlx::query!(
            r#"
            SELECT market_id, open_price, high_price, low_price, close_price,
                  volume, updated_at
            FROM market_summaries
            WHERE market_id = $1
            "#,
            market_id
        )
        .fetch_optional(&self.pool)
        .await?;
        
        Ok(result.map(|r| MarketSummary {
            market_id: r.market_id,
            open_price: Decimal::from_str(&r.open_price).unwrap_or_default(),
            high_price: Decimal::from_str(&r.high_price).unwrap_or_default(),
            low_price: Decimal::from_str(&r.low_price).unwrap_or_default(),
            close_price: Decimal::from_str(&r.close_price).unwrap_or_default(),
            volume: Decimal::from_str(&r.volume).unwrap_or_default(),
            updated_at: r.updated_at,
        }))
    }

    async fn save_trade(&self, trade: &Trade) -> Result<()> {
        sqlx::query!(
            r#"
            INSERT INTO trades (
                id, market_id, maker_order_id, taker_order_id, 
                price, quantity, executed_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            ON CONFLICT (id) DO NOTHING
            "#,
            trade.id,
            trade.market_id,
            trade.maker_order_id,
            trade.taker_order_id,
            trade.price.to_string(),
            trade.quantity.to_string(),
            trade.executed_at
        )
        .execute(&self.pool)
        .await?;
        
        Ok(())
    }

    async fn get_recent_trades(&self, market_id: &str, limit: usize) -> Result<Vec<Trade>> {
        let results = sqlx::query!(
            r#"
            SELECT id, market_id, maker_order_id, taker_order_id, 
                  price, quantity, executed_at
            FROM trades
            WHERE market_id = $1
            ORDER BY executed_at DESC
            LIMIT $2
            "#,
            market_id,
            limit as i64
        )
        .fetch_all(&self.pool)
        .await?;
        
        Ok(results
            .into_iter()
            .map(|r| Trade {
                id: r.id,
                market_id: r.market_id,
                maker_order_id: r.maker_order_id,
                taker_order_id: r.taker_order_id,
                price: Decimal::from_str(&r.price).unwrap_or_default(),
                quantity: Decimal::from_str(&r.quantity).unwrap_or_default(),
                executed_at: r.executed_at,
            })
            .collect())
    }

    async fn save_order_book(&self, market_id: &str, order_book: &OrderBook) -> Result<()> {
        // Convert order book to JSON
        let order_book_json = serde_json::to_value(order_book)?;
        
        sqlx::query!(
            r#"
            INSERT INTO order_books (market_id, data, updated_at)
            VALUES ($1, $2, $3)
            ON CONFLICT (market_id) 
            DO UPDATE SET
                data = $2,
                updated_at = $3
            "#,
            market_id,
            order_book_json,
            Utc::now()
        )
        .execute(&self.pool)
        .await?;
        
        Ok(())
    }

    async fn get_order_book(&self, market_id: &str) -> Result<Option<OrderBook>> {
        let result = sqlx::query!(
            r#"
            SELECT data
            FROM order_books
            WHERE market_id = $1
            "#,
            market_id
        )
        .fetch_optional(&self.pool)
        .await?;
        
        if let Some(row) = result {
            let order_book: OrderBook = serde_json::from_value(row.data)?;
            Ok(Some(order_book))
        } else {
            Ok(None)
        }
    }
}