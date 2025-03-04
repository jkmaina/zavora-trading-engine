//! Decimal type utilities for precise financial calculations

use rust_decimal::Decimal;
pub use rust_decimal_macros::dec;

/// Price type with high precision
pub type Price = Decimal;

/// Quantity type with high precision
pub type Quantity = Decimal;

/// Amount type with high precision (typically Price * Quantity)
pub type Amount = Decimal;

/// Precision helpers for common operations
pub mod precision {
    use super::*;
    
    /// Default price precision (8 decimal places)
    pub const PRICE_PRECISION: u32 = 8;
    
    /// Default quantity precision (8 decimal places)
    pub const QUANTITY_PRECISION: u32 = 8;
    
    /// Round price to standard precision
    pub fn round_price(price: Price) -> Price {
        price.round_dp(PRICE_PRECISION)
    }
    
    /// Round quantity to standard precision
    pub fn round_quantity(qty: Quantity) -> Quantity {
        qty.round_dp(QUANTITY_PRECISION)
    }
}
