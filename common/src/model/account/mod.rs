//! Account models and related types

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::decimal::Quantity;
#[cfg(feature = "utoipa")]
use crate::utoipa::ToSchema;

/// Account model
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct Account {
    /// Unique account ID
    pub id: Uuid,
    /// Account creation timestamp
    pub created_at: DateTime<Utc>,
    /// Last update timestamp
    pub updated_at: DateTime<Utc>,
}

/// Balance model
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct Balance {
    /// Account ID
    pub account_id: Uuid,
    /// Asset symbol (e.g., "BTC", "USD")
    pub asset: String,
    /// Total balance
    pub total: Quantity,
    /// Available balance (not locked in orders)
    pub available: Quantity,
    /// Locked balance (in open orders)
    pub locked: Quantity,
    /// Last update timestamp
    pub updated_at: DateTime<Utc>,
}

impl Balance {
    /// Create a new balance with zero amounts
    pub fn new(account_id: Uuid, asset: String) -> Self {
        Self {
            account_id,
            asset,
            total: Quantity::ZERO,
            available: Quantity::ZERO,
            locked: Quantity::ZERO,
            updated_at: Utc::now(),
        }
    }
    
    /// Lock funds for an order
    pub fn lock(&mut self, amount: Quantity) -> Result<(), String> {
        if amount > self.available {
            return Err(format!("Insufficient balance: {} {}", self.available, self.asset));
        }
        
        self.available -= amount;
        self.locked += amount;
        self.updated_at = Utc::now();
        Ok(())
    }
    
    /// Unlock funds (on order cancel)
    pub fn unlock(&mut self, amount: Quantity) {
        self.locked -= amount;
        self.available += amount;
        self.updated_at = Utc::now();
    }
    
    /// Add funds to the balance
    pub fn deposit(&mut self, amount: Quantity) {
        self.total += amount;
        self.available += amount;
        self.updated_at = Utc::now();
    }
    
    /// Remove funds from the balance
    pub fn withdraw(&mut self, amount: Quantity) -> Result<(), String> {
        if amount > self.available {
            return Err(format!("Insufficient available balance: {} {}", self.available, self.asset));
        }
        
        self.total -= amount;
        self.available -= amount;
        self.updated_at = Utc::now();
        Ok(())
    }
}
