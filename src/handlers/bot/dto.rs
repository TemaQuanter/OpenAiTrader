// This file contains all the Data Transfer Objects
// that are necessary to parse incoming data from ChatGPT.

use rust_decimal::Decimal;
use serde::Deserialize;

/// This structure represents an opening of a new position.
#[derive(Deserialize, Debug, Clone)]
pub struct PositionOpening {
    /// A ticker of the asset.
    pub ticker: String,
    /// A direction (short/long).
    pub direction: String,
    /// A risk for per trade represented in xR
    /// For example 0.5R, where R is the default risk per trade
    pub risk: Decimal,
    /// Order type.
    pub order_type: String,
    /// Lower bound entrance point.
    pub lower_bound_entry: Decimal,
    /// Upper bound entrance point.
    pub upper_bound_entry: Decimal,
    /// Stop loss price.
    pub stop_loss: Decimal,
    /// Take profit lower bound.
    pub take_profit_lower_bound: Option<Decimal>,
    /// Take profit upper bound.
    pub take_profit_upper_bound: Option<Decimal>,
} // end struct PositionOpening

/// This structure represents an increase of
/// the size of already opened position.
#[derive(Deserialize, Debug, Clone)]
pub struct PositionIncrease {
    /// A ticker of the asset.
    pub ticker: String,
    /// A direction (short/long).
    pub direction: String,
    /// A new stop loss.
    pub new_stop: Decimal,
    /// A percentage to increase the position by.
    pub percentage_to_increase: Decimal,
} // end struct PositionIncrease

/// This structure represents a closure of
/// an opened position.
#[derive(Deserialize, Debug, Clone)]
pub struct PositionClosure {
    /// A ticker of the asset.
    pub ticker: String,
    /// A direction (short/long).
    pub direction: String,
    /// What percentage of the volume
    /// should be closed.
    pub percentage_to_close: Decimal,
} // end struct PositionClosure

/// This structure represents a stop loss
/// adjustment on the opened position.
#[derive(Deserialize, Debug, Clone)]
pub struct StopLossAdjustment {
    /// A ticker of the asset.
    pub ticker: String,
    /// A direction (short/long).
    pub direction: String,
    /// A new stop.
    pub new_stop: Decimal,
} // end struct StopLossAdjustment

/// This structure represents a conditional order removal.
#[derive(Deserialize, Debug, Clone)]
pub struct ConditionalOrderRemoval {
    /// A ticker of the asset.
    pub ticker: String,
    /// A direction (short/long).
    pub direction: String,
    /// A new stop.
    pub trigger_price: Decimal,
} // end struct StopLossAdjustment
