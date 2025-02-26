use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::sync::atomic::{AtomicU64, Ordering};

use chrono::Utc;
use common::decimal::{Price, Quantity};
use common::error::{Error, Result};
use common::model::order::{Order, OrderStatus, Side, OrderType, TimeInForce};
use common::model::trade::Trade;
use dashmap::DashMap;
use tracing::{debug, info};
use uuid::Uuid;

use crate::order_book::OrderBook;

/// Result of a matching operation
#[derive(Debug, Default)]
pub struct MatchingResult {
    /// The updated taker order
    pub taker_order: Option<Arc<Order>>,
    /// Maker orders that were matched against
    pub maker_orders: Vec<Arc<Order>>,
    /// Trades that were generated
    pub trades: Vec<Trade>,
}

/// The matching engine responsible for processing orders and generating trades
pub struct MatchingEngine {
    /// Map of market symbol to order book
    order_books: HashMap<String, Arc<RwLock<OrderBook>>>,
    /// Map of order ID to order
    orders: DashMap<Uuid, Arc<Order>>,
    /// Sequence number for order processing
    sequence: AtomicU64,
}

impl MatchingEngine {
    /// Create a new matching engine
    pub fn new() -> Self {
        Self {
            order_books: HashMap::new(),
            orders: DashMap::new(),
            sequence: AtomicU64::new(1),
        }
    }
    /// Register a new market
    pub fn register_market(&mut self, market: String) {
        if !self.order_books.contains_key(&market) {
            info!("Registering market: {}", market);
            let order_book = OrderBook::new(market.clone());
            self.order_books.insert(market, Arc::new(RwLock::new(order_book)));
        }
    }
    
    
    /// Get an order book for a market
    fn get_order_book(&self, market: &str) -> Result<Arc<RwLock<OrderBook>>> {
        self.order_books
            .get(market)
            .cloned()
            .ok_or_else(|| Error::Internal(format!("Market not found: {}", market)))
    }
    
    /// Get an order by ID
    pub fn get_order(&self, order_id: Uuid) -> Option<Arc<Order>> {
        self.orders.get(&order_id).map(|o| o.clone())
    }
    
    /// Place a new order
    pub fn place_order(&self, order: Order) -> Result<MatchingResult> {
        let market = order.market.clone();
        let order_book = self.get_order_book(&market)?;
        
        // Sequence number for this order
        let _seq = self.sequence.fetch_add(1, Ordering::SeqCst);
        
        // Clone and wrap the order for thread safety
        let order = Arc::new(order);
        
        // Store order in our index
        self.orders.insert(order.id, order.clone());
        
        // Process the order based on type
        match order.order_type {
            OrderType::Market => self.execute_market_order(order, order_book),
            OrderType::Limit => self.execute_limit_order(order, order_book),
        }
    }
    
    /// Execute a market order (immediate execution)
    fn execute_market_order(&self, order: Arc<Order>, order_book: Arc<RwLock<OrderBook>>) -> Result<MatchingResult> {
        let side = order.side;
        let mut result = MatchingResult::default();
        
        // Get exclusive access to the order book
        let mut order_book = order_book.write().unwrap();
        
        // Match against the opposite side of the book
        let (matched_order, matched_makers, trades) = match side {
            Side::Buy => {
                self.match_against_asks(order.clone(), &mut order_book)
            },
            Side::Sell => {
                self.match_against_bids(order.clone(), &mut order_book)
            }
        };
        
        result.taker_order = matched_order;
        result.maker_orders = matched_makers;
        result.trades = trades;
        
        // Since this is a market order, if it's not fully filled, the remainder is canceled
        if let Some(ref taker) = result.taker_order {
            if !taker.is_filled() {
                // TODO: In a real system, we'd update the order status to Canceled in the database
                debug!("Market order {} partially filled, canceling remainder", taker.id);
            }
        }
        
        Ok(result)
    }
    
    /// Execute a limit order
    fn execute_limit_order(&self, order: Arc<Order>, order_book: Arc<RwLock<OrderBook>>) -> Result<MatchingResult> {
        let side = order.side;
        let mut result = MatchingResult::default();
        
        // Get exclusive access to the order book
        let mut order_book = order_book.write().unwrap();
        
        // Check if this order can match immediately
        let price = order.price.expect("Limit orders must have a price");
        let can_match = match side {
            Side::Buy => order_book.best_ask().map_or(false, |ask| price >= ask),
            Side::Sell => order_book.best_bid().map_or(false, |bid| price <= bid),
        };
        
        if can_match {
            // Match against the opposite side of the book
            let (matched_order, matched_makers, trades) = match side {
                Side::Buy => {
                    self.match_against_asks(order.clone(), &mut order_book)
                },
                Side::Sell => {
                    self.match_against_bids(order.clone(), &mut order_book)
                }
            };
            
            result.taker_order = matched_order;
            result.maker_orders = matched_makers;
            result.trades = trades;
            
            // If the order wasn't fully filled and it's GTC, add the remainder to the book
            if let Some(ref taker) = result.taker_order {
                if !taker.is_filled() && taker.time_in_force == TimeInForce::GTC {
                    debug!("Adding remaining limit order to the book: {}", taker.id);
                    order_book.add_order(taker.clone());
                }
            }
        } else {
            // No immediate match, add to the book if GTC
            if order.time_in_force == TimeInForce::GTC {
                debug!("Adding limit order to the book: {}", order.id);
                order_book.add_order(order.clone());
            }
            
            result.taker_order = Some(order);
        }
        
        Ok(result)
    }
    
    /// Match an order against the ask side of the book
    fn match_against_asks(
        &self,
        taker: Arc<Order>,
        order_book: &mut OrderBook,
    ) -> (Option<Arc<Order>>, Vec<Arc<Order>>, Vec<Trade>) {
        let mut matched_makers = Vec::new();
        let mut trades = Vec::new();
        let mut taker_quantity = taker.remaining_quantity;
        let mut taker_filled = false;
        
        // Create a mutable clone of the taker order
        let mut taker_clone = Order {
            remaining_quantity: taker_quantity,
            filled_quantity: taker.filled_quantity,
            average_fill_price: taker.average_fill_price,
            status: taker.status,
            updated_at: taker.updated_at,
            ..taker.as_ref().clone()
        };
        
        // While we have quantity to fill and there are matching asks
        while taker_quantity > Quantity::ZERO {
            // Get the best ask
            let best_ask = match order_book.best_ask() {
                Some(price) => price,
                None => break, // No more asks to match against
            };
            
            // For market orders or if the limit price is acceptable
            if taker.order_type == OrderType::Market || 
               taker.price.map_or(false, |p| p >= best_ask) {
                // Get orders at this price level
                let asks = match order_book.asks().orders_at(best_ask) {
                    Some(orders) => orders.clone(), // Clone to avoid borrow checker issues
                    None => break, // This shouldn't happen but just in case
                };
                
                // Match against each order at this price level
                for maker in asks {
                    if taker_quantity <= Quantity::ZERO {
                        break;
                    }
                    
                    // Skip orders that are already filled
                    if maker.is_filled() {
                        continue;
                    }
                    
                    // Calculate match quantity
                    let match_quantity = taker_quantity.min(maker.remaining_quantity);
                    
                    // Create the trade
                    let trade = Trade::new(
                        taker.market.clone(),
                        best_ask,
                        match_quantity,
                        taker.id,
                        maker.id,
                        taker.user_id,
                        maker.user_id,
                        Side::Buy, // Taker is buying, so taker side is Buy
                    );
                    
                    // Update taker
                    taker_quantity -= match_quantity;
                    taker_clone.remaining_quantity = taker_quantity;
                    taker_clone.filled_quantity += match_quantity;
                    
                    // Calculate new average fill price
                    let total_filled_amount = taker_clone.average_fill_price
                        .map_or(Quantity::ZERO, |p| p * taker_clone.filled_quantity);
                    let match_amount = best_ask * match_quantity;
                    let new_total_amount = total_filled_amount + match_amount;
                    taker_clone.average_fill_price = Some(new_total_amount / taker_clone.filled_quantity);
                    
                    // Update maker (in a real system, this would be persisted)
                    // For now we just track them for the result
                    matched_makers.push(maker.clone());
                    
                    // Add the trade to the result
                    trades.push(trade);
                    
                    // Update the order book's last price
                    order_book.set_last_price(best_ask);
                    
                    // Remove filled maker orders from the book
                    if maker.remaining_quantity == match_quantity {
                        order_book.remove_order(maker.id, Side::Sell);
                    }
                    
                    // Check if taker is filled
                    if taker_quantity == Quantity::ZERO {
                        taker_filled = true;
                        break;
                    }
                }
            } else {
                // Limit price not acceptable
                break;
            }
        }
        
        // Update taker status
        if taker_filled {
            taker_clone.status = OrderStatus::Filled;
        } else if taker_clone.filled_quantity > Quantity::ZERO {
            taker_clone.status = OrderStatus::PartiallyFilled;
        }
        taker_clone.updated_at = Utc::now();
        
        // Return the result
        let updated_taker = Arc::new(taker_clone);
        
        // Update the order in our index
        self.orders.insert(updated_taker.id, updated_taker.clone());
        
        (Some(updated_taker), matched_makers, trades)
    }
    
    /// Match an order against the bid side of the book
    fn match_against_bids(
        &self,
        taker: Arc<Order>,
        order_book: &mut OrderBook,
    ) -> (Option<Arc<Order>>, Vec<Arc<Order>>, Vec<Trade>) {
        let mut matched_makers = Vec::new();
        let mut trades = Vec::new();
        let mut taker_quantity = taker.remaining_quantity;
        let mut taker_filled = false;
        
        // Create a mutable clone of the taker order
        let mut taker_clone = Order {
            remaining_quantity: taker_quantity,
            filled_quantity: taker.filled_quantity,
            average_fill_price: taker.average_fill_price,
            status: taker.status,
            updated_at: taker.updated_at,
            ..taker.as_ref().clone()
        };
        
        // While we have quantity to fill and there are matching bids
        while taker_quantity > Quantity::ZERO {
            // Get the best bid
            let best_bid = match order_book.best_bid() {
                Some(price) => price,
                None => break, // No more bids to match against
            };
            
            // For market orders or if the limit price is acceptable
            if taker.order_type == OrderType::Market || 
               taker.price.map_or(false, |p| p <= best_bid) {
                // Get orders at this price level
                let bids = match order_book.bids().orders_at(best_bid) {
                    Some(orders) => orders.clone(), // Clone to avoid borrow checker issues
                    None => break, // This shouldn't happen but just in case
                };
                
                // Match against each order at this price level
                for maker in bids {
                    if taker_quantity <= Quantity::ZERO {
                        break;
                    }
                    
                    // Skip orders that are already filled
                    if maker.is_filled() {
                        continue;
                    }
                    
                    // Calculate match quantity
                    let match_quantity = taker_quantity.min(maker.remaining_quantity);
                    
                    // Create the trade
                    let trade = Trade::new(
                        taker.market.clone(),
                        best_bid,
                        match_quantity,
                        maker.id,
                        taker.id,
                        maker.user_id,
                        taker.user_id,
                        Side::Sell, // Taker is selling, so taker side is Sell
                    );
                    
                    // Update taker
                    taker_quantity -= match_quantity;
                    taker_clone.remaining_quantity = taker_quantity;
                    taker_clone.filled_quantity += match_quantity;
                    
                    // Calculate new average fill price
                    let total_filled_amount = taker_clone.average_fill_price
                        .map_or(Quantity::ZERO, |p| p * taker_clone.filled_quantity);
                    let match_amount = best_bid * match_quantity;
                    let new_total_amount = total_filled_amount + match_amount;
                    taker_clone.average_fill_price = Some(new_total_amount / taker_clone.filled_quantity);
                    
                    // Update maker (in a real system, this would be persisted)
                    // For now we just track them for the result
                    matched_makers.push(maker.clone());
                    
                    // Add the trade to the result
                    trades.push(trade);
                    
                    // Update the order book's last price
                    order_book.set_last_price(best_bid);
                    
                    // Remove filled maker orders from the book
                    if maker.remaining_quantity == match_quantity {
                        order_book.remove_order(maker.id, Side::Buy);
                    }
                    
                    // Check if taker is filled
                    if taker_quantity == Quantity::ZERO {
                        taker_filled = true;
                        break;
                    }
                }
            } else {
                // Limit price not acceptable
                break;
            }
        }
        
        // Update taker status
        if taker_filled {
            taker_clone.status = OrderStatus::Filled;
        } else if taker_clone.filled_quantity > Quantity::ZERO {
            taker_clone.status = OrderStatus::PartiallyFilled;
        }
        taker_clone.updated_at = Utc::now();
        
        // Return the result
        let updated_taker = Arc::new(taker_clone);
        
        // Update the order in our index
        self.orders.insert(updated_taker.id, updated_taker.clone());
        
        (Some(updated_taker), matched_makers, trades)
    }
    
    /// Cancel an order
    pub fn cancel_order(&self, order_id: Uuid) -> Result<Arc<Order>> {
        // Find the order
        let order = self.orders.get(&order_id).ok_or_else(|| {
            Error::OrderNotFound(format!("Order not found: {}", order_id))
        })?;
        
        // Only active orders can be canceled
        if !order.is_active() {
            return Err(Error::InvalidOrder(
                format!("Order {} cannot be canceled: status is {:?}", order_id, order.status)
            ));
        }
        
        // Get the order book
        let order_book_lock = self.get_order_book(&order.market)?;
        let mut order_book = order_book_lock.write().unwrap();
        
        // Remove from the book
        order_book.remove_order(order_id, order.side);
        
        // Create a canceled version of the order
        let mut canceled_order = order.as_ref().clone();
        canceled_order.status = OrderStatus::Cancelled;
        canceled_order.updated_at = Utc::now();
        
        // Update in our index
        let canceled_order = Arc::new(canceled_order);
        self.orders.insert(order_id, canceled_order.clone());
        
        Ok(canceled_order)
    }
    
    /// Get market depth
    pub fn get_market_depth(&self, market: &str, limit: usize) -> Result<(Vec<(Price, Quantity)>, Vec<(Price, Quantity)>)> {
        let order_book_lock = self.get_order_book(market)?;
        let order_book = order_book_lock.read().unwrap();
        
        let bids = order_book.bid_levels(limit);
        let asks = order_book.ask_levels(limit);
        
        Ok((bids, asks))
    }
    
}    