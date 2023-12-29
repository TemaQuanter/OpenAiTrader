// This file contains the main loop for incoming
// messages tracking.

use std::sync::Arc;

use chatgpt::{converse::Conversation, types::ChatMessage};
use chrono::Utc;
use diesel::PgConnection;
use teloxide::{
    adaptors::Throttle,
    types::{Message, User},
    Bot,
};
use tokio::sync::MutexGuard;

use crate::{
    db_objects::channel_history::ChannelHistory,
    handlers::{
        bot::bot_service::BotService,
        exchange::{
            exchange_service::ExchangeService,
            exchange_service_perpetual::ExchangeServicePerpetual,
            exchange_service_spot::ExchangeServiceSpot,
        },
    },
    utils::{app_error::AppError, database_service::DatabaseService},
};

use super::{bot_service::BotState, dto_perpetual, dto_spot};

/// This struct contains all the functionality to
/// track and to handle incoming messages for a Telegram client.
pub struct MessageService;

/// Implement all the functionality to track and to handle
/// incoming messages for a Telegram client.
impl MessageService {
    /// This function handles a new message from a particular chat.
    pub async fn handle_message(
        message: Message,
        bot_state: BotState,
        bot: &Throttle<Bot>,
    ) -> Result<(), AppError> {
        // Acquire the conversations.
        let chatgpt_conversation_perpetual = bot_state.chatgpt_conversation_perpetual.lock().await;
        let chatgpt_conversation_spot = bot_state.chatgpt_conversation_spot.lock().await;

        // Actual ChatGPT conversation.
        let mut chatgpt_conversation: MutexGuard<'_, Conversation>;

        // Make a copy of message date.
        let message_date = message.date;

        // Check if the time of the post is still appropriate for handling.
        if Utc::now() - message_date > chrono::Duration::minutes(3) {
            // Inform that the relevance of the message is low.
            BotService::inform_client(
                &bot,
                bot_state.log_channel_id,
                "The message is no longer relevant, skipping",
            )
            .await?;

            // Skip handling of the message, as its relevance is
            // very low.
            return Ok(());
        } // end if

        // Try to assemble a trade legend.
        let trade_legend = Self::assemble_trade_legend(message, bot_state.clone()).await;

        // Check if an error occurred while assembling the legend.
        if trade_legend.is_err() {
            // Inform the users in log channel about the error.
            BotService::inform_client(
                bot,
                bot_state.log_channel_id,
                "Failed to assemble a trade legend",
            )
            .await?;

            return Ok(());
        } // end if

        // Safely unwrap the legend.
        let (trade_legend, category) = trade_legend.unwrap();

        // Check if it is most likely to be a signal or not.
        if trade_legend.is_none() {
            // It is less likely to be a signal.
            return Ok(());
        } // end if

        // Determine what ChatGPT model should be used.
        if category == "perpetual" {
            chatgpt_conversation = chatgpt_conversation_perpetual;
        } else if category == "spot" {
            chatgpt_conversation = chatgpt_conversation_spot;
        } else {
            // There is no any other category.
            return Err(AppError::new(
                "Wrong category. Must be either \"perpetual\" or \"spot\"",
            ));
        } // end if

        // Safely unwrap the legend.
        let trade_legend = trade_legend.unwrap();

        // Feed the message to ChatGPT.
        let response_chatgpt = chatgpt_conversation
            .send_message(&trade_legend)
            .await
            .map_err(|error| AppError::new(error.to_string()))?;

        let chatgpt_message = response_chatgpt.message().content.clone();

        // Check whether it is a spot or perpetual signal, then
        // try to parse the the response from ChatGPT and
        // react to it appropriately.
        if category == "perpetual" {
            // A signal on perpetual swaps.
            Self::react_to_signal_perpetual(chatgpt_message, bot, bot_state.clone()).await?;
        } else if category == "spot" {
            // A signal on spot.
            Self::react_to_signal_spot(chatgpt_message, bot, bot_state.clone()).await?;
        } else {
            // There is no any other category.
            return Err(AppError::new(
                "Wrong category. Must be either \"perpetual\" or \"spot\"",
            ));
        } // end if

        // Remove a message from the history in order not to waste tokens.
        chatgpt_conversation.history.truncate(1);

        // Append ChatGPT reply on a system message.
        chatgpt_conversation.history.push(ChatMessage {
            role: chatgpt::types::Role::Assistant,
            content: "OK".to_string(),
        });

        // Inform a client about the result.

        BotService::inform_client(
            &bot,
            bot_state.log_channel_id,
            format!("<b>A potential {} signal!</b>💰", category),
        )
        .await?;

        BotService::inform_client(&bot, bot_state.log_channel_id, trade_legend).await?;

        BotService::inform_client(
            &bot,
            bot_state.log_channel_id,
            "💬<b>ChatGPT</b> said...👇👇👇",
        )
        .await?;

        // Respond to the user via Telegram bot about a trading decision.
        BotService::inform_client(
            &bot,
            bot_state.log_channel_id,
            response_chatgpt.message().content.to_string(),
        )
        .await?;

        Ok(())
    } // end fn handle_message

    /// This function tries to assemble a trade legend when it is possible.
    /// If it is just a message, it skips it.
    async fn assemble_trade_legend(
        message: Message,
        bot_state: BotState,
    ) -> Result<(Option<String>, String), AppError> {
        // Initialize an array that will be storing a trade legend.
        let mut trade_legend: Vec<String> = Vec::new();

        // Check if the message contains text.
        if message.text().is_none() {
            // There is no need to process the message,
            // if it does not contain any text data.
            return Ok((None, "".to_string()));
        } // end if

        // Check if the message is less than 4 symbols, then do not react.
        if message.text().unwrap().chars().count() < 4 {
            // Do not react to the message.
            return Ok((None, "".to_string()));
        } // end if

        // Get a connection to the database.
        let conn = &mut DatabaseService::get_pool_connection(&bot_state.pg_connection_pool)
            .map_err(|error| AppError::new(error.to_string()))?;

        // Get the current message from the database.
        let mut cur_msg = Self::get_message_from_db_by_id(message.id.0, conn)
            .map_err(|error| AppError::new(error.to_string()))?;

        // Start adding messages to the legend till the top message.
        loop {
            // Push a message to the legend.
            trade_legend.push(cur_msg.message_text.unwrap_or(String::from("")));

            // Check if the message is a reply to another message.
            if let Some(previous_msg_id) = cur_msg.response_message_id {
                // Jump to a new message and repeat the loop.
                // Fetch a message this message replies to.
                cur_msg = Self::get_message_from_db_by_id(previous_msg_id, conn)
                    .map_err(|error| AppError::new(error.to_string()))?;
            } else {
                // There are no more messages.
                // The entire legend is assembled.
                break;
            } // end if
        } // end loop

        // println!("Legend:\n{:#?}", &trade_legend);

        // Get the first message of the trade legend.
        let first_message = trade_legend.last().unwrap();

        // Check if the last inserted message is a signal on perpetual swaps.
        if Self::is_perpetual_signal(first_message) {
            // Convert the legend to string.
            let trade_legend = Self::convert_legend_to_string(trade_legend);
            // It is most likely to be a signal.
            return Ok((Some(trade_legend), "perpetual".to_string()));
        } // end if

        // Check if the last inserted message is a signal on spot.
        if Self::is_spot_signal(first_message) {
            // Convert the legend to string.
            let trade_legend = Self::convert_legend_to_string(trade_legend);
            // It is most likely to be a signal.
            return Ok((Some(trade_legend), "spot".to_string()));
        } // end if

        Ok((None, "".to_string()))
    } // end fn assemble_trade_snippet

    /// This function checks if a message is a signal on spot or not.
    /// (The role of the function is to eliminate the waste of tokens
    /// on ChatGPT)
    fn is_spot_signal(message: &String) -> bool {
        if message.contains("спот") || message.contains("spot") {
            return true;
        } // end if

        false
    } // end fn is_spot_signal

    /// This function checks if a message is a signal on perpetual swaps or not
    /// (The role of the function is to eliminate the waste of tokens
    /// on ChatGPT)
    fn is_perpetual_signal(message: &String) -> bool {
        // Declare variables that the final decision will be based on.
        let mut has_bybit = false;
        let mut has_usdt = false;

        // Put all letters in a lowercase.
        let message = message.to_lowercase();

        // Check if the function contains the word "bybit".
        if message.contains("bybit") {
            has_bybit = true;
        } // end if

        // Check if the function contains the word "usdt".
        if message.contains("usdt") {
            has_usdt = true;
        } // end if

        // Return true only if all conditions are true.
        has_usdt && has_bybit
    } // end fn is_signal

    /// This function converts a vector of messages into
    /// one main string.
    fn convert_legend_to_string(mut legend: Vec<String>) -> String {
        // Initialize the result.
        let mut result = String::new();

        // Counter is necessary to enumerate the messages.
        let mut counter: i32 = 1;

        // Traverse the legend in the reverse order and
        // assemble a string.
        while let Some(message) = legend.pop() {
            result.push_str(&format!("Msg #{counter}\n"));
            result.push_str(&message);
            result.push_str("\n\n");

            // Increase counter.
            counter += 1;
        } // end while

        result
    } // end fn convert_legend_to_string

    /// This function retrieves a message from the database by message ID.
    /// if an error occurs, the function logs the error and returns it.
    fn get_message_from_db_by_id(
        msg_id: i32,
        conn: &mut PgConnection,
    ) -> Result<ChannelHistory, diesel::result::Error> {
        match ChannelHistory::get_from_db_by_id(msg_id, conn) {
            Ok(cur_msg) => return Ok(cur_msg),
            Err(error) => {
                log::error!("{}", error);

                return Err(error);
            } // end Err
        }; // end match
    } // end fn get_message_from_db_by_id

    /// This function gets read of old messages from the database.
    /// This function is considered to be run as a separate asynchronous task.
    pub async fn remove_old_messages(
        bot_state: BotState,
        age: chrono::Duration,
        wait_time_secs: u64,
        bot: Throttle<Bot>,
    ) {
        // This variable indicates whether or not the operation was successful.
        let mut is_success = false;

        // Endless loop that consistently removes legacy messages.
        loop {
            // Check whether or not the operation was successful.
            if is_success {
                // Reset the flag to default value.
                is_success = false;

                // Wait till the next day to perform the task.
                tokio::time::sleep(tokio::time::Duration::from_secs(wait_time_secs)).await;
            } else {
                // Wait for some time to repeat the task.
                tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;
            } // end if

            // Get a connection to the database.
            let conn =
                &mut match DatabaseService::get_pool_connection(&bot_state.pg_connection_pool) {
                    Ok(conn) => conn,
                    Err(error) => {
                        log::error!("{}", error);

                        // Inform the users in the log channel about the error.
                        match BotService::inform_client(
                            &bot,
                            bot_state.log_channel_id,
                            "Failed to connect to the database (to remove old posts)",
                        )
                        .await
                        {
                            Ok(_) => (),
                            Err(error) => {
                                log::error!("{}", error);
                            } // end Err
                        }; // end match

                        continue;
                    } // end Err
                }; // end match

            // Calculate the timestamp of an "old" post.
            let old_timestamp = Utc::now() - age;

            // Try to remove the "old" messages from the database.
            match ChannelHistory::remove_old_messages(old_timestamp, conn) {
                Ok(_) => (),
                Err(error) => {
                    log::error!("{}", error);

                    // Inform the users in the log channel about the error.
                    match BotService::inform_client(
                        &bot,
                        bot_state.log_channel_id,
                        "Failed to remove old posts",
                    )
                    .await
                    {
                        Ok(_) => (),
                        Err(error) => {
                            log::error!("{}", error);
                        } // end Err
                    }; // end match
                }
            } // end match

            // Indicate that the operation was completed successfully.
            is_success = true;

            // Send the message to the log channel that the operation was successful.
            // Inform the users in the log channel about the error.
            match BotService::inform_client(
                &bot,
                bot_state.log_channel_id,
                "Old posts were removed from the database",
            )
            .await
            {
                Ok(_) => (),
                Err(error) => {
                    log::error!("{}", error);
                } // end Err
            }; // end match
        } // end loop
    } // end fn remove_old_messages

    /// This function parses the response from ChatGPT and reacts to
    /// it opening, closing or increasing positions on perpetual swaps in USDT.
    pub async fn react_to_signal_perpetual(
        signal: String,
        bot: &Throttle<Bot>,
        bot_state: BotState,
    ) -> Result<(), AppError> {
        // Try to parse the the response from ChatGPT and
        // react to it appropriately.
        if let Ok(position_opening) =
            serde_json::from_str::<dto_perpetual::PositionOpening>(&signal)
        {
            // This is a position opening signal.
            // Open a new position.
            Self::send_exchange_order_execution_feedback(
                ExchangeService::execute_exchange_order_for_all_users(
                    Arc::new(position_opening),
                    ExchangeServicePerpetual::open_position,
                    bot_state.clone(),
                )
                .await,
                &bot,
                &bot_state,
            )
            .await;
        } else if let Ok(position_increase) =
            serde_json::from_str::<dto_perpetual::PositionIncrease>(&signal)
        {
            // This is a position increase, pyramiding.
            // Increase the position size.
            Self::send_exchange_order_execution_feedback(
                ExchangeService::execute_exchange_order_for_all_users(
                    Arc::new(position_increase),
                    ExchangeServicePerpetual::increase_position,
                    bot_state.clone(),
                )
                .await,
                &bot,
                &bot_state,
            )
            .await;
        } else if let Ok(order_removal) =
            serde_json::from_str::<dto_perpetual::ConditionalOrderRemoval>(&signal)
        {
            // This is a removal of an opened order.
            // Remove the opened order on perpetual swaps.
            Self::send_exchange_order_execution_feedback(
                ExchangeService::execute_exchange_order_for_all_users(
                    Arc::new(order_removal),
                    ExchangeServicePerpetual::cancel_order,
                    bot_state.clone(),
                )
                .await,
                &bot,
                &bot_state,
            )
            .await;
        } else if let Ok(position_closure) =
            serde_json::from_str::<dto_perpetual::PositionClosure>(&signal)
        {
            // This is a position closure signal.
            // Close a part of the position.
            Self::send_exchange_order_execution_feedback(
                ExchangeService::execute_exchange_order_for_all_users(
                    Arc::new(position_closure),
                    ExchangeServicePerpetual::close_part_of_position,
                    bot_state.clone(),
                )
                .await,
                &bot,
                &bot_state,
            )
            .await;
        } else if let Ok(stop_loss_adjustment) =
            serde_json::from_str::<dto_perpetual::StopLossAdjustment>(&signal)
        {
            // This is a stop loss adjustment signal.
            // Adjust stop loss of a position.
            Self::send_exchange_order_execution_feedback(
                ExchangeService::execute_exchange_order_for_all_users(
                    Arc::new(stop_loss_adjustment),
                    ExchangeServicePerpetual::adjust_stop_loss,
                    bot_state.clone(),
                )
                .await,
                &bot,
                &bot_state,
            )
            .await;
        } else {
            // There is nothing to do.
            // Skip the signal.
        } // end if

        Ok(())
    } // end react_to_signal_perpetual

    /// This function parses the response from ChatGPT and reacts to
    /// it opening, closing or increasing positions on spot.
    pub async fn react_to_signal_spot(
        signal: String,
        bot: &Throttle<Bot>,
        bot_state: BotState,
    ) -> Result<(), AppError> {
        // Try to parse the the response from ChatGPT and
        // react to it appropriately.
        if let Ok(position_opening) = serde_json::from_str::<dto_spot::PositionOpening>(&signal) {
            // This is a position opening signal.
            // Open a new position.
            Self::send_exchange_order_execution_feedback(
                ExchangeService::execute_exchange_order_for_all_users(
                    Arc::new(position_opening),
                    ExchangeServiceSpot::open_position,
                    bot_state.clone(),
                )
                .await,
                &bot,
                &bot_state,
            )
            .await;
        } else if let Ok(position_increase) =
            serde_json::from_str::<dto_spot::PositionIncrease>(&signal)
        {
            // This is a position increase, pyramiding.
            // Increase the position size.
            Self::send_exchange_order_execution_feedback(
                ExchangeService::execute_exchange_order_for_all_users(
                    Arc::new(position_increase),
                    ExchangeServiceSpot::increase_position,
                    bot_state.clone(),
                )
                .await,
                &bot,
                &bot_state,
            )
            .await;
        } else if let Ok(order_removal) =
            serde_json::from_str::<dto_spot::ConditionalOrderRemoval>(&signal)
        {
            // This is a removal of an opened order.
            // Remove the opened order on spot.
            Self::send_exchange_order_execution_feedback(
                ExchangeService::execute_exchange_order_for_all_users(
                    Arc::new(order_removal),
                    ExchangeServiceSpot::cancel_order,
                    bot_state.clone(),
                )
                .await,
                &bot,
                &bot_state,
            )
            .await;
        } else if let Ok(position_closure) =
            serde_json::from_str::<dto_spot::PositionClosure>(&signal)
        {
            // This is a position closure signal.
            // Close a part of the position.
            Self::send_exchange_order_execution_feedback(
                ExchangeService::execute_exchange_order_for_all_users(
                    Arc::new(position_closure),
                    ExchangeServiceSpot::close_part_of_position,
                    bot_state.clone(),
                )
                .await,
                &bot,
                &bot_state,
            )
            .await;
        } else if let Ok(stop_loss_adjustment) =
            serde_json::from_str::<dto_spot::StopLossAdjustment>(&signal)
        {
            // This is a stop loss adjustment signal.
            // Adjust stop loss of a position.
            Self::send_exchange_order_execution_feedback(
                ExchangeService::execute_exchange_order_for_all_users(
                    Arc::new(stop_loss_adjustment),
                    ExchangeServiceSpot::adjust_stop_loss,
                    bot_state.clone(),
                )
                .await,
                &bot,
                &bot_state,
            )
            .await;
        } else {
            // There is nothing to do.
            // Skip the signal.
        } // end if

        Ok(())
    } // end react_to_signal_spot

    /// This function sends information about successes and failures after executing
    /// an order on the exchange for multiple users.
    async fn send_exchange_order_execution_feedback(
        res: Result<(i32, i32, i32, i32), AppError>,
        bot: &Throttle<Bot>,
        bot_state: &BotState,
    ) {
        // Unwrap the result and send an appropriate feedback.
        match res {
            Ok((successful_threads, failed_threads, successes, failures)) => {
                log::info!("The attempt to execute orders for all users was successful!");

                // Inform the users in log channel about the success/failure statistics.
                match BotService::inform_client(
                    bot,
                    bot_state.log_channel_id,
                    format!(
                        "<b>🟢 The order was executed successfully!</b>\n\n\
                <b>📊 Statistics:</b>\n\n\
                <b>😃 Successful threads:</b> {}\n\
                <b>😔 Failed threads:</b> {}\n\n\
                <b>🟢 Successes:</b> {}\n\
                <b>🔴 Failures:</b> {}",
                        successful_threads, failed_threads, successes, failures
                    ),
                )
                .await
                {
                    Ok(_) => log::info!("The users are informed successfully"),
                    Err(error) => log::error!("{}", error),
                }; // end match
            } // end Ok
            Err(error) => {
                log::error!("{}", error);

                // Inform the users in log channel about a failure.
                // This failure occurred for all users.
                match BotService::inform_client(
                    bot,
                    bot_state.log_channel_id,
                    format!(
                        "<b>🔴 Failed to execute the order for all users!</b>\n\n\
                <b>Error:</b> {}",
                        error
                    ),
                )
                .await
                {
                    Ok(_) => log::info!("The users are informed successfully"),
                    Err(error) => log::error!("{}", error),
                }; // end match
            } // end Err
        } // end match
    } // end fn send_exchange_order_execution_feedback

    /// Get the user, who sent a message.
    pub fn get_user(msg: &Message) -> Option<&User> {
        match msg.from() {
            Some(user) => Some(user),
            None => None,
        } // end match
    } // end fn get_user_id
} // end impl ClientHandler
