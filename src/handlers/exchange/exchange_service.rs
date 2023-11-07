// This file contains all the functionality to interact with
// Bybit cryptocurrency exchange.

use std::{future::Future, sync::Arc};

use crypto_botters::{
    bybit::{BybitHttpAuth, BybitOption, BybitRequestError},
    Client,
};
use serde_json::json;

use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use tokio::task::JoinHandle;

use crate::{
    db_objects::{status_model::StatusId, user_model::User},
    handlers::{
        bot::{
            bot_service::BotState,
            dto::{PositionClosure, PositionOpening, StopLossAdjustment},
        },
        exchange::dto::OpenedOrder,
    },
    utils::{app_error::AppError, database_service::DatabaseService},
};

use super::dto::OpenedPosition;

/// This struct contains all the functionality to interact
/// with Bybit cryptocurrency exchange.
pub struct ExchangeService;

/// Implement all the functionality to interact with
/// Bybit cryptocurrency exchange.
impl ExchangeService {
    /// This function creates an exchange client.
    pub fn get_client(api: String, secret: String) -> Client {
        let mut client = Client::new();

        // Authorize the client.
        client.update_default_option(BybitOption::Key(api));
        client.update_default_option(BybitOption::Secret(secret));

        client
    } // end fn get_client

    /// This function checks whether or not a candidate user is
    /// eligible to trade.
    ///
    /// The function returns true - if the user is eligible to trade,
    /// The function returns false otherwise.
    async fn is_eligible_to_trade(candidate: &User) -> bool {
        // If the user did not allow the bot to trade for them,
        // then they are not eligible.
        if !candidate.bot_active {
            return false;
        } // end if

        // Check if the user provided their exchange credentials.
        if candidate.exchange_api_key.is_none() || candidate.exchange_api_secret.is_none() {
            return false;
        } // end if

        // Get candidate's credentials.
        let exchange_api_key = candidate.exchange_api_key.clone().unwrap();
        let exchange_api_secret = candidate.exchange_api_secret.clone().unwrap();

        // Check if the user's account is reachable.
        let client = Self::get_client(exchange_api_key, exchange_api_secret);

        // Try to get the candidate's balance.
        let balance = match Self::get_uta_usdt_balance(&client).await {
            Ok(balance) => balance,
            Err(_error) => {
                // The candidate's account is unreachable.
                return false;
            }
        };

        // Check if their balance is over some minimum limit.
        if balance < dec!(500) {
            return false;
        } // end if

        // The candidate passed all checks and is eligible to trade.
        true
    } // end fn is_eligible_to_trade

    /// This function gets an array of users who are eligible to trade
    async fn get_users_eligible_to_trade(bot_state: BotState) -> Result<Vec<User>, AppError> {
        // Try to establish a connection to the database.
        let conn = &mut match DatabaseService::try_get_pool_connection(
            &bot_state.pg_connection_pool,
        )
        .await
        {
            Ok(conn) => conn,
            Err(error) => {
                log::error!("{}", error);

                // Return an error that it was impossible to connect to the database.
                return Err(AppError::new(
                    "Failed to connect to the database to open a new position",
                ));
            } // end Err
        }; // end match

        // Retrieve all approved users from the database.
        let approved_users =
            User::get_users_with_status(&StatusId::Approved, conn).map_err(|_error| {
                AppError::new("Failed to retrieve the users for opening a new position")
            })?;
        // Create an array of eligible to trade users.

        let mut eligible_users: Vec<User> = Vec::new();

        // Traverse the array of users and select only those who are
        // eligible for trading.
        for candidate in approved_users {
            // Check if the candidate is eligible to trade.
            if Self::is_eligible_to_trade(&candidate).await {
                // Add the candidate to eligible users.
                eligible_users.push(candidate);
            } // end if
        } // end for

        Ok(eligible_users)
    } // end get_users_eligible_to_trade

    /// This function returns the amount of USDT on Unified Trading Account.
    /// Value is returned in USDT.
    pub async fn get_uta_usdt_balance(client: &Client) -> Result<Decimal, AppError> {
        // Get the information about USDT balance on UTA.
        let response: serde_json::Value = client
            .get(
                "/v5/account/wallet-balance",
                Some(&[("accountType", "UNIFIED"), ("coin", "USDT")]),
                [BybitOption::HttpAuth(BybitHttpAuth::V3AndAbove)],
            )
            .await
            .map_err(|error| AppError::new(error.to_string()))?;

        // Return the balance of the wallet in USDT.
        Ok(Decimal::from_str_exact(
            &response["result"]["list"][0]["coin"][0]["equity"]
                .to_string()
                .replace("\"", ""),
        )
        .unwrap())
    } // end fn get_uta_usdt_balance

    /// This function returns all the current opened
    /// perpetual swaps positions on UTA.
    pub async fn get_all_opened_positions(
        client: &Client,
    ) -> Result<Vec<OpenedPosition>, AppError> {
        // Send a request and get the information about opened positions.
        let response: serde_json::Value = client
            .get(
                "/v5/position/list",
                Some(&[("category", "linear"), ("settleCoin", "USDT")]),
                [BybitOption::HttpAuth(BybitHttpAuth::V3AndAbove)],
            )
            .await
            .map_err(|error| AppError::new(error.to_string()))?;

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
        // Send a request and get the information about opened orders.
        let response: serde_json::Value = client
            .get(
                "/v5/order/realtime",
                Some(&[("category", "linear"), ("settleCoin", "USDT")]),
                [BybitOption::HttpAuth(BybitHttpAuth::V3AndAbove)],
            )
            .await
            .map_err(|error| AppError::new(error.to_string()))?;

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

    /// This function executes an exchange order on perpetual swaps for every single user
    /// asynchronously.
    pub async fn execute_exchange_order_for_all_users<T, Out>(
        exchange_order: Arc<T>,
        func: fn(Arc<T>, User) -> Out,
        bot_state: BotState,
    ) -> Result<(i32, i32), AppError>
    where
        T: Send + Sync + 'static,
        Out: Future<Output = Result<(), AppError>> + Send + 'static,
    {
        // Get an array of users who can trade.
        let eligible_users = Self::get_users_eligible_to_trade(bot_state).await?;

        // An array that stores all asynchronous tasks.
        let mut async_tasks: Vec<JoinHandle<Result<(), AppError>>> = Vec::new();

        // Spawn a task for each user to execute the orders asynchronously.
        for user in eligible_users {
            // Clone the reference to the position opening.
            let exchange_order_clone = Arc::clone(&exchange_order);

            // Create a new task.
            let new_task = tokio::spawn(async move {
                // Try to open a position on perpetual swaps.
                func(exchange_order_clone, user).await
            });

            // Add the task to the array of running tasks.
            async_tasks.push(new_task);
        } // end for

        // Count positive and negative results.
        let mut positive_results = 0;
        let mut negative_results = 0;

        // Wait for all tasks to complete.
        for tsk in async_tasks {
            if let Err(error) = tsk.await {
                // The task finished with error.
                negative_results += 1;

                log::error!("{}", error);
            } else {
                // The result was OK.
                positive_results += 1;
            } // end if
        } // end for

        return Ok((positive_results, negative_results));
    } // end fn execute_exchange_order_for_all_users

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
    pub async fn get_instrument_price_on_perpetual_swaps(
        client: &Client,
        ticker: impl AsRef<str>,
    ) -> Result<Decimal, BybitRequestError> {
        // Send a request and get the information about an instrument.
        let response: serde_json::Value = client
            .get(
                "/v5/market/tickers",
                Some(&[("category", "linear"), ("symbol", ticker.as_ref())]),
                [BybitOption::HttpAuth(BybitHttpAuth::V3AndAbove)],
            )
            .await?;

        // Get the current market price.
        let market_price = Decimal::from_str_exact(
            &response["result"]["list"][0]["markPrice"]
                .to_string()
                .replace("\"", ""),
        )
        .unwrap();

        Ok(market_price)
    } // end get_instrument_price_on_perpetual_swaps

    /// This function retrieves the price step for a particular instrument
    /// on perpetual swaps in USDT.
    pub async fn get_price_step_on_perpetual_swaps(
        client: &Client,
        ticker: impl AsRef<str>,
    ) -> Result<Decimal, AppError> {
        // Send a request and get the information about an instrument.
        let response: serde_json::Value = client
            .get(
                "/v5/market/instruments-info",
                Some(&[("category", "linear"), ("symbol", ticker.as_ref())]),
                [BybitOption::HttpAuth(BybitHttpAuth::V3AndAbove)],
            )
            .await
            .map_err(|error| AppError::new(error.to_string()))?;

        // Get the quantity step for the instrument.
        let price_step = Decimal::from_str_exact(
            &response["result"]["list"][0]["priceFilter"]["tickSize"]
                .to_string()
                .replace("\"", ""),
        )
        .unwrap();

        Ok(price_step)
    } // end fn get_price_set_on_perpetual_swaps

    /// This function retrieves the quantity step for a contract
    /// on a cryptocurrency exchange.
    pub async fn get_quantity_step_on_perpetual_swaps(
        client: &Client,
        ticker: impl AsRef<str>,
    ) -> Result<Decimal, AppError> {
        // Send a request and get the information about an instrument.
        let response: serde_json::Value = client
            .get(
                "/v5/market/instruments-info",
                Some(&[("category", "linear"), ("symbol", ticker.as_ref())]),
                [BybitOption::HttpAuth(BybitHttpAuth::V3AndAbove)],
            )
            .await
            .map_err(|error| AppError::new(error.to_string()))?;

        // Get the quantity step for the instrument.
        let quantity_step = Decimal::from_str_exact(
            &response["result"]["list"][0]["lotSizeFilter"]["qtyStep"]
                .to_string()
                .replace("\"", ""),
        )
        .unwrap();

        Ok(quantity_step)
    } // end fn get_quantity_set_on_perpetual_swaps

    /// This function opens a new position on perpetual swaps
    /// in USDT.
    pub async fn open_position_on_perpetual_swaps(
        position_opening: Arc<PositionOpening>,
        user: User,
    ) -> Result<(), AppError> {
        // Get user credentials.
        let api_key = user.exchange_api_key.unwrap();
        let api_secret = user.exchange_api_secret.unwrap();

        // Get an exchange client.
        let client = Self::get_client(api_key, api_secret);

        // Make a copy of the position to modify it later.
        let mut position_opening = (*position_opening).clone();

        // Position IDX is a special parameter that is required for hedge-mode.
        let position_idx: i32;

        // This parameters are required to place a conditional order.
        let trigger_direction: Option<i32>;
        let trigger_price: Option<Decimal>;

        // If the lower bound price matches the upper bound price,
        // then it is most likely to be a conditional order.
        if position_opening.lower_bound_entry == position_opening.upper_bound_entry {
            position_opening.order_type = "conditional".to_string();
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

                Self::get_instrument_price_on_perpetual_swaps(&client, &position_opening.ticker)
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

        // Get the current market price of the contract.
        let cur_market_price =
            Self::get_instrument_price_on_perpetual_swaps(&client, &position_opening.ticker)
                .await
                .map_err(|error| AppError::new(error.to_string()))?;

        // If the current market price is lower than the lower bound of the entry price
        // for long position or higher than the upper bound for short position,
        // then it is unsafe to get into the position.

        // This rule applies only to the market orders, not conditional ones.
        if position_opening.order_type == "market" {
            // Check whether it is a short or long position.
            if position_opening.direction == "Buy"
                && position_opening.lower_bound_entry > cur_market_price
            {
                return Err(AppError::new(
                    "The market price is lower than the minimum entry price",
                )); // end return
            } else if position_opening.direction == "Short"
                && position_opening.lower_bound_entry < cur_market_price
            {
                return Err(AppError::new(
                    "The market price is higher than the minimum entry price",
                )); // end return
            } // end if
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
        let quantity_step =
            &Self::get_quantity_step_on_perpetual_swaps(&client, &position_opening.ticker)
                .await
                .map_err(|error| AppError::new(error.to_string()))?;

        // Adjust the quantity according to the quantity step.
        let qty = Decimal::floor(&(qty / quantity_step)) * quantity_step;

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
        Self::set_hedge_mode_on_perpetual_swaps(&client, &position_opening.ticker)
            .await
            .map_err(|error| AppError::new(error.to_string()))?;

        // Setup the leverage.
        Self::set_leverage(&client, &position_opening.ticker, leverage).await?;

        // Create the POST request for position opening.
        let post_request: serde_json::Value = json!({
            "category": "linear",
            "symbol": &position_opening.ticker,
            "side": position_opening.direction,
            "orderType": "Market",
            "qty": qty.to_string(),
            "takeProfit": position_opening.take_profit_upper_bound,
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
        }); // end post_request

        // Verify that the position is safe to open.
        // This function will generate and return an error,
        // if it is unsafe to open a position.
        Self::verify_position(&position_opening).await?;

        // Send a request to open a position on perpetual swaps.
        let response: serde_json::Value = client
            .post(
                "/v5/order/create",
                Some(post_request),
                [BybitOption::HttpAuth(BybitHttpAuth::V3AndAbove)],
            )
            .await
            .map_err(|error| AppError::new(error.to_string()))?;

        // The number of partial take profits to set.
        // It is float, as it is necessary for further calculations.
        let number_take_profits = Decimal::from_str_exact("3.0").unwrap();

        // Calculate the quantity to close on each partial take profit.
        let partial_qty =
            Decimal::floor(&((qty / number_take_profits) / quantity_step)) * quantity_step;

        // Calculate the price step to close the orders.
        let price_step = (position_opening.take_profit_upper_bound
            - position_opening.take_profit_lower_bound)
            .abs()
            / number_take_profits;

        // A variable to keep track of the price in the loop.
        // It is necessary to take the minimum to calculate take profits
        // for short.
        let mut cur_price = position_opening.take_profit_lower_bound;

        // Setup a grid of smaller take profits on the way to the main goal.
        for _ in 0..number_take_profits.to_string().parse::<f64>().unwrap() as i32 {
            // Set a new conditional order with take profit.
            Self::place_partial_take_profit_order(
                &client,
                &position_opening.ticker,
                &position_opening.direction,
                cur_price.to_string(),
                partial_qty.to_string(),
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

        Self::validate_exchange_response(&response)
    } // end fn open_position_on_perpetual_swaps

    /// This function closes a part of a position on perpetual swaps
    /// in USDT.
    pub async fn close_part_of_position_on_perpetual_swaps(
        position_closure: Arc<PositionClosure>,
        user: User,
    ) -> Result<(), AppError> {
        // Get a clone of the user to be able to adjust a stop loss.
        let user_clone = user.clone();

        // Get user credentials.
        let api_key = user.exchange_api_key.unwrap();
        let api_secret = user.exchange_api_secret.unwrap();

        // Get an exchange client.
        let client = Self::get_client(api_key, api_secret);

        // Remove all conditional orders related to the position and
        // get the information on whether or not it is necessary to
        // remove an actual position or it was a conditional order
        // that has not got executed yet.
        //
        // ATTENTION: The instruction above are valid ONLY if it
        // is necessary to close the entire position. For partial
        // position closure the instructions above are not valid!
        if position_closure.percentage_to_close == dec!(100.0) {
            // Close the related conditional orders and check
            // whether or not it is necessary to continue closing
            // the actual orders.
            if Self::remove_all_conditional_orders_on_perpetual_swaps(
                &client,
                &position_closure.ticker,
                &position_closure.direction,
            )
            .await?
            {
                // It is unnecessary to close any positions.
                // In this case all the required operations are done.
                return Ok(());
            } // end if
        } // end if

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

        // If it is necessary to close only a part of the position,
        // put the stop loss to non-loss.
        if position_closure.percentage_to_close != dec!(100.0) {
            // Get a price step of the opened position.
            let price_step =
                Self::get_price_step_on_perpetual_swaps(&client, &position_closure.ticker).await?;

            // A new stop loss.
            let stop_loss: Decimal;

            // Determine which direction to adjust the stop loss in.
            // Calculate a new stop loss.
            if side == "Buy" {
                // Set a stop loss in a slight profit for long position.
                stop_loss = (Decimal::ceil(&(&cur_position.avg_entry_price / &price_step))
                    + dec!(1.0))
                    * price_step;
            } else {
                // Set a stop loss in a slight profit for short position.
                stop_loss = (Decimal::floor(&(&cur_position.avg_entry_price / &price_step))
                    - dec!(1.0))
                    * price_step;
            } // end if

            // Put a stop loss to non-loss.
            Self::adjust_stop_loss_on_perpetual_swaps(
                Arc::new(StopLossAdjustment {
                    ticker: position_closure.ticker.to_string(),
                    direction: position_closure.direction.to_string(),
                    new_stop: stop_loss,
                }),
                user_clone,
            )
            .await?;
        } // end if

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
        let quantity_step =
            Self::get_quantity_step_on_perpetual_swaps(&client, cur_position.ticker).await?;

        // Calculate the quantity to fix.
        let qty = cur_position.qty * (position_closure.percentage_to_close / dec!(100.0));

        // Adjust the quantity so to fit the quantity step restrictions.
        let qty = Decimal::floor(&(qty / quantity_step)) * quantity_step;

        // Send a request to adjust the position size on perpetual swaps.
        let response: serde_json::Value = client
            .post(
                "/v5/order/create",
                Some(json!({
                    "category": "linear",
                    "symbol": position_closure.ticker,
                    "side": closure_side,
                    "orderType": "Market",
                    "qty": qty.to_string(),
                    "reduceOnly": true,
                    "positionIdx": position_idx
                })),
                [BybitOption::HttpAuth(BybitHttpAuth::V3AndAbove)],
            )
            .await
            .map_err(|error| AppError::new(error.to_string()))?;

        Self::validate_exchange_response(&response)
    } // end fn close_part_of_position_on_perpetual_swaps

    /// This function adjusts the stop loss of a particular position on
    /// perpetual swaps in USDT.
    pub async fn adjust_stop_loss_on_perpetual_swaps(
        stop_loss_adjustment: Arc<StopLossAdjustment>,
        user: User,
    ) -> Result<(), AppError> {
        // Get user credentials.
        let api_key = user.exchange_api_key.unwrap();
        let api_secret = user.exchange_api_secret.unwrap();

        // Get an exchange client.
        let client = Self::get_client(api_key, api_secret);

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

        // If the new stop loss is greater than the previous one,
        // then reject the operation. Stop loss can only reduce
        // the loss.
        if ((position_side == "Buy")
            && (stop_loss_adjustment.new_stop < cur_position.stop_loss_price))
            || ((position_side == "Sell")
                && (stop_loss_adjustment.new_stop > cur_position.stop_loss_price))
        {
            // Reject the operation.
            return Err(AppError::new("Stop loss cannot increase the position risk"));
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

        // Send a request to adjust a stop loss of the order on perpetual swaps.
        let response: serde_json::Value = client
            .post(
                "/v5/order/amend",
                Some(json!({
                    "category": "linear",
                    "symbol": stop_loss_adjustment.ticker,
                    "orderId": cur_order.order_id,
                    "triggerPrice": stop_loss_adjustment.new_stop.to_string()
                })),
                [BybitOption::HttpAuth(BybitHttpAuth::V3AndAbove)],
            )
            .await
            .map_err(|error| AppError::new(error.to_string()))?;

        Self::validate_exchange_response(&response)
    } // end fn adjust_stop_loss_on_perpetual_swaps

    /// This function sets a leverage for a particular position on
    /// perpetual swaps in USDT.
    pub async fn set_leverage(
        client: &Client,
        ticker: &String,
        leverage: Decimal,
    ) -> Result<(), AppError> {
        // Send a request to adjust a leverage on perpetual swaps.
        let response: serde_json::Value = client
            .post(
                "/v5/position/set-leverage",
                Some(json!({
                    "category": "linear",
                    "symbol": ticker,
                    "buyLeverage": leverage.to_string(),
                    "sellLeverage": leverage.to_string(),
                })),
                [BybitOption::HttpAuth(BybitHttpAuth::V3AndAbove)],
            )
            .await
            .map_err(|error| AppError::new(error.to_string()))?;

        Self::validate_exchange_response(&response)
    } // end fn set_leverage

    /// This function sets a position in a hedge-mode.
    pub async fn set_hedge_mode_on_perpetual_swaps(
        client: &Client,
        ticker: &String,
    ) -> Result<(), AppError> {
        // Send a request to set a position in a hedge-mode on perpetual swaps.
        let response: serde_json::Value = client
            .post(
                "/v5/position/switch-mode",
                Some(json!({
                    "category": "linear",
                    "symbol": ticker,
                    "mode": 3
                })),
                [BybitOption::HttpAuth(BybitHttpAuth::V3AndAbove)],
            )
            .await
            .map_err(|error| AppError::new(error.to_string()))?;

        Self::validate_exchange_response(&response)
    } // end fn set_hedge_mode_on_perpetual_swaps

    /// This function cancels an opened order on perpetual swaps in USDT.
    pub async fn cancel_order_on_perpetual_swaps(
        client: &Client,
        order: &OpenedOrder,
    ) -> Result<(), AppError> {
        // Send a request to cancel an order on perpetual swaps.
        let response: serde_json::Value = client
            .post(
                "/v5/order/cancel",
                Some(json!({
                    "category": "linear",
                    "symbol": order.ticker,
                    "orderId": order.order_id
                })),
                [BybitOption::HttpAuth(BybitHttpAuth::V3AndAbove)],
            )
            .await
            .map_err(|error| AppError::new(error.to_string()))?;

        Self::validate_exchange_response(&response)
    } // end fn cancel_order_on_perpetual_swaps

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
        client: &Client,
        ticker: &String,
        position_side: &String,
        trigger_price: String,
        qty: String,
    ) -> Result<(), AppError> {
        // The opposite direction to close the position.
        let side: &str;

        // Trigger direction.
        // The parameter is required when setting up a conditional
        // order.
        let trigger_direction: i32;

        // What position to close (hedge-short/hedge-long).
        let positions_idx: i32;

        // Determine the values for the order.
        if position_side == "Buy" {
            side = "Sell";
            trigger_direction = 1;
            positions_idx = 1;
        } else {
            side = "Buy";
            trigger_direction = 2;
            positions_idx = 2;
        } // end if

        // Send a request to adjust a leverage on perpetual swaps.
        let response: serde_json::Value = client
            .post(
                "/v5/order/create",
                Some(json!({
                    "category": "linear",
                    "symbol": ticker,
                    "side": side,
                    "orderType": "Market",
                    "qty": qty.to_string(),
                    "triggerDirection": trigger_direction,
                    "triggerPrice": trigger_price.to_string(),
                    "positionIdx": positions_idx,
                    "reduceOnly": true,
                    "tpslMode": "Partial",
                    "closeOnTrigger": true
                })),
                [BybitOption::HttpAuth(BybitHttpAuth::V3AndAbove)],
            )
            .await
            .map_err(|error| AppError::new(error.to_string()))?;

        Self::validate_exchange_response(&response)
    } // end fn place_partial_take_profit_order

    /// This function removes all conditional orders of a particular
    /// contract on perpetual swaps in USDT.
    ///
    /// The function returns True if a conditional order with the position to close
    /// was found and removed (necessary not to close the position itself if
    /// the order did not get executed). Otherwise the function returns false.
    pub async fn remove_all_conditional_orders_on_perpetual_swaps(
        client: &Client,
        ticker: &String,
        side: &String,
    ) -> Result<bool, AppError> {
        // Get a copy of the ticker.
        // It is required for further part of the code.
        let ticker = ticker.to_string();

        // Get all the opened orders.
        let opened_orders = Self::get_all_opened_orders(client).await?;

        // This variable stores the result of the function.
        let mut order_removed = false;

        // This parameter is required to work with orders in a hedge-mode.
        let position_idx: i32;

        // Convert the side from (short/long) into (Sell/Buy).
        // Determine the opposite side at the same time.
        // Set the proper position idx.
        let side = if side.to_lowercase() == "long" {
            position_idx = 1;
            "Buy"
        } else {
            position_idx = 2;
            "Sell"
        }; // end if

        // Traverse all the opened orders and find the orders that have to
        // be removed.
        for opened_order in opened_orders {
            // Check if the order ticker and position idx match
            // the orders that are searched for.
            if opened_order.ticker == ticker && opened_order.position_idx == position_idx {
                // It is one of the positions searched for.

                // Check if this order was meant to open a position.
                if opened_order.side == side {
                    // This is the order that was meant to open a new position.
                    order_removed = true;
                } // end if

                // Cancel the order.
                // Cancel only stop orders. (Stop losses and Take profits (partial and full)
                // will be closed automatically, when the trade is closed)
                if opened_order.stop_order_type == "Stop" {
                    Self::cancel_order_on_perpetual_swaps(client, &opened_order).await?;
                } // end if
            } // end if
        } // end for

        Ok(order_removed)
    } // end fn remove_all_conditional_orders_on_perpetual_swaps

    /// This function validates the response code from an exchange.
    /// If the response code is different from 0, then it generates
    /// an error and returns it.
    pub fn validate_exchange_response(response: &serde_json::Value) -> Result<(), AppError> {
        // This is a set of codes that can be considered as success.
        // NOTE: To check the codes, visit Bybit API documentation.
        let success_codes = vec![
            0,      // OK
            110025, // Position mode has not been modified.
            110026, // Cross/isolated margin mode has not been modified.
            110027, // Margin has not been modified.
            110028, // You have existing open orders, so the position mode cannot be switched.
            110043, // Set leverage has not been modified.
        ];

        // Get the error code as a number.
        let response_code = response["result"]["retCode"]
            .to_string()
            .replace("\"", "")
            .parse::<i32>()
            .unwrap();

        // Check if the return code is equal to one of the success codes.
        if success_codes.contains(&response_code) {
            // An operation was successful.
            return Ok(());
        } // end if

        // Otherwise there is an error in the response.
        // Return an error with the error message received from the
        // exchange.
        return Err(AppError::new(
            &response["result"]["retMsg"].to_string().replace("\"", ""),
        ));
    } // end fn validate_exchange_response
} // end impl ExchangeService
