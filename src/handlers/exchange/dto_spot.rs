// This file contains the Data Transfer Objects
// to communicate with an cryptocurrency exchange.

use rust_decimal::Decimal;
use serde::Deserialize;

/// This struct represents an opened position on
/// perpetual swaps.
#[derive(Deserialize, Debug)]
pub struct OpenedPosition {
    /// A ticker of the Coin
    pub coin: String,
    /// Transferable amount.
    pub transfer_balance: Decimal,
    /// Wallet balance.
    pub wallet_balance: Decimal,
} // end struct OpenedPosition

/// This struct represents an opened order on perpetual
/// swaps.
#[derive(Deserialize, Debug)]
pub struct OpenedOrder {
    /// A ticker of the Coin
    pub ticker: String,
    /// Order quantity.
    pub qty: String,
    /// Position side (short/long)
    pub side: String,
    /// A trigger price at which the order is supposed to get executed.
    pub trigger_price: String,
    /// Type of the order (Stop Loss, Take Profit, etc).
    pub stop_order_type: String,
    /// Order ID.
    pub order_id: String,
} // end struct OpenedPosition
