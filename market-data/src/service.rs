//! Market data service implementation

use std::sync::Arc;

use chrono::Utc;
use common::decimal::{Price, Quantity};
use common::error::Result;
use common::model::trade::Trade;
use dashmap::DashMap;

use crate::channel::{MarketDataChannel, Topic};
use crate::models::{
    MarketDepth, OrderBookUpdate, PriceLevel, TradeMessage, 
    Ticker, MarketSummary, Candle, CandleInterval,
};

/// Market data service for providing real-time market data
pub struct MarketDataService {
    /// Market data channel
    channel: Arc<MarketDataChannel>,
    /// Latest market depths
    market_depths: DashMap<String, MarketDepth>,
    /// Latest tickers
    tickers: DashMap<String, Ticker>,
    /// Market summaries
    _market_summaries: DashMap<String, MarketSummary>,
    /// Recent trades by market
    recent_trades: DashMap<String, Vec<TradeMessage>>,
    /// Price candles by market and interval
    candles: DashMap<(String, CandleInterval), Vec<Candle>>,
}

impl MarketDataService {
    /// Create a new market data service
    pub fn new() -> Self {
        Self {
            channel: Arc::new(MarketDataChannel::new()),
            market_depths: DashMap::new(),
            tickers: DashMap::new(),
            _market_summaries: DashMap::new(),
            recent_trades: DashMap::new(),
            candles: DashMap::new(),
        }
    }
    
    /// Get the market data channel
    pub fn channel(&self) -> Arc<MarketDataChannel> {
        self.channel.clone()
    }
    
    /// Update order book
    pub async fn update_order_book(&self, market: &str, bids: Vec<(Price, Quantity)>, asks: Vec<(Price, Quantity)>) -> Result<()> {
        let timestamp = Utc::now();
        
        // Convert to price levels
        let bids = bids.into_iter()
            .map(|(price, quantity)| PriceLevel { price, quantity })
            .collect();
        let asks = asks.into_iter()
            .map(|(price, quantity)| PriceLevel { price, quantity })
            .collect();
        
        // Create market depth
        let market_depth = MarketDepth {
            market: market.to_string(),
            timestamp,
            bids,
            asks,
        };
        
        // Store latest market depth
        self.market_depths.insert(market.to_string(), market_depth.clone());
        
        // Create order book update
        let update = OrderBookUpdate {
            market: market.to_string(),
            timestamp,
            bids: market_depth.bids.clone(),
            asks: market_depth.asks.clone(),
        };
        
        // Publish update
        self.channel.publish(Topic::OrderBook(market.to_string()), update).await;
        
        // Update ticker
        self.update_ticker_from_order_book(market, &market_depth).await?;
        
        Ok(())
    }
    
    /// Process a new trade
    pub async fn process_trade(&self, trade: &Trade) -> Result<()> {
        let market = &trade.market;
        
        // Convert to trade message
        let trade_message = TradeMessage::from(trade);
        
        // Store recent trade
        let mut recent_trades = self.recent_trades
            .entry(market.clone())
            .or_insert_with(|| Vec::with_capacity(100));
        
        recent_trades.push(trade_message.clone());
        
        // Keep only last 100 trades
        if recent_trades.len() > 100 {
            recent_trades.remove(0);
        }
        
        // Publish trade
        self.channel.publish(Topic::Trades(market.clone()), trade_message).await;
        
        // Update candles
        self.update_candles(trade).await?;
        
        Ok(())
    }
    
    /// Update ticker from order book
    async fn update_ticker_from_order_book(&self, market: &str, depth: &MarketDepth) -> Result<()> {
        // Get existing ticker or create new one
        let mut ticker = self.tickers
            .entry(market.to_string())
            .or_insert_with(|| Ticker {
                market: market.to_string(),
                bid: None,
                ask: None,
                last: None,
                change_24h: None,
                change_24h_percent: None,
                high_24h: None,
                low_24h: None,
                volume_24h: None,
                quote_volume_24h: None,
                timestamp: Utc::now(),
            })
            .clone();
        
        // Update bid and ask
        ticker.bid = depth.bids.first().map(|level| level.price);
        ticker.ask = depth.asks.first().map(|level| level.price);
        ticker.timestamp = Utc::now();
        
        // Store updated ticker
        self.tickers.insert(market.to_string(), ticker.clone());
        
        // Publish ticker update
        self.channel.publish(Topic::Ticker(market.to_string()), ticker).await;
        
        Ok(())
    }
    
    /// Update ticker from trade
    async fn _update_ticker_from_trade(&self, trade: &Trade) -> Result<()> {
        let market = &trade.market;
        
        // Get existing ticker or create new one
        let mut ticker = self.tickers
            .entry(market.clone())
            .or_insert_with(|| Ticker {
                market: market.clone(),
                bid: None,
                ask: None,
                last: None,
                change_24h: None,
                change_24h_percent: None,
                high_24h: None,
                low_24h: None,
                volume_24h: None,
                quote_volume_24h: None,
                timestamp: Utc::now(),
            })
            .clone();
        
        // Update last price
        ticker.last = Some(trade.price);
        
        // Update 24h high/low
        if let Some(high) = ticker.high_24h {
            if trade.price > high {
                ticker.high_24h = Some(trade.price);
            }
        } else {
            ticker.high_24h = Some(trade.price);
        }
        
        if let Some(low) = ticker.low_24h {
            if trade.price < low {
                ticker.low_24h = Some(trade.price);
            }
        } else {
            ticker.low_24h = Some(trade.price);
        }
        
        // Update timestamp
        ticker.timestamp = Utc::now();
        
        // Store updated ticker
        self.tickers.insert(market.clone(), ticker.clone());
        
        // Publish ticker update
        self.channel.publish(Topic::Ticker(market.clone()), ticker).await;
        
        Ok(())
    }
    
    /// Update candles from trade
    async fn update_candles(&self, trade: &Trade) -> Result<()> {
        // For MVP, just update 1 minute candles
        self.update_candle_interval(trade, CandleInterval::Minute1).await?;
        
        Ok(())
    }
    
    /// Update candle for a specific interval
    async fn update_candle_interval(&self, trade: &Trade, interval: CandleInterval) -> Result<()> {
        let market = &trade.market;
        let trade_time = trade.created_at;
        
        // Calculate candle start time
        let interval_secs = interval.duration_secs();
        let timestamp_secs = trade_time.timestamp();
        let candle_start_secs = (timestamp_secs / interval_secs) * interval_secs;
        let candle_start = chrono::DateTime::from_timestamp(candle_start_secs, 0)
            .unwrap_or(trade_time);
        let candle_end = chrono::DateTime::from_timestamp(candle_start_secs + interval_secs, 0)
            .unwrap_or(trade_time);
        
        // Get candles for this market and interval
        let key = (market.clone(), interval);
        let mut candles = self.candles
            .entry(key.clone())  // Clone the key here
            .or_insert_with(Vec::new)
            .clone();
        
        // Check if current candle exists
        if let Some(current_candle) = candles.iter_mut().find(|c| c.open_time == candle_start) {
            // Update existing candle
            current_candle.high = current_candle.high.max(trade.price);
            current_candle.low = current_candle.low.min(trade.price);
            current_candle.close = trade.price;
            current_candle.volume += trade.quantity;
            current_candle.quote_volume += trade.price * trade.quantity;
            current_candle.trades += 1;
        } else {
            // Create new candle
            let new_candle = Candle {
                market: market.clone(),
                interval,
                open_time: candle_start,
                close_time: candle_end,
                open: trade.price,
                high: trade.price,
                low: trade.price,
                close: trade.price,
                volume: trade.quantity,
                quote_volume: trade.price * trade.quantity,
                trades: 1,
            };
            
            candles.push(new_candle);
            
            // Sort candles by time
            candles.sort_by(|a, b| a.open_time.cmp(&b.open_time));
            
            // Keep only last 1000 candles
            if candles.len() > 1000 {
                let skip_count = candles.len().saturating_sub(1000);
                candles = candles.iter().skip(skip_count).cloned().collect();
            }
        }
        
        // Store updated candles
        self.candles.insert(key, candles);
        
        Ok(())
    }
    
    /// Get market depth
    pub fn get_market_depth(&self, market: &str) -> Option<MarketDepth> {
        self.market_depths.get(market).map(|d| d.clone())
    }
    
    /// Get ticker
    pub fn get_ticker(&self, market: &str) -> Option<Ticker> {
        self.tickers.get(market).map(|t| t.clone())
    }
    
    /// Get all tickers
    pub fn get_all_tickers(&self) -> Vec<Ticker> {
        self.tickers.iter().map(|t| t.clone()).collect()
    }
    
    /// Get recent trades
    pub fn get_recent_trades(&self, market: &str, limit: usize) -> Vec<TradeMessage> {
        self.recent_trades
            .get(market)
            .map(|trades| {
                let mut result = trades.clone();
                result.sort_by(|a, b| b.timestamp.cmp(&a.timestamp)); // Newest first
                result.truncate(limit);
                result
            })
            .unwrap_or_default()
    }
    
    /// Get candles
    pub fn get_candles(&self, market: &str, interval: CandleInterval, limit: usize) -> Vec<Candle> {
        self.candles
            .get(&(market.to_string(), interval))
            .map(|candles| {
                let mut result = candles.clone();
                result.sort_by(|a, b| b.open_time.cmp(&a.open_time)); // Newest first
                result.truncate(limit);
                result
            })
            .unwrap_or_default()
    }
}