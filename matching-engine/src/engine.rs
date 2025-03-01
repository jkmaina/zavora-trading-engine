use std::sync::{Arc, RwLock};

use chrono::Utc;
use common::decimal::{Price, Quantity};
use common::error::{Error, Result};
use common::model::order::{Order, Status, Side, OrderType, TimeInForce};
use common::model::trade::Trade;
use dashmap::DashMap;
use tracing::{debug, info};
use uuid::Uuid;

use crate::order_book::{OrderBook, OrderBookSide};

/// Result of a matching operation
#[derive(Debug, Default)]
pub struct MatchingResult {
    /// The updated taker order
    pub taker_order: Option<Arc<Order>>,
    /// Maker orders that were matched
    pub maker_orders: Vec<Arc<Order>>,
    /// Trades that were generated
    pub trades: Vec<Trade>,
}

/// The matching engine responsible for processing orders and generating trades
pub struct MatchingEngine {
    /// Map of market symbols to order books
    order_books: DashMap<String, Arc<RwLock<OrderBook>>>,
}

impl MatchingEngine {
    /// Create a new matching engine
    pub fn new() -> Self {
        Self {
            order_books: DashMap::new(),
        }
    }
    
    /// Register a new market
    pub fn register_market(&self, market: String) {
        info!("Registering market: {}", market);
        self.order_books.insert(market.clone(), Arc::new(RwLock::new(OrderBook::new(market))));
    }
    
    /// Get an order by ID
    pub fn get_order(&self, order_id: Uuid) -> Option<Arc<Order>> {
        // Search in all order books
        for book_entry in self.order_books.iter() {
            let book = book_entry.value().read().unwrap();
            
            // For now, we'll scan the bids and asks for the order
            // In a real system, we'd have a global order map for efficient lookup
            let bids = book.bids().price_levels(100);
            for (price, _) in bids {
                if let Some(orders) = book.bids().orders_at(price) {
                    if let Some(order) = orders.iter().find(|o| o.id == order_id) {
                        return Some(order.clone());
                    }
                }
            }
            
            let asks = book.asks().get_price_levels(100);
            for (price, _) in asks {
                if let Some(orders) = book.asks().orders_at(price) {
                    if let Some(order) = orders.iter().find(|o| o.id == order_id) {
                        return Some(order.clone());
                    }
                }
            }
        }
        
        None
    }
    
    /// Cancel an order
    pub fn cancel_order(&self, order_id: Uuid) -> Result<Arc<Order>> {
        // First, find the order
        let original_order = match self.get_order(order_id) {
            Some(order) => order,
            None => return Err(Error::OrderNotFound(format!("Order not found: {}", order_id))),
        };
        
        // Find the order book for this market
        if let Some(book_entry) = self.order_books.get(&original_order.market) {
            let mut book = book_entry.write().unwrap();
            
            // Remove the order from the book
            if let Some(order) = book.remove_order(order_id, original_order.side) {
                // Create a canceled version of the order
                let canceled_order = Order {
                    status: Status::Cancelled,
                    updated_at: Utc::now(),
                    ..(*order).clone()
                };
                
                return Ok(Arc::new(canceled_order));
            }
        }
        
        Err(Error::OrderNotFound(format!("Order not found in book: {}", order_id)))
    }
    
    /// Get market depth
    pub fn get_market_depth(&self, market: &str, limit: usize) -> Result<(Vec<(Price, Quantity)>, Vec<(Price, Quantity)>)> {
        if let Some(book_entry) = self.order_books.get(market) {
            let book = book_entry.read().unwrap();
            
            // Get bid and ask levels
            let bids = book.bids().price_levels(limit);
            let asks = book.asks().price_levels(limit);
            
            Ok((bids, asks))
        } else {
            Err(Error::MarketNotFound(format!("Market not found: {}", market)))
        }
    }
    
    /// Process an incoming order
    pub fn place_order(&self, order: Order) -> Result<MatchingResult> {
        // Check if we have an order book for this market
        let order_book = match self.order_books.get(&order.market) {
            Some(ob) => ob.clone(),
            None => {
                return Err(Error::MarketNotFound(format!("Market not found: {}", order.market)));
            }
        };
        
        // Clone the order into an Arc for thread-safe sharing
        let order = Arc::new(order);
        
        // Execute the order based on type
        match order.order_type {
            OrderType::Market => {
                debug!("Processing market order: {}", order.id);
                self.execute_market_order(order, order_book)
            },
            OrderType::Limit => {
                debug!("Processing limit order: {}", order.id);
                self.execute_limit_order(order, order_book)
            }
        }
    }
    
    /// Execute a market order
    fn execute_market_order(&self, order: Arc<Order>, order_book: Arc<RwLock<OrderBook>>) -> Result<MatchingResult> {
        let side = order.side;
        let mut result = MatchingResult::default();
        
        // Get exclusive access to the order book
        let mut order_book = order_book.write().unwrap();
        
        // Check if the order book is empty on the opposite side
        let is_empty = match side {
            Side::Buy => order_book.best_ask().is_none(),
            Side::Sell => order_book.best_bid().is_none(),
        };
        
        if is_empty {
            return Err(Error::ValidationError(format!(
                "Cannot execute market {} order, no liquidity", 
                if side == Side::Buy { "buy" } else { "sell" }
            )));
        }
        
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
            
            // For limit orders, check if the price is acceptable
            if taker.order_type == OrderType::Limit {
                let limit_price = taker.price.unwrap();
                if limit_price < best_ask {
                    break; // Best ask is higher than our limit price
                }
            }
            
            // Get the first maker order at the best ask price
            if let Some(maker) = order_book.get_first_ask_order(best_ask) {
                // Calculate the match quantity
                let match_quantity = Quantity::min(taker_quantity, maker.remaining_quantity);
                
                // Create a trade
                let trade = self.create_trade(
                    best_ask,
                    match_quantity,
                    &taker.market,
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
        }
        
        // Update taker status
        if taker_filled {
            taker_clone.status = Status::Filled;
        } else if taker_clone.filled_quantity > Quantity::ZERO {
            taker_clone.status = Status::PartiallyFilled;
        }
        
        taker_clone.updated_at = Utc::now();
        
        // Return updated taker order and trades
        (Some(Arc::new(taker_clone)), matched_makers, trades)
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
            
            // For limit orders, check if the price is acceptable
            if taker.order_type == OrderType::Limit {
                let limit_price = taker.price.unwrap();
                if limit_price > best_bid {
                    break; // Best bid is lower than our limit price
                }
            }
            
            // Get the first maker order at the best bid price
            if let Some(maker) = order_book.get_first_bid_order(best_bid) {
                // Calculate the match quantity
                let match_quantity = Quantity::min(taker_quantity, maker.remaining_quantity);
                
                // Create a trade
                let trade = self.create_trade(
                    best_bid,
                    match_quantity,
                    &taker.market,
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
        }
        
        // Update taker status
        if taker_filled {
            taker_clone.status = Status::Filled;
        } else if taker_clone.filled_quantity > Quantity::ZERO {
            taker_clone.status = Status::PartiallyFilled;
        }
        
        taker_clone.updated_at = Utc::now();
        
        // Return updated taker order and trades
        (Some(Arc::new(taker_clone)), matched_makers, trades)
    }
    
    /// Create a trade from a match
    fn create_trade(
        &self,
        price: Price,
        quantity: Quantity,
        market: &str,
        buyer_order_id: Uuid,
        seller_order_id: Uuid,
        buyer_id: Uuid,
        seller_id: Uuid,
        taker_side: Side,
    ) -> Trade {
        Trade {
            id: Uuid::new_v4(),
            market: market.to_string(),
            price,
            quantity,
            amount: price * quantity,
            buyer_order_id,
            seller_order_id,
            buyer_id,
            seller_id,
            taker_side,
            created_at: Utc::now(),
        }
    }
}