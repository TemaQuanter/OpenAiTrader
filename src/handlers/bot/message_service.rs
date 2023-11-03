// This file contains the main loop for incoming
// messages tracking.

use chatgpt::prelude::Conversation;
use teloxide::{
    types::{Message, User, UserId},
    Bot,
};

use crate::{handlers::bot::bot_service::BotService, utils::types_and_constants::OneResult};

use super::dto::{PositionClosure, PositionOpening, StopLossAdjustment};

/// This struct contains all the functionality to
/// track and to handle incoming messages for a Telegram client.
pub struct MessageService;

/// Implement all the functionality to track and to handle
/// incoming messages for a Telegram client.
impl MessageService {
    /// This function handles a new message from a particular chat.
    // pub async fn handle_message(
    //     message: Message,
    //     mut chatgpt_conversation: Conversation,
    //     bot: &Bot,
    //     trusted_user_id: i64,
    //     tg_client: &Client,
    //     exchange_client: &crypto_botters::Client,
    // ) -> Result<Conversation> {
    //     log::info!(
    //         "Got a new message!\n\
    //     ChatID: {}\n\
    //     ChatName: {}",
    //         message.chat().id(),
    //         message.chat().name()
    //     ); // end log::info

    //     // // Load the application settings.
    //     // let application_settings = AppSettingsService::load_settings()?;

    //     // // Check if the bot is allowed to trade.
    //     // if !application_settings.is_active {
    //     //     // The bot is not allowed to trade.
    //     //     return Ok(chatgpt_conversation);
    //     // } // end if

    //     // Try to assemble a trade legend.
    //     let trade_legend = Self::assemble_trade_legend(message, tg_client).await;

    //     // Check if it is most likely to be a signal or not.
    //     if trade_legend.is_none() {
    //         // It is less likely to be a signal.
    //         return Ok(chatgpt_conversation);
    //     } // end if

    //     // Unwrap trade_legend.
    //     let trade_legend = trade_legend.unwrap();

    //     // Feed the message to ChatGPT.
    //     let response_chatgpt = chatgpt_conversation.send_message(&trade_legend).await?;

    //     let chatgpt_message = response_chatgpt.message().content.clone();

    //     // Try to parse the the response from ChatGPT and
    //     // react to it appropriately.
    //     // Self::react_to_signal(chatgpt_message, bot, trusted_user_id, exchange_client).await?;

    //     // Remove a message from the history in order not to waste tokens.
    //     chatgpt_conversation.history.truncate(1);

    //     BotService::inform_client(bot, trusted_user_id, "<b>A potential signal!</b>💰").await?;

    //     BotService::inform_client(bot, trusted_user_id, trade_legend).await?;

    //     BotService::inform_client(bot, trusted_user_id, "💬<b>ChatGPT</b> said...👇👇👇").await?;

    //     // Respond to the user via Telegram bot about a trading decision.
    //     BotService::inform_client(
    //         bot,
    //         trusted_user_id,
    //         response_chatgpt.message().content.to_string(),
    //     )
    //     .await?;

    //     Ok(chatgpt_conversation)
    // } // end fn handle_message

    // /// This function tries to assemble a trade legend when it is possible.
    // /// If it is just a message, it skips it.
    // async fn assemble_trade_legend(mut message: Message, tg_client: &Client) -> Option<String> {
    //     // Initialize an array that will be storing a trade legend.
    //     let mut trade_legend: Vec<String> = Vec::new();

    //     // Check if the message is less than 4 symbols, then do not react.
    //     if message.text().chars().count() < 4 {
    //         // Do not react to the message.
    //         return None;
    //     } // end if

    //     // Start adding messages to the legend till the top message.
    //     loop {
    //         // Push a message to the legend.
    //         trade_legend.push(message.text().to_string());

    //         // Check if the message is a reply to another message.
    //         if let Some(msg_id) = message.reply_to_message_id() {
    //             // Jump to a new message and repeat the loop.
    //             // Fetch a message this message replies to.
    //             message = tg_client
    //                 .get_messages_by_id(message.chat(), &[msg_id])
    //                 .await
    //                 .unwrap()
    //                 .pop()
    //                 .unwrap()
    //                 .unwrap();
    //         } else {
    //             // There are no more messages.
    //             // The entire legend is assembled.
    //             break;
    //         } // end if
    //     } // end loop

    //     // Check if the last inserted message is a signal.
    //     if Self::is_signal(trade_legend.last().unwrap()) {
    //         // Convert the legend to string.
    //         let trade_legend = Self::convert_legend_to_string(trade_legend);
    //         // It is most likely to be a signal.
    //         return Some(trade_legend);
    //     } // end if

    //     None
    // } // end fn assemble_trade_snippet

    /// This function checks if a message is a signal or not
    /// (The role of the function is to eliminate the waste of tokens
    /// on ChatGPT)
    fn is_signal(message: &String) -> bool {
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

    // This function parses the response from ChatGPT and reacts to
    // it opening, closing or increasing positions on perpetual swaps in USDT.
    // pub async fn react_to_signal(
    //     signal: String,
    //     bot: &Bot,
    //     trusted_user_id: i64,
    //     exchange_client: &crypto_botters::Client,
    // ) -> Result<()> {
    //     // Try to parse the the response from ChatGPT and
    //     // react to it appropriately.
    //     if let Ok(position_opening) = serde_json::from_str::<PositionOpening>(&signal) {
    //         // This is a position opening signal.
    //         // Open a new position.
    //         match ExchangeService::open_position_on_perpetual_swaps(
    //             &position_opening,
    //             exchange_client,
    //         )
    //         .await
    //         {
    //             Ok(_) => {
    //                 log::info!(
    //                     "A position was opened successfully:\n{:#?}",
    //                     position_opening
    //                 ); // end log

    //                 // Inform a user in Telegram.
    //                 BotService::inform_client(
    //                     bot,
    //                     trusted_user_id,
    //                     "🟢<b>A position was opened successfully!</b>🟢",
    //                 )
    //                 .await?;
    //             }
    //             Err(error) => {
    //                 log::error!(
    //                     "Failed to open a position:\n{:#?}\nBecause of: {}",
    //                     position_opening,
    //                     error
    //                 ); // end log

    //                 // Inform a user in Telegram.
    //                 BotService::inform_client(
    //                     bot,
    //                     trusted_user_id,
    //                     format!(
    //                         "🔴<b>Failed to open a new position!</b>🔴\n\
    //                     <b>Error:</b> {}",
    //                         error
    //                     ),
    //                 )
    //                 .await?;
    //             } // end Err
    //         }; // end match
    //     } else if let Ok(position_closure) = serde_json::from_str::<PositionClosure>(&signal) {
    //         // This is a position closure signal.
    //         // Close a part of the position.
    //         match ExchangeService::close_part_of_position_on_perpetual_swaps(
    //             &position_closure,
    //             exchange_client,
    //         )
    //         .await
    //         {
    //             Ok(_) => {
    //                 log::info!(
    //                     "A position was closed successfully:\n{:#?}",
    //                     position_closure
    //                 ); // end log

    //                 // Inform a user in Telegram.
    //                 BotService::inform_client(
    //                     bot,
    //                     trusted_user_id,
    //                     "🟢<b>A position was closed successfully!</b>🟢",
    //                 )
    //                 .await?;
    //             } // end Ok
    //             Err(error) => {
    //                 log::error!(
    //                     "Failed to close a part of position:\n{:#?}\nBecause of: {}",
    //                     position_closure,
    //                     error
    //                 ); // end log

    //                 // Inform a user in Telegram.
    //                 BotService::inform_client(
    //                     bot,
    //                     trusted_user_id,
    //                     format!(
    //                         "🔴<b>Failed to close a part of position!</b>🔴\n\
    //                     <b>Error:</b> {}",
    //                         error
    //                     ),
    //                 )
    //                 .await?;
    //             } // end Err
    //         }; // end match
    //     } else if let Ok(stop_loss_adjustment) = serde_json::from_str::<StopLossAdjustment>(&signal)
    //     {
    //         // This is a stop loss adjustment signal.
    //         // Adjust stop loss of a position.
    //         match ExchangeService::adjust_stop_loss_on_perpetual_swaps(
    //             &stop_loss_adjustment,
    //             exchange_client,
    //         )
    //         .await
    //         {
    //             Ok(_) => {
    //                 log::info!(
    //                     "A stop loss was adjusted successfully:\n{:#?}",
    //                     stop_loss_adjustment
    //                 ); // end log

    //                 // Inform a user in Telegram.
    //                 BotService::inform_client(
    //                     bot,
    //                     trusted_user_id,
    //                     "🟢<b>A stop loss was adjusted successfully!</b>🟢",
    //                 )
    //                 .await?;
    //             } // end Ok
    //             Err(error) => {
    //                 log::error!(
    //                     "Failed to adjust a stop loss:\n{:#?}\nBecause of: {}",
    //                     stop_loss_adjustment,
    //                     error
    //                 ); // end log

    //                 // Inform a user in Telegram.
    //                 BotService::inform_client(
    //                     bot,
    //                     trusted_user_id,
    //                     format!(
    //                         "🔴<b>Failed to adjust a stop loss!</b>🔴\n\
    //                     <b>Error:</b> {}",
    //                         error
    //                     ),
    //                 )
    //                 .await?;
    //             } // end Err
    //         }; // end match
    //     } else {
    //         // There is nothing to do.
    //         // Skip the signal.
    //     } // end if

    //     Ok(())
    // } // end react_to_signal

    /// Get the user, who sent a message.
    pub fn get_user(msg: &Message) -> Option<&User> {
        match msg.from() {
            Some(user) => Some(user),
            None => None,
        } // end match
    } // end fn get_user_id
} // end impl ClientHandler
