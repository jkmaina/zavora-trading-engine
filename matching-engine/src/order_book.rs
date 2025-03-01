//! Order book implementation for price-time priority matching

use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;

use common::decimal::{Price, Quantity};
use common::model::order::{Order, Side};
use rust_decimal::Decimal;
use uuid::Uuid;

/// The buy side of the order book (bids)
pub struct BidSide {
    /// Price-ordered map of limit orders (price -> orders)
    /// For bids (buy orders), higher prices come first (reverse ordering)
    limits: BTreeMap<Price, Vec<Arc<Order>>>,
    /// Index for fast order lookup by ID
    order_map: HashMap<Uuid, (Price, usize)>,
}

impl BidSide {
    /// Create a new empty bid side
    pub fn new() -> Self {
        Self {
            limits: BTreeMap::new(),
            order_map: HashMap::new(),
        }
    }

    /// Add an order to the bid side
    pub fn add(&mut self, order: Arc<Order>) {
        if let Some(price) = order.price {
            // Store in reverse order for bids (highest price first)
            let price_level = self.limits.entry(price).or_default();
            let position = price_level.len();
            price_level.push(order.clone());
            self.order_map.insert(order.id, (price, position));
        }
    }

    /// Get the best price (highest bid)
    pub fn best_price(&self) -> Option<Price> {
        self.limits.keys().next().copied()
    }

    /// Get orders at the given price level
    pub fn orders_at(&self, price: Price) -> Option<&Vec<Arc<Order>>> {
        self.limits.get(&price)
    }

    /// Get all price levels with their orders (for market data)
    pub fn price_levels(&self, limit: usize) -> Vec<(Price, Quantity)> {
        self.limits
            .iter()
            .take(limit)
            .map(|(price, orders)| {
                let total_quantity = orders
                    .iter()
                    .map(|order| order.remaining_quantity)
                    .sum();
                (*price, total_quantity)
            })
            .collect()
    }

    /// Remove an order by ID
    pub fn remove(&mut self, order_id: Uuid) -> Option<Arc<Order>> {
        if let Some((price, position)) = self.order_map.remove(&order_id) {
            if let Some(orders) = self.limits.get_mut(&price) {
                if position < orders.len() {
                    // Remove the order and adjust positions for all following orders
                    let order = orders.remove(position);
                    
                    // Update positions for all orders after the removed one
                    for i in position..orders.len() {
                        if let Some(id) = orders.get(i).map(|o| o.id) {
                            self.order_map.insert(id, (price, i));
                        }
                    }
                    
                    // Clean up empty price levels
                    if orders.is_empty() {
                        self.limits.remove(&price);
                    }
                    
                    return Some(order);
                }
            }
        }
        None
    }
}

/// The sell side of the order book (asks)
pub struct AskSide {
    /// Price-ordered map of limit orders (price -> orders)
    /// For asks (sell orders), lower prices come first (natural ordering)
    limits: BTreeMap<Price, Vec<Arc<Order>>>,
    /// Index for fast order lookup by ID
    order_map: HashMap<Uuid, (Price, usize)>,
}

impl AskSide {
    /// Create a new empty ask side
    pub fn new() -> Self {
        Self {
            limits: BTreeMap::new(),
            order_map: HashMap::new(),
        }
    }

    /// Add an order to the ask side
    pub fn add(&mut self, order: Arc<Order>) {
        if let Some(price) = order.price {
            // Store in natural order for asks (lowest price first)
            let price_level = self.limits.entry(price).or_default();
            let position = price_level.len();
            price_level.push(order.clone());
            self.order_map.insert(order.id, (price, position));
        }
    }

    /// Get the best price (lowest ask)
    pub fn best_price(&self) -> Option<Price> {
        self.limits.keys().next().copied()
    }

    /// Get orders at the given price level
    pub fn orders_at(&self, price: Price) -> Option<&Vec<Arc<Order>>> {
        self.limits.get(&price)
    }

    /// Get all price levels with their orders (for market data)
    pub fn price_levels(&self, limit: usize) -> Vec<(Price, Quantity)> {
        self.limits
            .iter()
            .take(limit)
            .map(|(price, orders)| {
                let total_quantity = orders
                    .iter()
                    .map(|order| order.remaining_quantity)
                    .sum();
                (*price, total_quantity)
            })
            .collect()
    }

    /// Remove an order by ID
    pub fn remove(&mut self, order_id: Uuid) -> Option<Arc<Order>> {
        if let Some((price, position)) = self.order_map.remove(&order_id) {
            if let Some(orders) = self.limits.get_mut(&price) {
                if position < orders.len() {
                    // Remove the order and adjust positions for all following orders
                    let order = orders.remove(position);
                    
                    // Update positions for all orders after the removed one
                    for i in position..orders.len() {
                        if let Some(id) = orders.get(i).map(|o| o.id) {
                            self.order_map.insert(id, (price, i));
                        }
                    }
                    
                    // Clean up empty price levels
                    if orders.is_empty() {
                        self.limits.remove(&price);
                    }
                    
                    return Some(order);
                }
            }
        }
        None
    }
}

/// Common trait for order book sides
pub trait OrderBookSide {
    /// Add an order to this side
    fn add_order(&mut self, order: Arc<Order>);
    
    /// Remove an order from this side
    fn remove_order(&mut self, order_id: Uuid) -> Option<Arc<Order>>;
    
    /// Get the best price on this side
    fn best_price(&self) -> Option<Price>;
    
    /// Get all price levels with quantities
    fn get_price_levels(&self, limit: usize) -> Vec<(Price, Quantity)>;
}

impl OrderBookSide for BidSide {
    fn add_order(&mut self, order: Arc<Order>) {
        self.add(order);
    }
    
    fn remove_order(&mut self, order_id: Uuid) -> Option<Arc<Order>> {
        self.remove(order_id)
    }
    
    fn best_price(&self) -> Option<Price> {
        self.best_price()
    }
    
    fn get_price_levels(&self, limit: usize) -> Vec<(Price, Quantity)> {
        self.price_levels(limit)
    }
}

impl OrderBookSide for AskSide {
    fn add_order(&mut self, order: Arc<Order>) {
        self.add(order);
    }
    
    fn remove_order(&mut self, order_id: Uuid) -> Option<Arc<Order>> {
        self.remove(order_id)
    }
    
    fn best_price(&self) -> Option<Price> {
        self.best_price()
    }
    
    fn get_price_levels(&self, limit: usize) -> Vec<(Price, Quantity)> {
        self.price_levels(limit)
    }
}

/// Order book for a single market
pub struct OrderBook {
    /// Market symbol
    pub market: String,
    /// Buy side (bids)
    bids: BidSide,
    /// Sell side (asks)
    asks: AskSide,
    /// Last traded price
    pub last_price: Option<Price>,
}

impl OrderBook {
    /// Create a new empty order book for the given market
    pub fn new(market: String) -> Self {
        Self {
            market,
            bids: BidSide::new(),
            asks: AskSide::new(),
            last_price: None,
        }
    }
    
    /// Add an order to the book
    pub fn add_order(&mut self, order: Arc<Order>) {
        match order.side {
            Side::Buy => self.bids.add_order(order),
            Side::Sell => self.asks.add_order(order),
        }
    }
    
    /// Remove an order from the book
    pub fn remove_order(&mut self, order_id: Uuid, side: Side) -> Option<Arc<Order>> {
        match side {
            Side::Buy => self.bids.remove_order(order_id),
            Side::Sell => self.asks.remove_order(order_id),
        }
    }
    
    /// Get the best bid price
    pub fn best_bid(&self) -> Option<Price> {
        self.bids.best_price()
    }
    
    /// Get the best ask price
    pub fn best_ask(&self) -> Option<Price> {
        self.asks.best_price()
    }
    
    /// Get the current spread
    pub fn spread(&self) -> Option<Price> {
        match (self.best_ask(), self.best_bid()) {
            (Some(ask), Some(bid)) => Some(ask - bid),
            _ => None,
        }
    }
    
    /// Get the mid price
    pub fn mid_price(&self) -> Option<Price> {
        match (self.best_ask(), self.best_bid()) {
            (Some(ask), Some(bid)) => Some((ask + bid) / Decimal::TWO),
            _ => self.last_price,
        }
    }
    
    /// Get bid price levels with quantities (for market data)
    pub fn bid_levels(&self, limit: usize) -> Vec<(Price, Quantity)> {
        self.bids.get_price_levels(limit)
    }
    
    /// Get ask price levels with quantities (for market data)
    pub fn ask_levels(&self, limit: usize) -> Vec<(Price, Quantity)> {
        self.asks.get_price_levels(limit)
    }
    
    /// Check if orders would match
    pub fn would_match(&self, price: Price, side: Side) -> bool {
        match side {
            Side::Buy => self.best_ask().map_or(false, |ask| price >= ask),
            Side::Sell => self.best_bid().map_or(false, |bid| price <= bid),
        }
    }
    
    /// Update the last traded price
    pub fn set_last_price(&mut self, price: Price) {
        self.last_price = Some(price);
    }

    // Get a reference to the bids side
    pub fn bids(&self) -> &BidSide {
        &self.bids
    }
    
    /// Get a reference to the asks side
    pub fn asks(&self) -> &AskSide {
        &self.asks
    }
    
    /// Get the first order at the given ask price
    pub fn get_first_ask_order(&mut self, price: Price) -> Option<Arc<Order>> {
        if let Some(orders) = self.asks.orders_at(price) {
            if !orders.is_empty() {
                return Some(orders[0].clone());
            }
        }
        None
    }
    
    /// Get the first order at the given bid price
    pub fn get_first_bid_order(&mut self, price: Price) -> Option<Arc<Order>> {
        if let Some(orders) = self.bids.orders_at(price) {
            if !orders.is_empty() {
                return Some(orders[0].clone());
            }
        }
        None
    }
}