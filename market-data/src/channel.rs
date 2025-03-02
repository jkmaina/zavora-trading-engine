//! Channel for market data distribution

use std::collections::HashMap;
use std::sync::Arc;

use crossbeam_channel::{self, Receiver, Sender};
use tokio::sync::Mutex;
use uuid::Uuid;

/// Topic types for market data
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Topic {
    /// Order book updates for a market
    OrderBook(String),
    /// Trades for a market
    Trades(String),
    /// Ticker updates for a market
    Ticker(String),
    /// All order book updates
    AllOrderBooks,
    /// All trades
    AllTrades,
    /// All ticker updates
    AllTickers,
}

/// Subscription entry
struct SubscriptionEntry {
    /// Sender channel
    sender: Sender<Arc<dyn std::any::Any + Send + Sync>>,
    /// Subscription ID
    id: Uuid,
}

/// Market data channel
pub struct MarketDataChannel {
    /// Senders by topic
    senders: Mutex<HashMap<Topic, Vec<SubscriptionEntry>>>,
}

impl MarketDataChannel {
    /// Create a new market data channel
    pub fn new() -> Self {
        Self {
            senders: Mutex::new(HashMap::new()),
        }
    }
    
    /// Subscribe to a topic
    pub async fn subscribe<T: 'static + Send + Sync>(&self, topic: Topic) -> Receiver<Arc<dyn std::any::Any + Send + Sync>> {
        let (sender, receiver) = crossbeam_channel::unbounded();
        let subscription_id = Uuid::new_v4();
        
        let mut senders = self.senders.lock().await;
        senders.entry(topic).or_default().push(SubscriptionEntry {
            sender,
            id: subscription_id,
        });
        
        receiver
    }
    
    /// Publish to a topic
    pub async fn publish<T: 'static + Send + Sync>(&self, topic: Topic, message: T) {
        let message = Arc::new(message) as Arc<dyn std::any::Any + Send + Sync>;
        let mut senders = self.senders.lock().await;
        
        // Publish to specific topic
        if let Some(topic_senders) = senders.get_mut(&topic) {
            // Remove closed channels
            topic_senders.retain(|entry| !entry.sender.is_empty());
            
            // Send to all subscribers
            for entry in topic_senders.iter() {
                let _ = entry.sender.try_send(message.clone());
            }
        }
        
        // Also publish to "all" topics if applicable
        match &topic {
            Topic::OrderBook(_market) => {
                if let Some(all_senders) = senders.get_mut(&Topic::AllOrderBooks) {
                    all_senders.retain(|entry| !entry.sender.is_empty());
                    for entry in all_senders.iter() {
                        let _ = entry.sender.try_send(message.clone());
                    }
                }
            },
            Topic::Trades(_market) => {
                if let Some(all_senders) = senders.get_mut(&Topic::AllTrades) {
                    all_senders.retain(|entry| !entry.sender.is_empty());
                    for entry in all_senders.iter() {
                        let _ = entry.sender.try_send(message.clone());
                    }
                }
            },
            Topic::Ticker(_market) => {
                if let Some(all_senders) = senders.get_mut(&Topic::AllTickers) {
                    all_senders.retain(|entry| !entry.sender.is_empty());
                    for entry in all_senders.iter() {
                        let _ = entry.sender.try_send(message.clone());
                    }
                }
            },
            _ => {}
        }
    }
    
    /// Unsubscribe using subscription ID
    pub async fn unsubscribe_by_id(&self, subscription_id: Uuid) -> bool {
        let mut senders = self.senders.lock().await;
        let mut found = false;
        
        // Check all topics for the subscription ID
        for (_topic, entries) in senders.iter_mut() {
            let initial_len = entries.len();
            entries.retain(|entry| entry.id != subscription_id);
            
            if entries.len() < initial_len {
                found = true;
            }
        }
        
        found
    }
    
    /// Unsubscribe from a topic (backwards compatibility)
    pub async fn unsubscribe(&self, topic: Topic, _receiver: &Receiver<Arc<dyn std::any::Any + Send + Sync>>) {
        // This is kept for backwards compatibility
        // The new unsubscribe_by_id method should be used instead
        let mut senders = self.senders.lock().await;
        if let Some(topic_senders) = senders.get_mut(&topic) {
            // Just remove closed channels
            topic_senders.retain(|entry| !entry.sender.is_empty());
        }
    }
}