// This file contains basic and universal functionality
// to perform some general operations with Bybit account.

use std::{future::Future, sync::Arc};

use crypto_botters::{
    bybit::{BybitHttpAuth, BybitOption},
    Client,
};

use rust_decimal::Decimal;
use teloxide::{adaptors::Throttle, Bot};
use tokio::task::JoinHandle;

use crate::{
    db_objects::{status_model::StatusId, user_model::User},
    handlers::bot::bot_service::{BotService, BotState},
    utils::{
        app_error::AppError, database_service::DatabaseService,
        lazy_static::DELAY_BETWEEN_UNSUCCESSFUL_REQUESTS,
        types_and_constants::NUMBER_ATTEMPTS_SEND_REQUEST,
    },
};

/// This struct contains all the functionality to interact
/// with Bybit cryptocurrency exchange.
pub struct ExchangeService;

/// Implement all the functionality to interact with
/// Bybit cryptocurrency exchange on perpetual swaps.
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
    pub async fn is_eligible_to_trade(candidate: &User) -> bool {
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
        let _balance = match Self::get_uta_total_equity(&client).await {
            Ok(balance) => balance,
            Err(_error) => {
                // The candidate's account is unreachable.
                return false;
            }
        };

        // Check if their balance is over some minimum limit.
        // if balance < dec!(500) {
        //     return false;
        // } // end if

        // The candidate passed all checks and is eligible to trade.
        true
    } // end fn is_eligible_to_trade

    /// This function gets an array of users who are eligible to trade
    pub async fn get_users_eligible_to_trade(bot_state: BotState) -> Result<Vec<User>, AppError> {
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

    /// This function returns total equity of Unified Trading Account in USDT.
    pub async fn get_uta_total_equity(client: &Client) -> Result<Decimal, AppError> {
        // Get the information about UTA equity.
        let response: serde_json::Value = client
            .get(
                "/v5/account/wallet-balance",
                Some(&[("accountType", "UNIFIED"), ("coin", "USDT")]),
                [BybitOption::HttpAuth(BybitHttpAuth::V3AndAbove)],
            )
            .await
            .map_err(|error| AppError::new(error.to_string()))?;

        // Validate exchange response.
        Self::validate_exchange_response(&response)?;

        // Return the balance of the wallet in USDT.
        Ok(Decimal::from_str_exact(
            &response["result"]["list"][0]["totalEquity"]
                .to_string()
                .replace("\"", ""),
        )
        .unwrap())
    } // end fn get_uta_total_equity

    /// This function executes an exchange order on perpetual swaps for every single user
    /// asynchronously.
    ///
    /// Return values:
    /// Element 1 - the number of threads that executed successfully.
    /// Element 2 - the number of threads that failed during execution.
    /// Element 3 - the number of tasks that were completed successfully.
    /// Element 4 - the number of tasks that failed.
    pub async fn execute_exchange_order_for_all_users<T, Out>(
        order_description: String,
        exchange_order: Arc<T>,
        func: fn(Arc<T>, User) -> Out,
        bot_state: BotState,
        bot: &Throttle<Bot>,
    ) -> Result<(i32, i32, i32, i32), AppError>
    where
        T: Send + Sync + 'static,
        Out: Future<Output = Result<(), AppError>> + Send + 'static,
    {
        // Get an array of users who can trade.
        let eligible_users = ExchangeService::get_users_eligible_to_trade(bot_state).await?;

        // An array that stores all asynchronous tasks.
        let mut async_tasks: Vec<(User, JoinHandle<Result<(), AppError>>)> = Vec::new();

        // Spawn a task for each user to execute the orders asynchronously.
        for user in eligible_users {
            // Clone the reference to the position opening.
            let exchange_order_clone = Arc::clone(&exchange_order);

            // Get a clone of the user.
            let user_clone = user.clone();

            // Create a new task.
            let new_task = tokio::spawn(async move {
                // Try to execute an order on perpetual swaps.
                func(exchange_order_clone, user_clone).await
            });

            // Add the task to the array of running tasks.
            async_tasks.push((user, new_task));
        } // end for

        // Count failed and successful threads.
        let mut successful_threads = 0;
        let mut failed_threads = 0;

        // Count positive and negative results.
        let mut positive_results = 0;
        let mut negative_results = 0;

        // Array with users and the messages that they need to receive.
        let mut users_messages: Vec<(i64, String)> = Vec::new();

        // Wait for all tasks to complete.
        for tsk in async_tasks {
            // Check if the thread completed successfully.
            match tsk.1.await {
                // The thread completed successfully.
                Ok(tsk_result) => {
                    // Increment the successful threads counter.
                    successful_threads += 1;

                    // Check if the operation succeeded for a user.
                    if let Err(error) = tsk_result {
                        // The task finished with error.
                        negative_results += 1;

                        log::error!(
                            "[\"user_id\": {} \"username\": {}] : {}",
                            tsk.0.telegram_id,
                            tsk.0.telegram_username.unwrap_or("None".to_string()),
                            error
                        );

                        // Add a signal to inform the user about failure to execute the order.
                        users_messages.push((tsk.0.telegram_id,
                        format!(
                            "🔴 <b>Failed to execute the order on your account:</b>\n\n👉 {}\n\n⚠️ <b>Error:</b> {}",
                            order_description,
                            error
                        )));
                    } else {
                        // The result was OK.
                        positive_results += 1;

                        // Add a signal to inform the user about the successful order execution.
                        users_messages.push((tsk.0.telegram_id,
                            format!(
                                "🟢 <b>The order is successfully executed on your account:</b>\n\n👉 {}",
                                order_description
                            ),
                        ));
                    } // end if
                } // end Ok
                Err(error) => {
                    // Increment failed threads counter.
                    failed_threads += 1;

                    // Log the error.
                    log::error!(
                        "[\"user_id\": {} \"username\": {}] : {}",
                        tsk.0.telegram_id,
                        tsk.0.telegram_username.unwrap_or("None".to_string()),
                        error
                    );

                    // Add a signal to inform the user about failure to execute the order.
                    users_messages.push((tsk.0.telegram_id,
                        format!(
                            "🔴 <b>Failed to execute the order on your account:</b>\n\n👉 {}\n\n⚠️ <b>Error:</b> {}",
                            order_description,
                            error
                        ),
                    ));
                } // end Err
            } // end match
        } // end for

        // Acquire the bot before creating a separate thread.
        let bot = bot.to_owned();

        // Run a separate task to broadcast messages to the users.
        let _ = tokio::spawn(async move {
            BotService::broadcast_messages(bot, users_messages).await;
        });

        return Ok((
            successful_threads,
            failed_threads,
            positive_results,
            negative_results,
        ));
    } // end fn execute_exchange_order_for_all_users

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
        let response_code = response["retCode"]
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

        // Try to get an error message.
        let error_message = if let serde_json::Value::String(error_msg) = &response["retMsg"] {
            error_msg
        } else {
            "Null"
        }; // end if

        // Log the error.
        log::error!(
            "{}",
            error_message.to_string() + format!(" Response code: {}", response_code).as_str()
        ); // end log

        // Return an error with the error message received from the
        // exchange.
        Err(AppError::new(error_message))
    } // end fn validate_exchange_response

    /// This function returns the number of remaining days before API key expires.
    pub async fn get_api_deadline_days(client: &Client) -> Result<i32, AppError> {
        // Get the information about UTA equity.
        let response: serde_json::Value = client
            .get(
                "/v5/user/query-api",
                Option::<&[()]>::None,
                [BybitOption::HttpAuth(BybitHttpAuth::V3AndAbove)],
            )
            .await
            .map_err(|error| AppError::new(error.to_string()))?;

        // Validate exchange response.
        Self::validate_exchange_response(&response)?;

        // Return the balance of the wallet in USDT.
        Ok(response["result"]["deadlineDay"]
            .to_string()
            .parse::<i32>()
            .unwrap())
    } // end fn get_api_deadline_days

    /// This function allows sending a GET request to the exchange multiple times if it fails.
    pub async fn send_robust_get_request(
        client: &Client,
        url: &str,
        get_request: &[(&str, &str)],
    ) -> Result<serde_json::Value, AppError> {
        // Initialize the response.
        let mut response: Option<serde_json::Value> = None;

        // Try to send the request several times if it is not sent successfully.
        for i in 0..NUMBER_ATTEMPTS_SEND_REQUEST {
            // Try to send a request for the first time.
            let res: Result<serde_json::Value, _> = client
                .get(
                    url,
                    Some(get_request),
                    [BybitOption::HttpAuth(BybitHttpAuth::V3AndAbove)],
                )
                .await;

            // If the request sending failed, wait for some time and try again.
            if res.is_err() {
                // Check if it was the last possible request.
                if i == NUMBER_ATTEMPTS_SEND_REQUEST - 1 {
                    // All attempts are made, the operation failed.
                    return Err(AppError::new(res.err().unwrap().to_string()));
                } // end if

                // Otherwise, there are still available attempts, repeat the request again.
                tokio::time::sleep(*DELAY_BETWEEN_UNSUCCESSFUL_REQUESTS).await;
            } else {
                // Unwrap the successful response.
                response = Some(res.unwrap());

                // The response is received.
                break;
            } // end if
        } // end for

        // Check if the request was received.
        if response.is_none() {
            // The response was not received.
            return Err(AppError::new("Failed to get a response"));
        } // end if

        // Safely unwrap the response.
        let response = response.unwrap();

        Ok(response)
    } // end fn send_robust_get_request

    /// This function allows sending a POST request to the exchange multiple times if it fails.
    pub async fn send_robust_post_request(
        client: &Client,
        url: &str,
        post_request: &serde_json::Value,
    ) -> Result<serde_json::Value, AppError> {
        // Initialize the response.
        let mut response: Option<serde_json::Value> = None;

        // Try to send the request several times if it is not sent successfully.
        for i in 0..NUMBER_ATTEMPTS_SEND_REQUEST {
            // Try to send a request for the first time.
            let res: Result<serde_json::Value, _> = client
                .post(
                    url,
                    Some(post_request),
                    [BybitOption::HttpAuth(BybitHttpAuth::V3AndAbove)],
                )
                .await;

            // If the request sending failed, wait for some time and try again.
            if res.is_err() {
                // Check if it was the last possible request.
                if i == NUMBER_ATTEMPTS_SEND_REQUEST - 1 {
                    // All attempts are made, the operation failed.
                    return Err(AppError::new(res.err().unwrap().to_string()));
                } // end if

                // Otherwise, there are still available attempts, repeat the request again.
                tokio::time::sleep(*DELAY_BETWEEN_UNSUCCESSFUL_REQUESTS).await;
            } else {
                // Unwrap the successful response.
                response = Some(res.unwrap());

                // The response is received.
                break;
            } // end if
        } // end for

        // Check if the request was received.
        if response.is_none() {
            // The response was not received.
            return Err(AppError::new("Failed to get a response"));
        } // end if

        // Safely unwrap the response.
        let response = response.unwrap();

        Ok(response)
    } // end fn send_robust_post_request
} // end impl ExchangeService
