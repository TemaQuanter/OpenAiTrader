// This file contains all the functionality to interact with
// Bybit cryptocurrency exchange on perpetual swaps.

use std::sync::Arc;

use crypto_botters::Client;
use serde_json::json;

use rust_decimal::Decimal;
use rust_decimal_macros::dec;

use crate::{
    db_objects::user_model::User,
    handlers::{
        bot::dto_perpetual::{
            ConditionalOrderRemoval, PositionClosure, PositionIncrease, PositionOpening,
            StopLossAdjustment, TakeProfitPlacement,
        },
        exchange::dto_perpetual::OpenedOrder,
    },
    utils::{app_error::AppError, types_and_constants::SLIPPAGE_PERCENT},
};

use super::{dto_perpetual::OpenedPosition, exchange_service::ExchangeService};

/// This struct contains all the functionality to interact
/// with Bybit cryptocurrency exchange.
pub struct ExchangeServicePerpetual;

/// Implement all the functionality to interact with
/// Bybit cryptocurrency exchange on perpetual swaps.
impl ExchangeServicePerpetual {
    /// This function returns all the current opened
    /// perpetual swaps positions on UTA.
    pub async fn get_all_opened_positions(
        client: &Client,
    ) -> Result<Vec<OpenedPosition>, AppError> {
        // Form a request and get the information about opened positions.
        let get_request = [("category", "linear"), ("settleCoin", "USDT")];

        // Send a request and get the information about opened positions.
        let response =
            ExchangeService::send_robust_get_request(client, "/v5/position/list", &get_request)
                .await?;

        // Validate exchange response.
        ExchangeService::validate_exchange_response(&response)?;

        // Parse the data into a DTO.

        // Convert the response into an array of opened positions.
        let opened_positions_raw = response["result"]["list"].as_array().unwrap();

        // Declare an array that stores parsed opened positions.
        let mut opened_positions: Vec<OpenedPosition> = Vec::new();

        // Traverse each raw position and parse it into a structure.
        for raw_position in opened_positions_raw {
            // Parse an opened position into a struct.
            let position = Self::parse_opened_position(raw_position);

            // Insert the parsed position into the array.
            opened_positions.push(position);
        } // end for

        Ok(opened_positions)
    } // end fn get_all_opened_positions

    /// This function returns all opened orders on perpetual swaps in USDT.
    pub async fn get_all_opened_orders(client: &Client) -> Result<Vec<OpenedOrder>, AppError> {
        // Form a request and get the information about opened orders.
        let get_request = [("category", "linear"), ("settleCoin", "USDT")];

        // Send a request and get the information about opened orders.
        let response =
            ExchangeService::send_robust_get_request(client, "/v5/order/realtime", &get_request)
                .await?;

        // Validate exchange response.
        ExchangeService::validate_exchange_response(&response)?;

        // Parse the data into a DTO.

        // Convert the response into an array of opened orders.
        let opened_orders_raw = response["result"]["list"].as_array().unwrap();

        // Declare an array that stores parsed opened orders.
        let mut opened_orders: Vec<OpenedOrder> = Vec::new();

        // Traverse each raw order and parse it into a structure.
        for raw_order in opened_orders_raw {
            // Parse an opened order into a struct.
            let order = Self::parse_opened_order(raw_order);

            // Insert the parsed order into the array.
            opened_orders.push(order);
        } // end for

        Ok(opened_orders)
    } // end fn get_all_opened_orders

    /// This function parses the raw JSON object with an opened position
    /// into a structure.
    pub fn parse_opened_position(raw_opened_position: &serde_json::Value) -> OpenedPosition {
        // Try to parse the object into a structure.
        //
        // P.S There are extra double quotes that it is necessary
        // to get rid off.
        let opened_position = OpenedPosition {
            ticker: raw_opened_position["symbol"].to_string().replace("\"", ""),
            side: raw_opened_position["side"].to_string().replace("\"", ""),
            avg_entry_price: Decimal::from_str_exact(
                &raw_opened_position["avgPrice"]
                    .to_string()
                    .replace("\"", ""),
            )
            .unwrap(),
            last_market_price: Decimal::from_str_exact(
                &raw_opened_position["markPrice"]
                    .to_string()
                    .replace("\"", ""),
            )
            .unwrap(),
            stop_loss_price: Decimal::from_str_exact(
                &raw_opened_position["stopLoss"]
                    .to_string()
                    .replace("\"", ""),
            )
            .unwrap_or(dec!(-1.0)),
            qty: Decimal::from_str_exact(
                &raw_opened_position["size"].to_string().replace("\"", ""),
            )
            .unwrap(),
            position_idx: raw_opened_position["positionIdx"]
                .to_string()
                .replace("\"", "")
                .parse::<i32>()
                .unwrap(),
        }; // end opened_position

        opened_position
    } // end fn parse_opened_position

    /// This function parses the raw JSON object with an opened position
    /// into a structure.
    pub fn parse_opened_order(raw_opened_order: &serde_json::Value) -> OpenedOrder {
        // Try to parse the object into a structure.
        //
        // P.S There are extra double quotes that it is necessary
        // to get rid off.
        let opened_order = OpenedOrder {
            ticker: raw_opened_order["symbol"].to_string().replace("\"", ""),
            side: raw_opened_order["side"].to_string().replace("\"", ""),
            trigger_price: raw_opened_order["triggerPrice"]
                .to_string()
                .replace("\"", ""),
            stop_order_type: raw_opened_order["stopOrderType"]
                .to_string()
                .replace("\"", ""),
            order_id: raw_opened_order["orderId"].to_string().replace("\"", ""),
            position_idx: raw_opened_order["positionIdx"]
                .to_string()
                .replace("\"", "")
                .parse::<i32>()
                .unwrap(),
            reduce_only: raw_opened_order["reduceOnly"]
                .to_string()
                .replace("\"", "")
                .parse::<bool>()
                .unwrap(),
        }; // end opened_order

        opened_order
    } // end fn parse_opened_order

    /// This function retrieves the current price of an instrument
    /// on perpetual swaps in USDT.
    pub async fn get_instrument_price(
        client: &Client,
        ticker: impl AsRef<str>,
    ) -> Result<Decimal, AppError> {
        // Form a request and get the information about an instrument.
        let get_request = [("category", "linear"), ("symbol", ticker.as_ref())];

        // Send a request and get the information about an instrument.
        let response =
            ExchangeService::send_robust_get_request(client, "/v5/market/tickers", &get_request)
                .await?;

        // Validate the response from exchange.
        ExchangeService::validate_exchange_response(&response)?;

        // Get the current market price.
        let market_price = Decimal::from_str_exact(
            &response["result"]["list"][0]["markPrice"]
                .to_string()
                .replace("\"", ""),
        )
        .unwrap();

        Ok(market_price)
    } // end get_instrument_price

    /// This function retrieves the price step for a particular instrument
    /// on perpetual swaps in USDT.
    pub async fn _get_price_step(
        client: &Client,
        ticker: impl AsRef<str>,
    ) -> Result<Decimal, AppError> {
        // Form a request and get the information about an instrument.
        let get_request = [("category", "linear"), ("symbol", ticker.as_ref())];

        // Send a request and get the information about an instrument.
        let response = ExchangeService::send_robust_get_request(
            client,
            "/v5/market/instruments-info",
            &get_request,
        )
        .await?;

        // Validate the response from exchange.
        ExchangeService::validate_exchange_response(&response)?;

        // Get the quantity step for the instrument.
        let price_step = Decimal::from_str_exact(
            &response["result"]["list"][0]["priceFilter"]["tickSize"]
                .to_string()
                .replace("\"", ""),
        )
        .unwrap();

        Ok(price_step)
    } // end fn get_price_step

    /// This function retrieves the quantity step for a contract
    /// on a cryptocurrency exchange.
    pub async fn get_quantity_step(
        client: &Client,
        ticker: impl AsRef<str>,
    ) -> Result<Decimal, AppError> {
        // Form a request and get the information about an instrument.
        let get_request = [("category", "linear"), ("symbol", ticker.as_ref())];

        // Send a request and get the information about an instrument.
        let response = ExchangeService::send_robust_get_request(
            client,
            "/v5/market/instruments-info",
            &get_request,
        )
        .await?;

        // Validate the response from exchange.
        ExchangeService::validate_exchange_response(&response)?;

        // Get the quantity step for the instrument.
        let quantity_step = Decimal::from_str_exact(
            &response["result"]["list"][0]["lotSizeFilter"]["qtyStep"]
                .to_string()
                .replace("\"", ""),
        )
        .unwrap();

        Ok(quantity_step)
    } // end fn get_quantity_step

    /// This function opens a new position on perpetual swaps
    /// in USDT.
    pub async fn open_position(
        position_opening: Arc<PositionOpening>,
        user: User,
    ) -> Result<(), AppError> {
        // Get a cloned user for partial take profits.
        let cloned_user = user.clone();

        // Get user credentials.
        let api_key = user.exchange_api_key.unwrap();
        let api_secret = user.exchange_api_secret.unwrap();

        // Get an exchange client.
        let client = ExchangeService::get_client(api_key, api_secret);

        // Make a copy of the position to modify it later.
        let mut position_opening = (*position_opening).clone();

        // Position IDX is a special parameter that is required for hedge-mode.
        let position_idx: i32;

        // These parameters are required to place a conditional order.
        let trigger_direction: Option<i32>;
        let trigger_price: Option<Decimal>;

        // If the lower bound price matches the upper bound price,
        // then it is most likely to be a conditional order.
        if position_opening.lower_bound_entry == position_opening.upper_bound_entry {
            position_opening.order_type = "conditional".to_string();
        } // end if

        // Make sure that lower bound entry is always equal or less than upper bound entry.
        if position_opening.lower_bound_entry > position_opening.upper_bound_entry {
            // Swap them.
            let temp = position_opening.lower_bound_entry;
            position_opening.lower_bound_entry = position_opening.upper_bound_entry;
            position_opening.upper_bound_entry = temp;
        } // end if

        // Parse the side received from ChatGPT into a proper
        // format for exchange API.
        if position_opening.direction.to_lowercase().as_str() == "long" {
            // Change it to "Buy".
            position_opening.direction = "Buy".to_string();

            // Determine the lower and upper bound for long take profit.
            let upper_bound = position_opening
                .take_profit_upper_bound
                .max(position_opening.take_profit_lower_bound);
            let lower_bound = position_opening
                .take_profit_upper_bound
                .min(position_opening.take_profit_lower_bound);

            position_opening.take_profit_lower_bound = lower_bound;
            position_opening.take_profit_upper_bound = upper_bound;

            // Set IDX parameter.
            position_idx = 1;
        } else if position_opening.direction.to_lowercase().as_str() == "short" {
            // Change it to "Sell".
            position_opening.direction = "Sell".to_string();

            // Determine the lower and upper bound for short take profit.
            let upper_bound = position_opening
                .take_profit_upper_bound
                .min(position_opening.take_profit_lower_bound);
            let lower_bound = position_opening
                .take_profit_upper_bound
                .max(position_opening.take_profit_lower_bound);

            position_opening.take_profit_lower_bound = lower_bound;
            position_opening.take_profit_upper_bound = upper_bound;

            // Set IDX parameter.
            position_idx = 2;
        } else {
            // It is most likely to be an error.
            return Err(AppError::new("Failed to parse position side"));
        } // end if

        // Get the current price of the contract.
        // If the order type is conditional,
        // then the variable is equal to the entry price.
        let cur_contract_price = match position_opening.order_type.to_lowercase().as_ref() {
            "market" => {
                // Set parameters required to place a conditional order to None.
                trigger_direction = None;
                trigger_price = None;

                Self::get_instrument_price(&client, &position_opening.ticker)
                    .await
                    .map_err(|error| AppError::new(error.to_string()))?
            } // end "market"
            "conditional" => {
                // Set parameters required to place a conditional order.

                // Set trigger direction.
                if position_opening.direction == "Buy" {
                    // The order should become active if the price raises to
                    // the target.
                    trigger_direction = Some(1);
                } else {
                    // The order should become active if the price falls to
                    // the target.
                    trigger_direction = Some(2);
                } // end if

                // Set a trigger price.
                trigger_price = Some(position_opening.lower_bound_entry);

                position_opening.lower_bound_entry
            } // end "conditional"
            _ => return Err(AppError::new("Failed to parse order type")),
        }; // end match

        // Check if it is a market order.
        if position_opening.order_type == "market" {
            // Get all positions currently opened for a user.
            let opened_positions = Self::get_all_opened_positions(&client).await?;

            // Check if the position is already opened.
            // If so, then stop function execution.
            for opened_position in opened_positions {
                // Check if the current position is the position the function
                // is trying to open.
                if opened_position.ticker == position_opening.ticker
                    && opened_position.side == position_opening.direction
                {
                    // This is a position that the function is trying to open.
                    // Stop the function execution.
                    return Err(AppError::new("The position is already opened"));
                } // end if
            } // end for
        } // end if

        // Get the current market price of the contract.
        let cur_market_price = Self::get_instrument_price(&client, &position_opening.ticker)
            .await
            .map_err(|error| AppError::new(error.to_string()))?;

        // If the current market price is lower than the lower bound of the entry price
        // for long position or higher than the upper bound for short position,
        // then it is unsafe to get into the position.

        // It is also unsafe to get into the position if the price rose
        // above the upper entry bound for long position or fell below
        // the lower bound for short position.
        //
        // Explanation:
        //
        // The risk/reward ratio is not reasonable anymore.

        // This rule applies only to the market orders, not conditional ones.
        if position_opening.order_type == "market" {
            // Check if the price is withing the given range.
            if position_opening.lower_bound_entry > cur_market_price {
                // The asset price fell below the lower bound for entrance
                // into a position.
                return Err(AppError::new(
                    "The market price is lower than the minimum entry price",
                )); // end return
            } else if position_opening.upper_bound_entry < cur_market_price {
                // The asset price rose above the upper bound for entrance
                // into a position.
                return Err(AppError::new(
                    "The market price is higher than the maximum entry price",
                )); // end return
            } // end if
        } // end if

        // Check that the user has deposit set.
        if user.deposit.is_none() {
            // The risk must be set.
            return Err(AppError::new("Deposit is not set for the user"));
        } // end if

        // Check that the user has risk set.
        if user.risk.is_none() {
            // The risk must be set.
            return Err(AppError::new("Risk is not set for the user"));
        } // end if

        // Calculate the price movement before stop loss in percentage.
        let stop_loss_percentage =
            (dec!(1.0) - (position_opening.stop_loss / cur_contract_price)).abs() * dec!(100.0);

        // Calculate the value of contracts to buy/sell in USDT.
        let contracts_value = (user.deposit.unwrap() / stop_loss_percentage)
            * (user.risk.unwrap() * position_opening.risk);

        // Calculate the quantity of contracts to buy/sell.
        let qty = contracts_value / cur_contract_price;

        // Get the quantity step for the instrument.
        let quantity_step = &Self::get_quantity_step(&client, &position_opening.ticker)
            .await
            .map_err(|error| AppError::new(error.to_string()))?;

        // Adjust the quantity according to the quantity step.
        // If the quantity is less then the minimum quantity step, then make it equal to quantity step.
        // Otherwise, the position will not be opened.
        let qty =
            (Decimal::floor(&(qty / quantity_step)) * quantity_step).max(quantity_step.to_owned());

        // Calculate an appropriate leverage.
        let leverage = if stop_loss_percentage < dec!(6.0) {
            // Very short stop - use huge leverage.
            dec!(10.0)
        } else if stop_loss_percentage > dec!(20.0) {
            // Very long stop - calculate the closest safe leverage.
            Decimal::floor(&(dec!(100.0) / stop_loss_percentage - dec!(0.3)))
        } else {
            // Default leverage.
            dec!(5.0)
        }; // end if

        // Set a position mode to hedge-mode.
        Self::set_hedge_mode(&client, &position_opening.ticker)
            .await
            .map_err(|error| AppError::new(error.to_string()))?;

        // Setup the leverage.
        Self::set_leverage(
            &client,
            &position_opening.ticker,
            &position_opening.direction,
            leverage,
        )
        .await?;

        // Create the POST request for position opening.
        let mut post_request = json!({
            "category": "linear",
            "symbol": &position_opening.ticker,
            "side": position_opening.direction,
            "orderType": "Market",
            "qty": qty.to_string(),
            "stopLoss": position_opening.stop_loss,
            // Required for hedge-mode
            "positionIdx": position_idx,
            // Required for conditional orders.
            "triggerDirection": if trigger_direction.is_none() {
                // It is a market order.
                // The parameter is not required.
                serde_json::Value::Null
            } else {
                // It is a conditional order.
                // The parameter is required.
                serde_json::Value::String(trigger_direction.unwrap().to_string())
            }, // end if
            // Required for conditional orders.
            "triggerPrice": if trigger_price.is_none() {
                // It is a market order.
                // The parameter is not required.
                serde_json::Value::Null
            } else {
                // It is a conditional order.
                // The parameter is required.
                serde_json::Value::String(trigger_price.unwrap().to_string())
            }, // end if
            "tpslMode": "Full"
        }); // end json!

        // Verify that the position is safe to open.
        // This function will generate and return an error,
        // if it is unsafe to open a position.
        Self::verify_position(&position_opening).await?;

        // Try to unwrap take profit boundaries.

        // Check if there is no take profit set for the position.
        if position_opening.take_profit_lower_bound.is_none()
            && position_opening.take_profit_upper_bound.is_some()
        {
            // Upper bound has a value, lower bound - not.

            // Assign the upper boundary to the lower one.
            position_opening.take_profit_lower_bound = position_opening.take_profit_upper_bound;

            // Add a full take profit for the position.
            post_request["takeProfit"] = serde_json::Value::String(
                position_opening
                    .take_profit_upper_bound
                    .unwrap()
                    .to_string(),
            );
        } else if position_opening.take_profit_lower_bound.is_some()
            && position_opening.take_profit_upper_bound.is_none()
        {
            // Lower bound has a value, upper bound - not.

            // Assign the lower boundary to the upper one.
            position_opening.take_profit_upper_bound = position_opening.take_profit_lower_bound;

            // Add a full take profit for the position.
            post_request["takeProfit"] = serde_json::Value::String(
                position_opening
                    .take_profit_upper_bound
                    .unwrap()
                    .to_string(),
            );
        } else if position_opening.take_profit_lower_bound.is_some()
            && position_opening.take_profit_upper_bound.is_some()
        {
            // Both boundaries contain a value.
            // Add a full take profit for the position.
            post_request["takeProfit"] = serde_json::Value::String(
                position_opening
                    .take_profit_upper_bound
                    .unwrap()
                    .to_string(),
            );
        } // end if

        // Send a request to open a position on perpetual swaps.
        let response: serde_json::Value =
            ExchangeService::send_robust_post_request(&client, "/v5/order/create", &post_request)
                .await?;

        // Check if it was a conditional order, then it is impossible to set
        // partial take profits and it is possible to exit the function right here.
        if position_opening.order_type == "conditional" {
            return ExchangeService::validate_exchange_response(&response);
        } // end if

        // Check if there were no take profit information provided.
        if position_opening.take_profit_lower_bound.is_none()
            || position_opening.take_profit_upper_bound.is_none()
        {
            // There is no information about take profits and it is necessary to exit the function right here.
            return ExchangeService::validate_exchange_response(&response);
        } // end if

        // Safely unwrap take profit boundaries.
        let take_profit_lower_bound = position_opening.take_profit_lower_bound.unwrap();
        let take_profit_upper_bound = position_opening.take_profit_upper_bound.unwrap();

        // Check if both boundaries are equal, then there is no need to set partial orders.
        if take_profit_lower_bound == take_profit_upper_bound {
            return ExchangeService::validate_exchange_response(&response);
        } // end if

        // The number of partial take profits to set.
        // It is float, as it is necessary for further calculations.
        let number_take_profits = dec!(3.0);

        // Calculate the quantity to close on each partial take profit.
        let partial_qty =
            Decimal::floor(&((qty / number_take_profits) / quantity_step)) * quantity_step;

        // If the partial quantity is less then the quantity step, then do not set the take-profit grid.
        if partial_qty < quantity_step.to_owned() {
            return ExchangeService::validate_exchange_response(&response);
        } // end if

        // Calculate the price step to close the orders.
        let price_step =
            (take_profit_upper_bound - take_profit_lower_bound).abs() / number_take_profits;

        // A variable to keep track of the price in the loop.
        // It is necessary to take the minimum to calculate take profits
        // for short.
        let mut cur_price = take_profit_lower_bound;

        // Setup a grid of smaller take profits on the way to the main goal.
        for _ in 0..number_take_profits.to_string().parse::<f64>().unwrap() as i32 {
            // Set a new conditional order with take profit.
            Self::place_partial_take_profit_order(
                Arc::new(TakeProfitPlacement {
                    ticker: position_opening.ticker.clone(),
                    direction: position_opening.direction.clone(),
                    take_profit_price: cur_price,
                    take_profit_percentage: dec!(100) / (number_take_profits + dec!(1)),
                }),
                cloned_user.clone(),
            )
            .await?;

            // Get a new price for the next take profit.
            // If it is short position, then subtract, if long - add.
            if position_opening.direction == "Buy" {
                cur_price += price_step;
            } else {
                cur_price -= price_step;
            } // end if
        } // end while

        ExchangeService::validate_exchange_response(&response)
    } // end fn open_position

    /// This function increases the size of an opened position on perpetual swaps.
    pub async fn increase_position(
        position_increase_order: Arc<PositionIncrease>,
        user: User,
    ) -> Result<(), AppError> {
        // Check if position to increase is not greater than an adequate value.
        if position_increase_order.percentage_to_increase > dec!(300) {
            return Err(AppError::new("Inadequate risk"));
        } // end if

        // Get user credentials.
        let api_key = user.exchange_api_key.unwrap();
        let api_secret = user.exchange_api_secret.unwrap();

        // Get an exchange client.
        let client = ExchangeService::get_client(api_key, api_secret);

        // Get all opened positions on perpetual swaps.
        let opened_positions = Self::get_all_opened_positions(&client).await?;

        // Initialize the position to increase;
        let mut position_to_increase: Option<OpenedPosition> = None;

        // Shadow position increase order in order to modify it.
        let mut position_increase_order = (*position_increase_order).clone();

        // A special parameter that is required to identify a position side in a hedge-mode.
        let position_idx: i32;

        // Convert the position direction into the standard required by the exchange.
        if position_increase_order.direction == "long" {
            position_increase_order.direction = "Buy".to_string();

            // Set a special index to opening a long position in a hedge-mode.
            position_idx = 1;
        } else if position_increase_order.direction == "short" {
            position_increase_order.direction = "Sell".to_string();

            // Set a special index to opening a short position in a hedge-mode.
            position_idx = 2;
        } else {
            // The direction format is invalid.
            return Err(AppError::new("Direction format is invalid"));
        } // end if

        // Try to find the position to increase.
        for opened_position in opened_positions {
            // Check whether or not the current position is the position
            // to increase the size of.
            if opened_position.ticker == position_increase_order.ticker
                && opened_position.side == position_increase_order.direction
            {
                // It is the position to increase the size of.
                position_to_increase = Some(opened_position);

                // There is no need to loop further, as the position
                // is already found.
                break;
            } // end if
        } // end for

        // If the position to increase was not found, then return an error.
        if position_to_increase.is_none() {
            return Err(AppError::new(
                "The position to increase the size of was not found",
            )); // end return
        } // end if

        // Safely unwrap the position to increase the size of.
        let position_to_increase = position_to_increase.unwrap();

        // Calculate the quantity it is necessary to buy/sell to increase the position size.
        let extra_qty = position_to_increase.qty
            * (position_increase_order.percentage_to_increase / dec!(100.0));

        // Get the price step of the asset on perpetual swaps.
        let qty_step = Self::get_quantity_step(&client, &position_increase_order.ticker).await?;

        // Round the quantity.
        let extra_qty = (Decimal::ceil(&(extra_qty / qty_step)) * qty_step).max(qty_step);

        // Check that the user has risk set.
        if user.risk.is_none() {
            // The risk must be set.
            return Err(AppError::new("Risk is not set for the user"));
        } // end if

        // Check that the user has deposit set.
        if user.deposit.is_none() {
            // The risk must be set.
            return Err(AppError::new("Deposit is not set for the user"));
        } // end if

        // Safely unwrap risk and deposit.
        let risk = user.risk.unwrap();
        let deposit = user.deposit.unwrap();

        // New average entry price.
        let new_avg_entry_price = (position_to_increase.qty * position_to_increase.avg_entry_price
            + extra_qty * position_to_increase.last_market_price)
            / (position_to_increase.qty + extra_qty);

        // Check what direction this deal should be applied to.
        if position_increase_order.direction == "Buy" {
            // This is a long position increase.
            // Check that the new average entry price and the new stop
            // would not make a risk more than one risk unit, set by the user.
            if (new_avg_entry_price - position_increase_order.new_stop)
                * (position_to_increase.qty + extra_qty)
                > ((risk + SLIPPAGE_PERCENT) / dec!(100.0)) * deposit
            {
                // The risk is unacceptable.
                return Err(AppError::new(
                    "The new position condition exceeds the maximum risk limit",
                ));
            } // end if
        } else if position_increase_order.direction == "Sell" {
            // This is a short position to increase.
            // Check that the new average entry price and the new stop
            // would not make a risk more than one risk unit, set by the user.
            if (position_increase_order.new_stop - new_avg_entry_price)
                * (position_to_increase.qty + extra_qty)
                > ((risk + SLIPPAGE_PERCENT) / dec!(100.0)) * deposit
            {
                // The risk is unacceptable.
                return Err(AppError::new(
                    "The new position condition exceeds the maximum risk limit",
                ));
            } // end if
        } else {
            // This is an error. There is no third option available.
            return Err(AppError::new("Only Sell and Buy directions supported"));
        } // end if

        // Form a request to place an order to increase the position size on perpetual swaps.
        let post_request = json!({
            "category": "linear",
            "symbol": position_increase_order.ticker,
            "side": position_increase_order.direction,
            "orderType": "Market",
            "qty": extra_qty.to_string(),
            "stopLoss": position_increase_order.new_stop.to_string(),
            "positionIdx": position_idx,
        }); // end json!

        // Place an order to increase the position size on perpetual swaps.
        let response =
            ExchangeService::send_robust_post_request(&client, "/v5/order/create", &post_request)
                .await?;

        // Validate the response from exchange.
        ExchangeService::validate_exchange_response(&response)
    } // end fn increase_position

    /// This function closes a part of a position on perpetual swaps
    /// in USDT.
    pub async fn close_part_of_position(
        position_closure: Arc<PositionClosure>,
        user: User,
    ) -> Result<(), AppError> {
        // Get a clone of the user to be able to adjust a stop loss.
        // let user_clone = user.clone();

        // Get user credentials.
        let api_key = user.exchange_api_key.unwrap();
        let api_secret = user.exchange_api_secret.unwrap();

        // Get an exchange client.
        let client = ExchangeService::get_client(api_key, api_secret);

        // Get all the currently opened position.
        let opened_positions = Self::get_all_opened_positions(&client).await?;

        // A position that it is required to adjust.
        let mut cur_position: Option<OpenedPosition> = None;

        // Convert contract direction from (short/long) to (Sell/Buy).
        let side = if position_closure.direction.to_lowercase() == "long" {
            "Buy"
        } else {
            "Sell"
        }; // end if

        // Traverse all the opened positions and select the required one.
        for opened_position in opened_positions {
            // Check if current position is the position being searched for.
            if opened_position.ticker == position_closure.ticker && opened_position.side == side {
                // This is the required position.
                cur_position = Some(opened_position);
                break;
            } // end if
        } // end for

        // If the position was not found, then return an error.
        if cur_position.is_none() {
            return Err(AppError::new("The position to close is not found"));
        } // end if

        // Safely unwrap the position.
        let cur_position = cur_position.unwrap();

        // A variable to determine what side of the position to close
        // (short/long). The parameter is required by API.
        let position_idx: i32;

        // Close the position buy placing an order in the opposite direction.
        let closure_side: &str;

        // Determine what side to place the order on.
        if side == "Buy" {
            position_idx = 1;
            closure_side = "Sell";
        } else {
            position_idx = 2;
            closure_side = "Buy";
        } // end if

        // Get the quantity step of the contract.
        let quantity_step = Self::get_quantity_step(&client, cur_position.ticker).await?;

        // Calculate the quantity to fix.
        let qty = cur_position.qty * (position_closure.percentage_to_close / dec!(100.0));

        // Adjust the quantity so to fit the quantity step restrictions.
        // If the quantity to fix is less, than the quantity step, then make it equal to quantity step.
        let qty = (Decimal::floor(&(qty / quantity_step)) * quantity_step).max(quantity_step);

        // Form a request to adjust the position size on perpetual swaps.
        let post_request = json!({
            "category": "linear",
            "symbol": position_closure.ticker,
            "side": closure_side,
            "orderType": "Market",
            "qty": qty.to_string(),
            "reduceOnly": true,
            "positionIdx": position_idx
        }); // end json!

        // Send a request to adjust the position size on perpetual swaps.
        let response =
            ExchangeService::send_robust_post_request(&client, "/v5/order/create", &post_request)
                .await?;

        ExchangeService::validate_exchange_response(&response)
    } // end fn close_part_of_position

    /// This function adjusts the stop loss of a particular position on
    /// perpetual swaps in USDT.
    pub async fn adjust_stop_loss(
        stop_loss_adjustment: Arc<StopLossAdjustment>,
        user: User,
    ) -> Result<(), AppError> {
        // Get user credentials.
        let api_key = user.exchange_api_key.unwrap();
        let api_secret = user.exchange_api_secret.unwrap();

        // Get an exchange client.
        let client = ExchangeService::get_client(api_key, api_secret);

        // This variable shows the side of the opened position.
        let position_side: &str;

        // Convert contract direction from (short/long) to (Sell/Buy).
        // ATTENTION: To Stop Loss for Long is Sell, for Short - Buy.
        let side = if stop_loss_adjustment.direction.to_lowercase() == "long" {
            position_side = "Buy";
            "Sell"
        } else {
            position_side = "Sell";
            "Buy"
        }; // end if

        // A position that it is required to adjust a stop loss of.
        let mut cur_position: Option<OpenedPosition> = None;

        // Get all the currently opened positions.
        let opened_positions = Self::get_all_opened_positions(&client).await?;

        // Traverse all the opened positions and find the one
        // that it is necessary to adjust the stop loss of.
        for opened_position in opened_positions {
            // Check if the current position is the position searched for.
            if opened_position.ticker == stop_loss_adjustment.ticker
                && opened_position.side == position_side
            {
                // This is the position searched for.
                cur_position = Some(opened_position);
                break;
            } // end if
        } // end for

        // Check if the current position was found.
        if cur_position.is_none() {
            // The position was not found.
            return Err(AppError::new(
                "The position to adjust the stop loss of was not found",
            )); // end return
        } // end if

        // Safely unwrap the position.
        let cur_position = cur_position.unwrap();

        // Check that the user has risk set.
        if user.risk.is_none() {
            // The risk must be set.
            return Err(AppError::new("Risk is not set for the user"));
        } // end if

        // Check that the user has deposit set.
        if user.deposit.is_none() {
            // The risk must be set.
            return Err(AppError::new("Deposit is not set for the user"));
        } // end if

        // Safely unwrap risk and deposit.
        let risk = user.risk.unwrap();
        let deposit = user.deposit.unwrap();

        // Check that the new stop loss does not exceed one unit of risk.
        if cur_position.side == "Buy" {
            // If the position side is buy and the new stop loss is greater
            // than the current one, it is safe to change it, otherwise
            // extra checks required.
            if cur_position.stop_loss_price > stop_loss_adjustment.new_stop {
                // Make sure that a new stop loss will not make a risk
                // greater than one unit of risk, set by the user.
                if (cur_position.avg_entry_price - stop_loss_adjustment.new_stop) * cur_position.qty
                    > ((risk + SLIPPAGE_PERCENT) / dec!(100.0)) * deposit
                {
                    // This new stop exceeds the maximum risk limit.
                    return Err(AppError::new("The new stop exceeds the maximum risk limit"));
                } // end if
            } // end if
        } else if cur_position.side == "Sell" {
            // If the position side is sell and the new stop loss is lower
            // than the current one, it is safe to change it, otherwise
            // extra checks required.
            if cur_position.stop_loss_price < stop_loss_adjustment.new_stop {
                // Make sure that a new stop loss will not make a risk
                // greater than one unit of risk, set by the user.
                if (stop_loss_adjustment.new_stop - cur_position.avg_entry_price) * cur_position.qty
                    > ((risk + SLIPPAGE_PERCENT) / dec!(100.0)) * deposit
                {
                    // This new stop exceeds the maximum risk limit.
                    return Err(AppError::new("The new stop exceeds the maximum risk limit"));
                } // end if
            } // end if
        } else {
            // Throw an error. The position cannot be anything else.
            return Err(AppError::new("The position side can only be Buy or Sell"));
        } // end if

        // Get all the opened orders.
        let opened_orders = Self::get_all_opened_orders(&client).await?;

        // An order that it is required to adjust a stop loss of.
        let mut cur_order: Option<OpenedOrder> = None;

        // Get only one order (full stop loss of the required contract).
        for opened_order in opened_orders {
            // Check if current order is the order that is searched for.
            if opened_order.ticker == stop_loss_adjustment.ticker
                && opened_order.side == side
                && opened_order.stop_order_type == "StopLoss"
            {
                // This is the required order.
                cur_order = Some(opened_order);
                break;
            } // end if
        } // end for

        // If the order was not found, then return error.
        if cur_order.is_none() {
            return Err(AppError::new(
                "The order was not found for stop loss adjustment",
            ));
        } // end if

        // Safely unwrap the order.
        let cur_order = cur_order.unwrap();

        // Form a request to adjust a stop loss of the order on perpetual swaps.
        let post_request = json!({
            "category": "linear",
            "symbol": stop_loss_adjustment.ticker,
            "orderId": cur_order.order_id,
            "triggerPrice": stop_loss_adjustment.new_stop.to_string()
        }); // end json!

        // Send a request to adjust a stop loss of the order on perpetual swaps.
        let response =
            ExchangeService::send_robust_post_request(&client, "/v5/order/amend", &post_request)
                .await?;

        ExchangeService::validate_exchange_response(&response)
    } // end fn adjust_stop_loss

    /// This function sets a leverage for a particular position on
    /// perpetual swaps in USDT.
    ///
    /// NOTE: Direction must be "Sell" or "Buy".
    pub async fn set_leverage(
        client: &Client,
        ticker: &String,
        direction: &String,
        leverage: Decimal,
    ) -> Result<(), AppError> {
        // Get set leverages on perpetual swaps for a particular asset.
        let (mut long_position_leverage, mut short_position_leverage) =
            Self::get_position_leverage(client, ticker).await?;

        // Check what position leverage it is necessary to adjust.
        if direction == "Buy" {
            // It is necessary to adjust the leverage of a long position.
            long_position_leverage = leverage
        } else if direction == "Sell" {
            // It is necessary to adjust the leverage of a short position.
            short_position_leverage = leverage
        } else {
            // The direction is given incorrectly.
            return Err(AppError::new("The direction is provided incorrectly"));
        } // end if

        // Form a request to adjust a leverage on perpetual swaps.
        let post_request = json!({
            "category": "linear",
            "symbol": ticker,
            "buyLeverage": long_position_leverage.to_string(),
            "sellLeverage": short_position_leverage.to_string(),
        }); // end json!

        // Send a request to adjust a leverage on perpetual swaps.
        let response = ExchangeService::send_robust_post_request(
            client,
            "/v5/position/set-leverage",
            &post_request,
        )
        .await?;

        ExchangeService::validate_exchange_response(&response)
    } // end fn set_leverage

    /// This function retrieves a set leverage fro a particular position
    /// on perpetual swaps.
    ///
    /// The first element of the tuple is long position leverage.
    /// The second element of the tuple is short position leverage.
    pub async fn get_position_leverage(
        client: &Client,
        ticker: &String,
    ) -> Result<(Decimal, Decimal), AppError> {
        // Form a request to get a position leverage on perpetual swaps.
        let get_request = [("category", "linear"), ("symbol", ticker)];

        // Send a request to get a position leverage on perpetual swaps.
        let response =
            ExchangeService::send_robust_get_request(client, "/v5/position/list", &get_request)
                .await?;

        // Retrieve the leverage for long position.
        let long_position_leverage = response["result"]["list"][0]["leverage"]
            .to_string()
            .replace("\"", "");

        // Retrieve the leverage for short position.
        let short_position_leverage = response["result"]["list"][1]["leverage"]
            .to_string()
            .replace("\"", "");

        Ok((
            Decimal::from_str_exact(&long_position_leverage).unwrap(),
            Decimal::from_str_exact(&short_position_leverage).unwrap(),
        ))
    } // end fn get_position_leverage

    /// This function sets a position in a hedge-mode.
    pub async fn set_hedge_mode(client: &Client, ticker: &String) -> Result<(), AppError> {
        // Form a request to set a position in a hedge-mode on perpetual swaps.
        let post_request = json!({
            "category": "linear",
            "symbol": ticker,
            "mode": 3
        }); // end json!

        // Send a request to set a position in a hedge-mode on perpetual swaps.
        let response = ExchangeService::send_robust_post_request(
            client,
            "/v5/position/switch-mode",
            &post_request,
        )
        .await?;

        ExchangeService::validate_exchange_response(&response)
    } // end fn set_hedge_mode

    /// This function cancels an opened order on perpetual swaps in USDT.
    pub async fn cancel_order(
        order_removal: Arc<ConditionalOrderRemoval>,
        user: User,
    ) -> Result<(), AppError> {
        // Get user credentials.
        let api_key = user.exchange_api_key.unwrap();
        let api_secret = user.exchange_api_secret.unwrap();

        // Get an exchange client.
        let client = ExchangeService::get_client(api_key, api_secret);

        // Get a clone of the order to remove.
        let mut order_removal = (*order_removal).clone();

        // Adjust the direction of the order to remove, so that it would
        // the exchange standards.
        if order_removal.direction == "long" {
            order_removal.direction = "Buy".to_string();
        } else if order_removal.direction == "short" {
            order_removal.direction = "Sell".to_string();
        } else {
            // The format of the direction is invalid.
            return Err(AppError::new("The format of direction is invalid"));
        } // end if

        // Get a trigger price as a string.
        let trigger_price_string = order_removal.trigger_price.to_string();

        // Get all opened orders on perpetual swaps.
        let opened_orders = Self::get_all_opened_orders(&client).await?;

        // Initialize the order to close.
        let mut order: Option<OpenedOrder> = None;

        // Traverse each order and find the one to close.
        for opened_order in opened_orders {
            // Check if the current opened order is the order to close.
            if opened_order.ticker == order_removal.ticker
                && opened_order.side == order_removal.direction
                && opened_order.trigger_price == trigger_price_string
            {
                // It is the order to close.
                order = Some(opened_order);

                // The order is found, no need to continue searching.
                break;
            } // end if
        } // end for

        // Check if the order to close was found.
        if order.is_none() {
            // The order was not found.
            // Inform a client about it.
            return Err(AppError::new("The order to remove was not found"));
        } // end if

        // Safely unwrap the order.
        let order = order.unwrap();

        // Form a request to cancel an order on perpetual swaps.
        let post_request = json!({
            "category": "linear",
            "symbol": order.ticker,
            "orderId": order.order_id
        }); // end json!

        // Send a request to cancel an order on perpetual swaps.
        let response =
            ExchangeService::send_robust_post_request(&client, "/v5/order/cancel", &post_request)
                .await?;

        ExchangeService::validate_exchange_response(&response)
    } // end fn cancel_order

    /// This function verifies if the position can be opened.
    /// It is kind of a safety guard to prevent unexpected losses.
    /// It returns True, if the position opening passed the verification.
    /// False - otherwise.
    pub async fn verify_position(position_opening: &PositionOpening) -> Result<(), AppError> {
        // Make sure the risk is not greater than 1.
        if position_opening.risk > dec!(1.0) || position_opening.risk < dec!(0.0) {
            // It is not safe to get into such a position.
            return Err(AppError::new("The risk must be in the rage from 0 to 1"));
        } // end if

        // The verification is passed, allow to proceed.
        Ok(())
    } // end fn verify_position

    /// This function places a partial take profit order.
    pub async fn place_partial_take_profit_order(
        take_profit_order: Arc<TakeProfitPlacement>,
        user: User,
    ) -> Result<(), AppError> {
        // Get user credentials.
        let api_key = user.exchange_api_key.unwrap();
        let api_secret = user.exchange_api_secret.unwrap();

        // Get an exchange client.
        let client = ExchangeService::get_client(api_key, api_secret);

        // What position to close (hedge-short/hedge-long).
        let positions_idx: i32;
        let position_side: &str;

        // Determine the values and directions for the order.
        if take_profit_order.direction.to_lowercase() == "long"
            || take_profit_order.direction == "Buy"
        {
            positions_idx = 1;
            position_side = "Buy";
        } else if take_profit_order.direction.to_lowercase() == "short"
            || take_profit_order.direction == "Sell"
        {
            positions_idx = 2;
            position_side = "Sell";
        } else {
            // It is an invalid position side.
            return Err(AppError::new("Invalid position side"));
        } // end if

        // Get all opened positions on perpetual swaps.
        let opened_positions = Self::get_all_opened_positions(&client).await?;

        // The position searched for.
        let mut cur_position: Option<OpenedPosition> = None;

        // Find the position to set the take profit for.
        for opened_position in opened_positions {
            // Check if the current position is the position to set the take profit for.
            if opened_position.ticker == take_profit_order.ticker
                && opened_position.side == position_side
            {
                // This is the position searched for.
                cur_position = Some(opened_position);

                // The position is found.
                break;
            } // end if
        } // end for

        // Check if the position was found.
        if cur_position.is_none() {
            // Position to set the take profit for was not found.
            return Err(AppError::new(
                "Position to set take profit for was not found.",
            )); // end return
        } // end if

        // Safely unwrap the position.
        let cur_position = cur_position.unwrap();

        // Get the quantity step of the contract.
        let qty_step = Self::get_quantity_step(&client, &take_profit_order.ticker).await?;

        // Calculate the quantity to fix.
        let qty = cur_position.qty * (take_profit_order.take_profit_percentage / dec!(100));

        // Adjust the quantity with regard to quantity step of the contract.
        let qty = (Decimal::floor(&(qty / qty_step)) * qty_step).max(qty_step);

        // Form a request to place a partial take profit order on perpetual swaps.
        let post_request = json!({
            "category": "linear",
            "symbol": take_profit_order.ticker,
            "takeProfit":  take_profit_order.take_profit_price,
            "tpSize": qty.to_string(),
            "positionIdx": positions_idx,
            "tpslMode": "Partial",
        }); // end json!

        // Send a request to place a partial take profit order on perpetual swaps.
        let response = ExchangeService::send_robust_post_request(
            &client,
            "/v5/position/trading-stop",
            &post_request,
        )
        .await?;

        ExchangeService::validate_exchange_response(&response)
    } // end fn place_partial_take_profit_order
} // end impl ExchangeServicePerpetual
