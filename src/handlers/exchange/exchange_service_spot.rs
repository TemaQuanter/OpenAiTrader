// This file contains all the functionality to interact with
// Bybit cryptocurrency exchange on stop.

use std::sync::Arc;

use crypto_botters::{
    bybit::{BybitHttpAuth, BybitOption},
    Client,
};
use serde_json::json;

use rust_decimal::Decimal;
use rust_decimal_macros::dec;

use crate::{
    db_objects::user_model::User,
    handlers::{
        bot::dto_spot::{
            ConditionalOrderRemoval, PositionClosure, PositionIncrease, PositionOpening,
            StopLossAdjustment,
        },
        exchange::dto_spot::OpenedOrder,
    },
    utils::{app_error::AppError, types_and_constants::SPOT_OPENED_POSITION_BIAS_VALUE},
};

use super::{dto_spot::OpenedPosition, exchange_service::ExchangeService};

/// This struct contains all the functionality to interact
/// with Bybit cryptocurrency exchange on spot.
pub struct ExchangeServiceSpot;

/// Implement all the functionality to interact with
/// Bybit cryptocurrency exchange on spot.
impl ExchangeServiceSpot {
    /// This function returns all the current opened
    /// spot positions on UTA.
    pub async fn _get_all_opened_positions(
        client: &Client,
    ) -> Result<Vec<OpenedPosition>, AppError> {
        // Send a request and get the information about opened positions.
        let response: serde_json::Value = client
            .get(
                "/v5/asset/transfer/query-account-coins-balance",
                Some(&[("accountType", "UNIFIED")]),
                [BybitOption::HttpAuth(BybitHttpAuth::V3AndAbove)],
            )
            .await
            .map_err(|error| AppError::new(error.to_string()))?;

        // Validate exchange response.
        ExchangeService::validate_exchange_response(&response)?;

        // Parse the data into a DTO.

        // Convert the response into an array of opened positions.
        let opened_positions_raw = response["result"]["balance"].as_array().unwrap();

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

    /// This function returns a specific
    /// spot position on UTA.
    pub async fn get_opened_position(
        client: &Client,
        coin: impl AsRef<str>,
    ) -> Result<OpenedPosition, AppError> {
        // Send a request and get the information about opened positions.
        let response: serde_json::Value = client
            .get(
                "/v5/asset/transfer/query-account-coins-balance",
                Some(&[("accountType", "UNIFIED"), ("coin", coin.as_ref())]),
                [BybitOption::HttpAuth(BybitHttpAuth::V3AndAbove)],
            )
            .await
            .map_err(|error| AppError::new(error.to_string()))?;

        // Validate exchange response.
        ExchangeService::validate_exchange_response(&response)?;

        // Parse the data into a DTO.

        // Convert the response into an array of opened positions.
        let opened_positions_raw = response["result"]["balance"].as_array().unwrap();

        // Get the opened position.
        let opened_position = if opened_positions_raw.is_empty() {
            // The position is not opened.
            // Assign a default value.
            OpenedPosition {
                coin: coin.as_ref().to_string(),
                transfer_balance: dec!(0),
                wallet_balance: dec!(0),
            } // end OpenedPosition
        } else {
            // Parse an opened position into a struct.
            Self::parse_opened_position(&opened_positions_raw[0])
        }; // end if

        Ok(opened_position)
    } // end fn get_opened_position

    /// This function returns all opened orders on spot in USDT.
    pub async fn get_all_opened_orders(client: &Client) -> Result<Vec<OpenedOrder>, AppError> {
        // Send a request and get the information about opened orders.
        let response: serde_json::Value = client
            .get(
                "/v5/order/realtime",
                Some(&[("category", "spot")]),
                [BybitOption::HttpAuth(BybitHttpAuth::V3AndAbove)],
            )
            .await
            .map_err(|error| AppError::new(error.to_string()))?;

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
            coin: raw_opened_position["coin"].to_string().replace("\"", ""),
            transfer_balance: Decimal::from_str_exact(
                &raw_opened_position["transferBalance"]
                    .to_string()
                    .replace("\"", ""),
            )
            .unwrap_or(dec!(-1.0)),
            wallet_balance: Decimal::from_str_exact(
                &raw_opened_position["walletBalance"]
                    .to_string()
                    .replace("\"", ""),
            )
            .unwrap_or(dec!(-1.0)),
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
            qty: Decimal::from_str_exact(&raw_opened_order["qty"].to_string().replace("\"", ""))
                .unwrap(),
            trigger_price: Decimal::from_str_exact(
                &raw_opened_order["triggerPrice"]
                    .to_string()
                    .replace("\"", ""),
            )
            .unwrap(),
            stop_order_type: raw_opened_order["stopOrderType"]
                .to_string()
                .replace("\"", ""),
            order_id: raw_opened_order["orderId"].to_string().replace("\"", ""),
        }; // end opened_order

        opened_order
    } // end fn parse_opened_order

    /// This function retrieves the current price of an instrument
    /// on spot in USDT.
    pub async fn get_instrument_ask_price(
        client: &Client,
        ticker: impl AsRef<str>,
    ) -> Result<Decimal, AppError> {
        // Send a request and get the information about an instrument.
        let response: serde_json::Value = client
            .get(
                "/v5/market/tickers",
                Some(&[("category", "spot"), ("symbol", ticker.as_ref())]),
                [BybitOption::HttpAuth(BybitHttpAuth::V3AndAbove)],
            )
            .await
            .map_err(|error| AppError::new(error.to_string()))?;

        // Validate exchange response.
        ExchangeService::validate_exchange_response(&response)?;

        // Get the current market price.
        let market_price = Decimal::from_str_exact(
            &response["result"]["list"][0]["ask1Price"]
                .to_string()
                .replace("\"", ""),
        )
        .unwrap();

        Ok(market_price)
    } // end get_instrument_ask_price

    /// This function retrieves the price step for a particular instrument
    /// on spot in USDT.
    pub async fn get_price_step(
        client: &Client,
        ticker: impl AsRef<str>,
    ) -> Result<Decimal, AppError> {
        // Send a request and get the information about an instrument.
        let response: serde_json::Value = client
            .get(
                "/v5/market/instruments-info",
                Some(&[("category", "spot"), ("symbol", ticker.as_ref())]),
                [BybitOption::HttpAuth(BybitHttpAuth::V3AndAbove)],
            )
            .await
            .map_err(|error| AppError::new(error.to_string()))?;

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

    /// This function retrieves the quantity step for a coin
    /// on a cryptocurrency exchange.
    pub async fn get_quantity_step(
        client: &Client,
        ticker: impl AsRef<str>,
    ) -> Result<Decimal, AppError> {
        // Send a request and get the information about an instrument.
        let response: serde_json::Value = client
            .get(
                "/v5/market/instruments-info",
                Some(&[("category", "spot"), ("symbol", ticker.as_ref())]),
                [BybitOption::HttpAuth(BybitHttpAuth::V3AndAbove)],
            )
            .await
            .map_err(|error| AppError::new(error.to_string()))?;

        // Validate the response from exchange.
        ExchangeService::validate_exchange_response(&response)?;

        // Get the quantity step for the instrument.
        let quantity_step = Decimal::from_str_exact(
            &response["result"]["list"][0]["lotSizeFilter"]["basePrecision"]
                .to_string()
                .replace("\"", ""),
        )
        .unwrap();

        Ok(quantity_step)
    } // end fn get_quantity_step

    /// This function opens a new position on spot
    /// in USDT.
    pub async fn open_position(
        position_opening: Arc<PositionOpening>,
        user: User,
    ) -> Result<(), AppError> {
        // Get cloned user.
        let cloned_user = user.clone();

        // Get user credentials.
        let api_key = user.exchange_api_key.unwrap();
        let api_secret = user.exchange_api_secret.unwrap();

        // Get an exchange client.
        let client = ExchangeService::get_client(api_key, api_secret);

        // Make a copy of the position to modify it later.
        let mut position_opening = (*position_opening).clone();

        // These parameters are required to place a conditional order.
        let trigger_price: Option<Decimal>;
        let order_filter: &str;

        // If the lower bound price matches the upper bound price,
        // then it is most likely to be a conditional order.
        if position_opening.lower_bound_entry == position_opening.upper_bound_entry {
            position_opening.order_type = "conditional".to_string();

            // Set the order filter to create a conditional order.
            order_filter = "StopOrder";
        } else {
            // Set the order filter to create a market order.
            order_filter = "Order";
        } // end if

        // Make sure that lower bound entry is always equal or less than upper bound entry.
        if position_opening.lower_bound_entry > position_opening.upper_bound_entry {
            // Swap them.
            let temp = position_opening.lower_bound_entry;
            position_opening.lower_bound_entry = position_opening.upper_bound_entry;
            position_opening.upper_bound_entry = temp;
        } // end if

        // Get the current price of the coin.
        // If the order type is conditional,
        // then the variable is equal to the entry price.
        let cur_coin_price = match position_opening.order_type.to_lowercase().as_ref() {
            "market" => {
                // Set parameters required to place a conditional order to None.
                trigger_price = None;

                Self::get_instrument_ask_price(&client, &position_opening.ticker)
                    .await
                    .map_err(|error| AppError::new(error.to_string()))?
            } // end "market"
            "conditional" => {
                // Set parameters required to place a conditional order.

                // Set a trigger price.
                trigger_price = Some(position_opening.lower_bound_entry);

                position_opening.lower_bound_entry
            } // end "conditional"
            _ => return Err(AppError::new("Failed to parse order type")),
        }; // end match

        // Check if it is a market order.
        if position_opening.order_type == "market" {
            // Get a coin from the ticker.
            let coin = match position_opening.ticker.strip_suffix("USDT") {
                Some(coin) => coin,
                None => {
                    return Err(AppError::new("Wrong ticker provided."));
                } // end None
            }; // end match

            // Get all positions currently opened for a user.
            let opened_position = Self::get_opened_position(&client, coin).await?;

            // Check if the current position is the position the function
            // is trying to open.
            // The second condition deals with a bias, when it was impossible
            // to sell all coins and there is a tiny remainder.
            if opened_position.coin == coin
                && cur_coin_price * opened_position.wallet_balance > SPOT_OPENED_POSITION_BIAS_VALUE
            {
                // This is a position that the function is trying to open.
                // Stop the function execution.
                return Err(AppError::new("The position is already opened"));
            } // end if
        } // end if

        // Get the current market price of the coin.
        let cur_market_price = Self::get_instrument_ask_price(&client, &position_opening.ticker)
            .await
            .map_err(|error| AppError::new(error.to_string()))?;

        // If the current market price is lower than the lower bound of the entry price
        // for long position or higher than the upper bound for short position,
        // then it is unsafe to get into the position.
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

        // Calculate the value of coins to buy/sell in USDT.
        let coins_value =
            user.deposit.unwrap() * (position_opening.percentage_from_deposit / dec!(100));

        // Calculate the quantity of coins to buy/sell.
        let qty = coins_value / cur_coin_price;

        // Get the quantity step for the instrument.
        let quantity_step = &Self::get_quantity_step(&client, &position_opening.ticker)
            .await
            .map_err(|error| AppError::new(error.to_string()))?;

        // Adjust the quantity according to the quantity step.
        let qty = Decimal::floor(&(qty / quantity_step)) * quantity_step;

        // Create the POST request for position opening.
        let post_request: serde_json::Value = json!({
            "category": "spot",
            "symbol": &position_opening.ticker,
            "side": "Buy",
            "orderType": "Market",
            "qty": qty.to_string(),
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
            "orderFilter": order_filter,
            "marketUnit": "baseCoin"
        }); // end post_request

        // Verify that the position is safe to open.
        // This function will generate and return an error,
        // if it is unsafe to open a position.
        Self::verify_position(&position_opening).await?;

        // Try to unwrap take profit boundaries.

        // Send a request to open a position on spot.
        let response: serde_json::Value = client
            .post(
                "/v5/order/create",
                Some(post_request),
                [BybitOption::HttpAuth(BybitHttpAuth::V3AndAbove)],
            )
            .await
            .map_err(|error| AppError::new(error.to_string()))?;

        // Check if it was a conditional order, then it is impossible to set
        // partial take profits and it is possible to exit the function right here.
        if position_opening.order_type == "conditional" {
            return ExchangeService::validate_exchange_response(&response);
        } // end if

        // Set a stop loss.

        // Validate that the order was executed successfully.
        let res = Self::adjust_stop_loss(
            Arc::new(StopLossAdjustment {
                ticker: position_opening.ticker.to_string(),
                new_stop: position_opening.stop_loss.clone(),
            }),
            cloned_user,
        )
        .await;

        // If there was an error, finish function execution.
        if res.is_err() {
            return Err(res.unwrap_err());
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
            // Set one take profit on the entire position.
            // Self::place_partial_take_profit_order(
            //     &client,
            //     &position_opening.ticker,
            //     take_profit_lower_bound.to_string(),
            //     qty.to_string(),
            // )
            // .await?;

            return ExchangeService::validate_exchange_response(&response);
        } // end if

        // The number of partial take profits to set.
        // It is float, as it is necessary for further calculations.
        let number_take_profits = dec!(4.0);

        // Calculate the quantity to close on each partial take profit.
        let _partial_qty =
            Decimal::floor(&((qty / number_take_profits) / quantity_step)) * quantity_step;

        // Calculate the price step to close the orders.
        let price_step =
            (take_profit_upper_bound - take_profit_lower_bound).abs() / number_take_profits;

        // Get the minimum quote step.
        let quote_step = Self::get_price_step(&client, &position_opening.ticker)
            .await
            .map_err(|error| AppError::new(error.to_string()))?;

        // Adjust price step.
        let price_step = Decimal::floor(&(price_step / quote_step)) * quote_step;

        // A variable to keep track of the price in the loop.
        // It is necessary to take the minimum to calculate take profits
        // for short.
        let mut cur_price = take_profit_lower_bound;

        // Setup a grid of smaller take profits on the way to the main goal.
        for _ in 0..number_take_profits.to_string().parse::<f64>().unwrap() as i32 {
            // Set a new conditional order with take profit.
            // Self::place_partial_take_profit_order(
            //     &client,
            //     &position_opening.ticker,
            //     cur_price.to_string(),
            //     partial_qty.to_string(),
            // )
            // .await?;

            // Get a new price for the next take profit.
            cur_price += price_step;
        } // end while

        ExchangeService::validate_exchange_response(&response)
    } // end fn open_position

    /// This function increases the size of an opened position on spot.
    pub async fn increase_position(
        position_increase_order: Arc<PositionIncrease>,
        user: User,
    ) -> Result<(), AppError> {
        // Check if position to increase is not greater than an adequate value.
        if position_increase_order.percentage_to_increase > dec!(300)
            && position_increase_order.percentage_from == "position"
            || position_increase_order.percentage_to_increase > dec!(5)
                && position_increase_order.percentage_from == "deposit"
        {
            return Err(AppError::new("Inadequate risk"));
        } // end if

        // Get a cloned user to use it for stop loss setup and adjustment.
        let cloned_user = user.clone();

        // Get user credentials.
        let api_key = user.exchange_api_key.unwrap();
        let api_secret = user.exchange_api_secret.unwrap();

        // Get an exchange client.
        let client = ExchangeService::get_client(api_key, api_secret);

        // Get a coin from the ticker.
        let coin = match position_increase_order.ticker.strip_suffix("USDT") {
            Some(coin) => coin,
            None => {
                return Err(AppError::new("Wrong ticker provided."));
            }
        }; // end None

        // Initialize the position to increase;
        let position_to_increase: OpenedPosition =
            Self::get_opened_position(&client, coin)
                .await
                .map_err(|error| AppError::new(error.to_string()))?;

        // Get current market price of the asset.
        let cur_coin_price =
            Self::get_instrument_ask_price(&client, &position_increase_order.ticker)
                .await
                .map_err(|error| AppError::new(error.to_string()))?;

        // Check whether or not the current position is opened.
        if position_to_increase.wallet_balance * cur_coin_price < SPOT_OPENED_POSITION_BIAS_VALUE {
            // Position is not opened, nothing to increase.
            return Err(AppError::new("Position to increase is not opened."));
        } // end if

        // Extra quantity of coins to buy.
        let extra_qty: Decimal;

        // Calculate the quantity it is necessary to buy to increase the position size.
        // Check whether it is necessary to increase the position size based on the opened
        // position or deposit size.
        if position_increase_order.percentage_from == "position" {
            // Calculate the quantity it is necessary to buy to increase the position size.
            extra_qty = position_to_increase.wallet_balance
                * (position_increase_order.percentage_to_increase / dec!(100.0));
        } else if position_increase_order.percentage_from == "deposit" {
            // Make sure that user has a deposit set.
            if user.deposit.is_none() {
                return Err(AppError::new("Deposit is not set"));
            } // end if

            // Get deposit size.
            let deposit = user.deposit.unwrap();

            // Calculate the quantity it is necessary to buy to increase the position size.
            extra_qty = (deposit * (position_increase_order.percentage_to_increase / dec!(100.0)))
                / cur_coin_price;
        } else {
            // Invalid signal.
            return Err(AppError::new(
                "percentage_from can only be \"deposit\" or \"position\"",
            )); // end return
        } // end if

        // Get the price step of the asset on spot trading.
        let qty_step = Self::get_quantity_step(&client, &position_increase_order.ticker).await?;

        // Round the quantity.
        let extra_qty = Decimal::floor(&(extra_qty / qty_step)) * qty_step;

        // Place an order to increase the position size on spot.
        let response: serde_json::Value = client
            .post(
                "/v5/order/create",
                Some(json!({
                    "category": "spot",
                    "symbol": position_increase_order.ticker,
                    "side": "Buy",
                    "orderType": "Market",
                    "qty": extra_qty.to_string(),
                    "orderFilter": "Order",
                    "marketUnit": "baseCoin"
                })),
                [BybitOption::HttpAuth(BybitHttpAuth::V3AndAbove)],
            )
            .await
            .map_err(|error| AppError::new(error.to_string()))?;

        // Get the response from exchange.
        let res = ExchangeService::validate_exchange_response(&response);

        // If there was an error, stop function execution.
        if res.is_err() {
            return Err(res.unwrap_err());
        } // end if

        // Adjust a stop loss order or set a new one.
        Self::adjust_stop_loss(
            Arc::new(StopLossAdjustment {
                ticker: position_increase_order.ticker.to_owned(),
                new_stop: position_increase_order.new_stop,
            }),
            cloned_user,
        )
        .await
        .map_err(|error| AppError::new(error.to_string()))?;

        Ok(())
    } // end fn increase_position

    /// This function closes a part of a position on spot.
    /// in USDT.
    pub async fn close_part_of_position(
        position_closure: Arc<PositionClosure>,
        user: User,
    ) -> Result<(), AppError> {
        // Check that position closure percentage is not greater than 100.
        if position_closure.percentage_to_close > dec!(100) {
            return Err(AppError::new("Percentage to close cannot exceed 100%"));
        } // end if

        // Get a clone of the user to be able to adjust a stop loss.
        // let user_clone = user.clone();

        // Get user credentials.
        let api_key = user.exchange_api_key.unwrap();
        let api_secret = user.exchange_api_secret.unwrap();

        // Get an exchange client.
        let client = ExchangeService::get_client(api_key, api_secret);

        // Get a coin from the ticker.
        let coin = match position_closure.ticker.strip_suffix("USDT") {
            Some(coin) => coin,
            None => {
                return Err(AppError::new("Wrong ticker provided."));
            } // end None
        }; // end match

        // Get the position, which is necessary to close.
        let opened_position = Self::get_opened_position(&client, coin).await?;

        // Get current coin price.
        let cur_coin_price = Self::get_instrument_ask_price(&client, &position_closure.ticker)
            .await
            .map_err(|error| AppError::new(error.to_string()))?;

        // Check if there is anything to close.
        if opened_position.wallet_balance * cur_coin_price < SPOT_OPENED_POSITION_BIAS_VALUE {
            return Err(AppError::new("Position to close is not found"));
        } // end if

        // Get the quantity step of the coin.
        let quantity_step =
            Self::get_quantity_step(&client, position_closure.ticker.to_string()).await?;

        // Calculate the quantity to fix.
        let qty =
            opened_position.wallet_balance * (position_closure.percentage_to_close / dec!(100.0));

        // Adjust the quantity so to fit the quantity step restrictions.
        let qty = Decimal::floor(&(qty / quantity_step)) * quantity_step;

        // Decrease the amount of stop loss (stop loss orders allocate coins, so it is
        // impossible to do anything with them).

        // Get all the opened orders.
        let opened_orders = Self::get_all_opened_orders(&client).await?;

        // An order that it is required to adjust a stop loss of.
        let mut cur_order: Option<OpenedOrder> = None;

        // Get only one order (full stop loss of the required coin).
        for opened_order in opened_orders {
            // Check if current order is the order that is searched for.
            if opened_order.ticker == position_closure.ticker
                && opened_order.side == "Sell"
                && opened_order.stop_order_type == "tpslOrder"
                && opened_order.trigger_price < cur_coin_price
            {
                // This is the required order.
                cur_order = Some(opened_order);
                break;
            } // end if
        } // end for

        // If the order was found, reduce the amount of it.
        if cur_order.is_some() {
            let cur_order = cur_order.unwrap();

            // Check if it necessary to close the position entirely.
            if position_closure.percentage_to_close == dec!(100) {
                // Remove the order.
                let response: serde_json::Value = client
                    .post(
                        "/v5/order/cancel",
                        Some(json!({
                            "category": "spot",
                            "symbol": position_closure.ticker,
                            "orderId": cur_order.order_id
                        })),
                        [BybitOption::HttpAuth(BybitHttpAuth::V3AndAbove)],
                    )
                    .await
                    .map_err(|error| AppError::new(error.to_string()))?;

                // Get the response from exchange.
                let res = ExchangeService::validate_exchange_response(&response);

                // If there was an error, stop function execution.
                if res.is_err() {
                    return Err(res.unwrap_err());
                } // end if
            } else {
                // Amend the order.
                let response: serde_json::Value = client
                    .post(
                        "/v5/order/amend",
                        Some(json!({
                            "category": "spot",
                            "symbol": position_closure.ticker,
                            "orderId": cur_order.order_id,
                            "qty": cur_order.qty - qty
                        })),
                        [BybitOption::HttpAuth(BybitHttpAuth::V3AndAbove)],
                    )
                    .await
                    .map_err(|error| AppError::new(error.to_string()))?;

                // Get the response from exchange.
                let res = ExchangeService::validate_exchange_response(&response);

                // If there was an error, stop function execution.
                if res.is_err() {
                    return Err(res.unwrap_err());
                } // end if
            } // end if
        } // end if

        // Send a request to adjust the position size on spot.
        let response: serde_json::Value = client
            .post(
                "/v5/order/create",
                Some(json!({
                    "category": "spot",
                    "symbol": position_closure.ticker,
                    "side": "Sell",
                    "orderType": "Market",
                    "qty": qty.to_string(),
                    "orderFilter": "Order",
                    "marketUnit": "baseCoin"
                })),
                [BybitOption::HttpAuth(BybitHttpAuth::V3AndAbove)],
            )
            .await
            .map_err(|error| AppError::new(error.to_string()))?;

        ExchangeService::validate_exchange_response(&response)
    } // end fn close_part_of_position

    /// This function adjusts the stop loss of a particular position on
    /// spot in USDT.
    pub async fn adjust_stop_loss(
        stop_loss_adjustment: Arc<StopLossAdjustment>,
        user: User,
    ) -> Result<(), AppError> {
        // Get user credentials.
        let api_key = user.exchange_api_key.unwrap();
        let api_secret = user.exchange_api_secret.unwrap();

        // Get an exchange client.
        let client = ExchangeService::get_client(api_key, api_secret);

        // Get a coin from the ticker.
        let coin = match stop_loss_adjustment.ticker.strip_suffix("USDT") {
            Some(coin) => coin,
            None => {
                return Err(AppError::new("Wrong ticker provided."));
            } // end None
        };

        // Get all the currently opened positions.
        let opened_position = Self::get_opened_position(&client, coin).await?;

        // Get current price of the asset.
        let cur_coin_price = Self::get_instrument_ask_price(&client, &stop_loss_adjustment.ticker)
            .await
            .map_err(|error| AppError::new(error.to_string()))?;

        // Check if the current position was found.
        if opened_position.wallet_balance * cur_coin_price < SPOT_OPENED_POSITION_BIAS_VALUE {
            // The position was not found.
            return Err(AppError::new(
                "The position to adjust the stop loss of was not found",
            )); // end return
        } // end if

        // Get all the opened orders.
        let opened_orders = Self::get_all_opened_orders(&client).await?;

        // An order that it is required to adjust a stop loss of.
        let mut cur_order: Option<OpenedOrder> = None;

        // Get only one order (full stop loss of the required coin).
        for opened_order in opened_orders {
            // Check if current order is the order that is searched for.
            if opened_order.ticker == stop_loss_adjustment.ticker
                && opened_order.side == "Sell"
                && opened_order.stop_order_type == "tpslOrder"
                && opened_order.trigger_price < cur_coin_price
            {
                // This is the required order.
                cur_order = Some(opened_order);
                break;
            } // end if
        } // end for

        // Get the quantity step for the instrument.
        let quantity_step = &Self::get_quantity_step(&client, &stop_loss_adjustment.ticker)
            .await
            .map_err(|error| AppError::new(error.to_string()))?;

        // Adjust the quantity according to the quantity step.
        let qty =
            Decimal::floor(&(&opened_position.wallet_balance / quantity_step)) * quantity_step;

        // If the order was not found, neither stop loss price was provided,
        // then just finish function execution.
        if cur_order.is_none() && stop_loss_adjustment.new_stop.is_none() {
            return Ok(());
        } // end if

        // If the order was not found, set it up.
        if cur_order.is_none() && stop_loss_adjustment.new_stop.is_some() {
            let response: serde_json::Value = client
                .post(
                    "/v5/order/create",
                    Some(json!({
                        "category": "spot",
                        "qty": qty.to_string(),
                        "orderType": "Market",
                        "side": "Sell",
                        "symbol": stop_loss_adjustment.ticker,
                        "triggerPrice": stop_loss_adjustment.new_stop.unwrap().to_string(),
                        "orderFilter": "tpslOrder",
                        "marketUnit": "baseCoin"
                    })),
                    [BybitOption::HttpAuth(BybitHttpAuth::V3AndAbove)],
                )
                .await
                .map_err(|error| AppError::new(error.to_string()))?;

            return ExchangeService::validate_exchange_response(&response);
        } // end if

        // If the order already exists, safely unwrap the order.
        let cur_order = cur_order.unwrap();

        let mut post_request = json!({
            "category": "spot",
            "symbol": stop_loss_adjustment.ticker,
            "orderId": cur_order.order_id,
            "qty": qty.to_string()
        });

        // Check if a new stop is provided.
        if stop_loss_adjustment.new_stop.is_some() {
            // Add it to the post request.
            post_request["triggerPrice"] =
                serde_json::Value::String(stop_loss_adjustment.new_stop.unwrap().to_string());
        } // end if

        // Send a request to adjust a stop loss of the order on spot.
        let response: serde_json::Value = client
            .post(
                "/v5/order/amend",
                Some(post_request),
                [BybitOption::HttpAuth(BybitHttpAuth::V3AndAbove)],
            )
            .await
            .map_err(|error| AppError::new(error.to_string()))?;

        ExchangeService::validate_exchange_response(&response)
    } // end fn adjust_stop_loss

    /// This function cancels an opened order on spot.
    pub async fn cancel_order(
        order_removal: Arc<ConditionalOrderRemoval>,
        user: User,
    ) -> Result<(), AppError> {
        // Get user credentials.
        let api_key = user.exchange_api_key.unwrap();
        let api_secret = user.exchange_api_secret.unwrap();

        // Get an exchange client.
        let client = ExchangeService::get_client(api_key, api_secret);

        // Get all opened orders on spot.
        let opened_orders = Self::get_all_opened_orders(&client).await?;

        // Initialize the order to close.
        let mut order: Option<OpenedOrder> = None;

        // Traverse each order and find the one to close.
        for opened_order in opened_orders {
            // Check if the current opened order is the order to close.
            if opened_order.ticker == order_removal.ticker
                && opened_order.trigger_price == order_removal.trigger_price
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

        // Send a request to cancel an order on spot.
        let response: serde_json::Value = client
            .post(
                "/v5/order/cancel",
                Some(json!({
                    "category": "spot",
                    "symbol": order.ticker,
                    "orderId": order.order_id
                })),
                [BybitOption::HttpAuth(BybitHttpAuth::V3AndAbove)],
            )
            .await
            .map_err(|error| AppError::new(error.to_string()))?;

        ExchangeService::validate_exchange_response(&response)
    } // end fn cancel_order

    /// This function verifies if the position can be opened.
    /// It is kind of a safety guard to prevent unexpected losses.
    /// It returns True, if the position opening passed the verification.
    /// False - otherwise.
    pub async fn verify_position(position_opening: &PositionOpening) -> Result<(), AppError> {
        // Make sure the percentage from deposit is not greater than 5%.
        if position_opening.percentage_from_deposit > dec!(5.0)
            || position_opening.percentage_from_deposit < dec!(0.0)
        {
            // It is not safe to get into such a position.
            return Err(AppError::new(
                "The percentage from deposit must be between 0% and 5%",
            ));
        } // end if

        // The verification is passed, allow to proceed.
        Ok(())
    } // end fn verify_position

    /// This function places a partial take profit order.
    pub async fn _place_partial_take_profit_order(
        client: &Client,
        ticker: &String,
        trigger_price: String,
        qty: String,
    ) -> Result<(), AppError> {
        // Send a request to place a partial take profit order on spot.
        let response: serde_json::Value = client
            .post(
                "/v5/order/create",
                Some(json!({
                    "category": "spot",
                    "side": "Sell",
                    "orderType": "Market",
                    "symbol": ticker,
                    "triggerPrice":  trigger_price,
                    "qty": qty.to_string(),
                    "orderFilter": "StopOrder",
                    "marketUnit": "baseCoin"
                })),
                [BybitOption::HttpAuth(BybitHttpAuth::V3AndAbove)],
            )
            .await
            .map_err(|error| AppError::new(error.to_string()))?;

        ExchangeService::validate_exchange_response(&response)
    } // end fn place_partial_take_profit_order
} // end impl ExchangeServiceSpot
