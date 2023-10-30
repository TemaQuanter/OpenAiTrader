// This file contains the Data Transfer Objects
// to communicate with an cryptocurrency exchange.

use rust_decimal::Decimal;
use serde::Deserialize;

/// This struct represents an opened position on
/// perpetual swaps.
#[derive(Deserialize, Debug)]
pub struct OpenedPosition {
    /// A ticker of the Coin
    pub ticker: String,
    /// Position side (short/long)
    pub side: String,
    /// Average entry price.
    pub avg_entry_price: Decimal,
    /// Last market price of the asset.
    pub last_market_price: Decimal,
    /// Stop loss price.
    pub stop_loss_price: Decimal,
    /// Quantity.
    pub qty: Decimal,
    /// Positions side in a hedge-mode.
    pub position_idx: i32,
} // end struct OpenedPosition

/// This struct represents an opened order on perpetual
/// swaps.
#[derive(Deserialize, Debug)]
pub struct OpenedOrder {
    /// A ticker of the Coin
    pub ticker: String,
    /// Position side (short/long)
    pub side: String,
    /// Type of the order (Stop Loss, Take Profit, etc).
    pub stop_order_type: String,
    /// Shows if the order is meant to reduce the
    /// position size.
    pub reduce_only: bool,
    /// Order ID.
    pub order_id: String,
    /// Position side in a hedge-mode.
    pub position_idx: i32,
} // end struct OpenedPosition
