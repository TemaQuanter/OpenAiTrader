// This file contains all the functionality necessary to interact
// with a user via Telegram bot.

use std::{collections::HashMap, sync::Arc};

use async_recursion::async_recursion;
use chatgpt::prelude::Conversation;
use chrono::Utc;
use diesel::{
    r2d2::{ConnectionManager, Pool, PooledConnection},
    PgConnection,
};
use rust_decimal::Decimal;

use crate::utils::{
    lazy_static::SUBSCRIPTION_BOT_URL,
    types_and_constants::{MAX_BROADCAST_ATTEMPTS, PATH_TO_BOT_MANUAL},
};
use serde::{Deserialize, Serialize};
use teloxide::{
    adaptors::Throttle,
    dispatching::{HandlerExt, UpdateFilterExt},
    dptree,
    payloads::SendMessageSetters,
    prelude::Dispatcher,
    requests::{Requester, ResponseResult},
    types::{
        CallbackQuery, ChatId, KeyboardButton, KeyboardMarkup, KeyboardRemove, Message, ParseMode,
        ReplyMarkup, Update,
    },
    utils::command::BotCommands,
    Bot,
};
use tokio::{sync::Mutex, task::JoinHandle};

use crate::{
    db_objects::{
        channel_history::ChannelHistory, role_model::RoleId, status_model::StatusId,
        user_model::User,
    },
    handlers::exchange::exchange_service::ExchangeService,
    utils::{database_service::DatabaseService, types_and_constants::ERROR_MESSAGE},
};

use super::message_service::MessageService;

/// This structure is necessary to make initial setup for
/// user account in the trading bot.
#[derive(Clone, Default, PartialEq)]
pub enum DialogState {
    #[default]
    Start,
    ReceiveDeposit,
    ReceiveRisk {
        deposit: Decimal,
    },
    ReceiveApiExchangeKey {
        deposit: Decimal,
        risk: Decimal,
    },
    ReceiveApiExchangeSecret {
        deposit: Decimal,
        risk: Decimal,
        exchange_api_key: String,
    },
    ReceiveBotAllowedToTrade {
        deposit: Decimal,
        risk: Decimal,
        exchange_api_key: String,
        exchange_api_secret: String,
    },
    // Superuser dialog states.
    Broadcast,
} // end DialogState

// Messages to request the user data in the dialog.
// They are necessary to skip the steps in the setup dialog.
const MESSAGE_REQUEST_RISK: &str =
    "Wonderful! Now, send me the risk per trade you would like to use in percentage\n\n\
    <b>Example:</b> 1.85\n\nIt means that your R (risk per trade) will be 1.85% from your deposit";
const MESSAGE_REQUEST_API_KEY: &str = "Awesome! Now, send me the API key from your \
    account/subaccount where you would like me to trade\n\n\
    <b>Example:</b> Qw123ghe...77wia1\n\nP.S It must be API key only! \
    API secret will be requested later.";
const MESSAGE_REQUEST_API_SECRET: &str = "Amazing! Now, send me the secret from the API \
    key you have just provided me with\n\n\
    <b>Example:</b> a2k343k2k3...6b8k23\n\n\
    P.S This secret should have been shown to you when you created your API key";
const MESSAGE_REQUEST_BOT_ALLOWED_TO_TRADE: &str =
    "Excellent! Now, tell me, whether or not you allow me to \
start managing your assets right now!\n";

/// This structure contains a common state for bot message
/// handler.
#[derive(Clone)]
pub struct BotState {
    pub user_dialog_states: Arc<Mutex<HashMap<i64, DialogState>>>,
    /// Database connection pool.
    pub pg_connection_pool: Arc<Pool<ConnectionManager<PgConnection>>>,
    /// ChatGPT conversation for perpetual swaps trading.
    pub chatgpt_conversation_perpetual: Arc<Mutex<Conversation>>,
    /// ChatGPT conversation for stop trading.
    pub chatgpt_conversation_spot: Arc<Mutex<Conversation>>,
    /// This is a trusted channel ID that the bot
    /// gets trading signals from.
    pub trusted_channel_id: i64,
    /// This is a channel where the bot sends its logs.
    pub log_channel_id: i64,
} // end struct BotConfig

/// These are all commands that the bot can handle for general user.
#[derive(BotCommands, Clone, PartialEq)]
#[command(
    rename_rule = "lowercase",
    description = "These commands are supported:"
)]
enum UserCommands {
    #[command(description = "start interaction with the bot.")]
    Start,
    #[command(description = "cancel dialog.")]
    Cancel,
    #[command(description = "skip a step in a dialog.")]
    Skip,
    #[command(description = "display this text.")]
    Help,
    #[command(description = "set a new deposit (for risk calculation)")]
    SetDeposit(Decimal),
    #[command(description = "set a new risk per trade in percentage")]
    SetRisk(Decimal),
    #[command(description = "set the api key from your exchange account")]
    SetExchangeApiKey(String),
    #[command(description = "set the api secret from your exchange account")]
    SetExchangeApiSecret(String),
    #[command(description = "set whether or not the bot is allowed to trade")]
    SetIsActive(bool),
    #[command(description = "get the application settings")]
    GetSettings,
    #[command(
        description = "check whether or not the bot is able to access your account on the exchange"
    )]
    CheckExchangeApiConnection,
} // end enum UserCommands

// These are all commands that the bot can handle for admin user.
#[derive(BotCommands, Clone)]
#[command(
    rename_rule = "lowercase",
    description = "These commands are supported for admins:"
)]
enum AdminCommands {
    #[command(description = "get list of pending users")]
    GetPendingUsers,
    #[command(description = "get banned users")]
    GetBannedUsers,
    #[command(description = "get approved users")]
    GetApprovedUsers,
    #[command(description = "get rejected users")]
    GetRejectedUsers,
    #[command(description = "get expired users")]
    GetExpiredUsers,
    #[command(description = "get admins")]
    GetAdmins,
    #[command(description = "make admin (assign an admin role to the user)")]
    MakeAdmin(i64),
    #[command(description = "make user (lower the user from admin to user")]
    MakeUser(i64),
    #[command(description = "approve a pending user")]
    Approve(i64),
    #[command(description = "reject a pending user")]
    Reject(i64),
    #[command(description = "ban a user")]
    Ban(i64),
    #[command(description = "make user subscription expired")]
    Expire(i64),
} // end enum AdminCommand

// These are all commands that the bot can handle for superuser.
#[derive(BotCommands, Clone)]
#[command(
    rename_rule = "lowercase",
    description = "These commands are supported for superusers:"
)]
enum SuperuserCommands {
    #[command(description = "make superuser")]
    MakeSuperuser(i64),
    #[command(description = "get superusers")]
    GetSuperusers,
    #[command(description = "broadcast a message to all users")]
    Broadcast,
} // end enum SuperuserCommands

/// Data structure to parse command example.
#[derive(Serialize, Deserialize, Debug)]
struct CommandExample {
    command: String,
    response: String,
} // end struct CommandExample

/// Data structure to parse command description.
#[derive(Serialize, Deserialize, Debug)]
struct Command {
    command: String,
    description: String,
    example: CommandExample,
} // end struct Command

/// Data structure to parse JSON user manual to the bot.
#[derive(Serialize, Deserialize, Debug)]
struct Commands {
    user: Vec<Command>,
    admin: Vec<Command>,
    superuser: Vec<Command>,
} // end struct Commands

/// This structure contains all the functionality
/// necessary to drive a Telegram bot.
pub struct BotService;

/// Implement all the functionality necessary to drive a Telegram
/// bot.
impl BotService {
    /// This function prompts a user for information.
    pub async fn inform_client(
        bot: &Throttle<Bot>,
        chat_id: i64,
        message: impl AsRef<str>,
    ) -> ResponseResult<()> {
        // Shadow chat ID for convenience.
        let chat_id = ChatId(chat_id);

        // Prompt a client for something.
        bot.send_message(chat_id, message.as_ref())
            .parse_mode(ParseMode::Html)
            .await?;

        Ok(())
    } // end fn inform_client

    /// This function sets up a bot listener that handles every incoming message,
    /// whether it is a command, common message or anything other type.
    pub async fn set_listener(bot: Throttle<Bot>, bot_state: BotState) -> JoinHandle<()> {
        // A handler that determines the behavior of the bot in regards to
        // chat messages.
        let chat_handler = Update::filter_message()
            // Handle commands sent by a general user.
            .branch(
                dptree::entry()
                    .filter_command::<UserCommands>()
                    .endpoint(Self::handle_user_command),
            )
            // Handle commands sent by an admin.
            .branch(
                dptree::entry()
                    .filter_command::<AdminCommands>()
                    .endpoint(Self::handle_admin_command),
            )
            // Handle commands sent by a superuser.
            .branch(
                dptree::entry()
                    .filter_command::<SuperuserCommands>()
                    .endpoint(Self::handle_superuser_command),
            )
            .branch(dptree::entry().endpoint(Self::handle_dialog)); // end Update::filter_message()

        // A handler that handles callback queries.
        let callback_query_handler = Update::filter_callback_query()
            .branch(dptree::entry().endpoint(Self::handle_callback_queries));

        // A handler that determines the behavior of the bot in regards to
        // channel posts.
        let channel_handler = Update::filter_channel_post().branch(
            // Allow posts only from a trusted channel.
            dptree::filter(move |msg: Message| msg.chat.id == ChatId(bot_state.trusted_channel_id))
                .endpoint(Self::read_channel_post),
        ); // end Update::filter_message

        // Main handler that joins all handlers into one.
        let handler = dptree::entry()
            .branch(chat_handler)
            .branch(callback_query_handler)
            .branch(channel_handler);

        let mut bot_service_job = Dispatcher::builder(bot, handler)
            // Specify initial dependencies that all handlers will receive; they can be
            .dependencies(dptree::deps![bot_state])
            // If no handler succeeded to handle an update, this closure will be called.
            // .default_handler(|upd| async move {
            //     log::warn!("Unhandled update: {:?}", upd);
            // })
            // If the dispatcher fails for some reason, execute this handler.
            // .error_handler(|| async move {})
            .enable_ctrlc_handler()
            .build();

        // Remember the reference to the async job.
        let listener = tokio::spawn(async move {
            bot_service_job.dispatch().await;
        });

        listener
    } // end fn set_listener

    /// This function handles callback queries in the bot in various dialogs.
    async fn handle_callback_queries(
        _bot_state: BotState,
        _bot: Throttle<Bot>,
        _q: CallbackQuery,
    ) -> ResponseResult<()> {
        todo!();
    } // end fn handle_callback_queries

    /// This function greets the user after they finished the dialog.
    #[async_recursion]
    async fn congratulate_and_guide_user(
        bot: Throttle<Bot>,
        msg: Message,
        bot_state: BotState,
    ) -> ResponseResult<()> {
        // Welcome the user in the service.
        bot.send_message(
            msg.chat.id,
            "<b>Congratulations! You are all set! 😍</b>\n\n\
                    Here are all your settings👇👇👇",
        )
        .reply_markup(ReplyMarkup::KeyboardRemove(KeyboardRemove::new()))
        .parse_mode(ParseMode::Html)
        .await?;

        // Display the settings to the user.
        Self::handle_user_command(
            bot_state.clone(),
            bot.clone(),
            msg.clone(),
            UserCommands::GetSettings,
        )
        .await?;

        // Explain the user how they can update settings.
        bot.send_message(
            msg.chat.id,
            "If you need to adjust some of the settings, you have <b>2 options</b>\n\n\
                    1) Use /help command to adjust settings via available commands\n\n\
                    2) User /start command to use the dialog you have just had with me again\n\n\
                    <b>Glad you decided to join me 😊</b>",
        )
        .parse_mode(ParseMode::Html)
        .await?;

        return Ok(());
    } // end fn congratulate_and_guide_user

    /// This function handles initial dialog that helps a user to set up everything
    /// required for automated trading.
    async fn handle_dialog(
        bot_state: BotState,
        bot: Throttle<Bot>,
        msg: Message,
    ) -> ResponseResult<()> {
        // Get a user from the message.
        let user = MessageService::get_user(&msg);

        // Check that the user was extracted successfully.
        if user.is_none() {
            // The user could not be extracted.
            return Ok(());
        } // end if

        // Safely unwrap the user.
        let user = user.unwrap();

        // Lock the mutex with user state.
        let user_dialog_states = bot_state.user_dialog_states.lock().await;

        // Get user state in the dialog.
        let user_state = user_dialog_states.get(&(user.id.0 as i64));

        // Make sure that the user has started the dialog.
        if user_state.is_none() {
            // The user has not started the dialog.
            return Ok(());
        } // end if

        // Safely unwrap the dialog state.
        let user_state = user_state.unwrap().clone();

        // Drop the mutex, to be able to call other functions, using bot state.
        drop(user_dialog_states);

        // Handle dialog message.
        match user_state {
            // Handle the default state of the dialog.
            DialogState::Start => {
                // If the user dialog state is Start, then do nothing
                // (This is handled by /start command).
                return Ok(());
            } // end DialogState::Start
            // Get the deposit from the user.
            DialogState::ReceiveDeposit => {
                // Get the deposit from the user.
                let amount_raw = msg.text().unwrap_or_default();

                // Try to convert the string to decimal.
                if let Ok(amount) = amount_raw.parse::<Decimal>() {
                    // The parsing was successful, call a function to set a deposit.
                    Self::handle_user_command(
                        bot_state.clone(),
                        bot.clone(),
                        msg.clone(),
                        UserCommands::SetDeposit(amount),
                    )
                    .await?;

                    // Change the state of the dialog.
                    bot_state.user_dialog_states.lock().await.insert(
                        user.id.0 as i64,
                        DialogState::ReceiveRisk { deposit: amount },
                    );

                    // Ask a user to send the risk.
                    bot.send_message(msg.chat.id, MESSAGE_REQUEST_RISK)
                        .parse_mode(ParseMode::Html)
                        .await?;
                } else {
                    // The user passed a non-decimal value.
                    // Inform them about it and allow them to repeat the attempt.
                    bot.send_message(
                        msg.chat.id,
                        "Oops, it seems to me it is not a number, please try again",
                    )
                    .parse_mode(ParseMode::Html)
                    .await?;
                } // end if
            } // end DialogState::ReceiveDeposit
            DialogState::ReceiveRisk { deposit } => {
                // Get the deposit from the user.
                let risk_raw = msg.text().unwrap_or_default();

                // Try to convert the string to decimal.
                if let Ok(risk) = risk_raw.parse::<Decimal>() {
                    // The parsing was successful, call a function to set a risk.
                    Self::handle_user_command(
                        bot_state.clone(),
                        bot.clone(),
                        msg.clone(),
                        UserCommands::SetRisk(risk),
                    )
                    .await?;

                    // Change the state of the dialog.
                    bot_state.user_dialog_states.lock().await.insert(
                        user.id.0 as i64,
                        DialogState::ReceiveApiExchangeKey { deposit, risk },
                    );

                    // Ask a user to send the exchange API key.
                    bot.send_message(msg.chat.id, MESSAGE_REQUEST_API_KEY)
                        .parse_mode(ParseMode::Html)
                        .await?;
                } else {
                    // The user passed a non-decimal value.
                    // Inform them about it and allow them to repeat the attempt.
                    bot.send_message(
                        msg.chat.id,
                        "Oops, it seems to me it is not a number, please try again",
                    )
                    .parse_mode(ParseMode::Html)
                    .await?;
                } // end if
            } // end DialogState::ReceiveRisk
            DialogState::ReceiveApiExchangeKey { deposit, risk } => {
                // Get the deposit from the user.
                let exchange_api_key = msg.text().unwrap_or_default().to_string();

                // Check the provided key.
                if !(exchange_api_key.chars().count() < 5 || exchange_api_key.contains(" ")) {
                    // The check was successful, call a function to set a deposit.
                    Self::handle_user_command(
                        bot_state.clone(),
                        bot.clone(),
                        msg.clone(),
                        UserCommands::SetExchangeApiKey(exchange_api_key.clone()),
                    )
                    .await?;

                    // Change the state of the dialog.
                    bot_state.user_dialog_states.lock().await.insert(
                        user.id.0 as i64,
                        DialogState::ReceiveApiExchangeSecret {
                            deposit,
                            risk,
                            exchange_api_key,
                        },
                    );

                    // Ask a user to send the exchange API secret.
                    bot.send_message(msg.chat.id, MESSAGE_REQUEST_API_SECRET)
                        .parse_mode(ParseMode::Html)
                        .await?;
                } else {
                    // The user passed a wrong value.
                    // Inform them about it and allow them to repeat the attempt.
                    bot.send_message(
                        msg.chat.id,
                        "Oops, it seems to me it is not a valid key, please try again",
                    )
                    .parse_mode(ParseMode::Html)
                    .await?;
                } // end if
            } // end DialogState::ReceiveExchangeApiKey
            DialogState::ReceiveApiExchangeSecret {
                deposit,
                risk,
                exchange_api_key,
            } => {
                // Get the deposit from the user.
                let exchange_api_secret = msg.text().unwrap_or_default().to_string();

                // Check the provided key.
                if !(exchange_api_secret.chars().count() < 5 || exchange_api_secret.contains(" ")) {
                    // The check was successful, call a function to set a deposit.
                    Self::handle_user_command(
                        bot_state.clone(),
                        bot.clone(),
                        msg.clone(),
                        UserCommands::SetExchangeApiSecret(exchange_api_secret.clone()),
                    )
                    .await?;

                    // Inform the user that there is an attempt to access their account.
                    bot.send_message(
                        msg.chat.id,
                        "Please, wait a little bit.\n\
                            I'm trying to access your account...",
                    )
                    .await?;

                    // Check the connection to the exchange via available credentials.
                    let client = ExchangeService::get_client(
                        exchange_api_key.clone(),
                        exchange_api_secret.clone(),
                    );

                    // Request total USDT equity of user's account.
                    match ExchangeService::get_uta_total_equity(&client).await {
                        Ok(total_usdt_equity) => {
                            // A request was successful.
                            // Inform a user about it.
                            bot.send_message(
                                msg.chat.id,
                                format!(
                                    "<b>I'm able to access your account!🥳🥳🥳</b>\n\n\
                            You have <b>${}</b> for trading :)",
                                    total_usdt_equity.floor()
                                ),
                            )
                            .parse_mode(ParseMode::Html)
                            .await?;
                        } // end Ok
                        Err(error) => {
                            log::error!("{}", error);

                            // Inform a user that is was impossible to connect
                            // to their account.
                            bot.send_message(
                                msg.chat.id,
                                "Unfortunately, I did not manage to connect to your account, \
                            please, try again to provide both API key and API secret.\n\n\
                            Right now, please send me API key again",
                            )
                            .await?;

                            // Set the dialog state back to receiving API key.
                            bot_state.user_dialog_states.lock().await.insert(
                                user.id.0 as i64,
                                DialogState::ReceiveApiExchangeKey { deposit, risk },
                            );

                            return Ok(());
                        } // end Err
                    } // end match

                    // Change the state of the dialog.
                    bot_state.user_dialog_states.lock().await.insert(
                        user.id.0 as i64,
                        DialogState::ReceiveBotAllowedToTrade {
                            deposit,
                            risk,
                            exchange_api_key,
                            exchange_api_secret,
                        },
                    );

                    // Ask a user to send the exchange API secret.
                    bot.send_message(msg.chat.id, MESSAGE_REQUEST_BOT_ALLOWED_TO_TRADE)
                        .reply_markup(KeyboardMarkup::new(vec![
                            vec![KeyboardButton::new("Yes, sure!")],
                            vec![KeyboardButton::new("No, not right now")],
                        ]))
                        .await?;
                } else {
                    // The user passed a wrong value.
                    // Inform them about it and allow them to repeat the attempt.
                    bot.send_message(
                        msg.chat.id,
                        "Oops, it seems to me it is not a valid secret, please try again",
                    )
                    .parse_mode(ParseMode::Html)
                    .await?;
                } // end if
            } // end DialogState::ReceiveExchangeApiSecret
            DialogState::ReceiveBotAllowedToTrade {
                deposit: _,
                risk: _,
                exchange_api_key: _,
                exchange_api_secret: _,
            } => {
                // Get the deposit from the user.
                let allowed_to_trade = msg.text().unwrap_or_default().to_string();

                // Try to shadow the permission.
                let allowed_to_trade = if allowed_to_trade == "Yes, sure!" {
                    true
                } else if allowed_to_trade == "No, not right now" {
                    false
                } else {
                    // Inform the user that it was incorrect and ask them to try again.
                    // The user passed a wrong value.
                    // Inform them about it and allow them to repeat the attempt.
                    bot.send_message(
                        msg.chat.id,
                        "Oops, it seems to me it is not a valid reply, please try again",
                    )
                    .parse_mode(ParseMode::Html)
                    .await?;

                    return Ok(());
                }; // end if

                // The check was successful, call a function to set a deposit.
                Self::handle_user_command(
                    bot_state.clone(),
                    bot.clone(),
                    msg.clone(),
                    UserCommands::SetIsActive(allowed_to_trade),
                )
                .await?;

                // This is the end of the dialog, exit it.
                bot_state
                    .user_dialog_states
                    .lock()
                    .await
                    .remove(&(user.id.0 as i64));

                // Congratulate and guide the user.
                Self::congratulate_and_guide_user(bot, msg, bot_state).await?;
            } // end DialogState::ReceiveBotAllowedToTrade
            DialogState::Broadcast => {
                // This is the message that it is required to broadcast.

                // The message that it is necessary to broadcast.
                let broadcast_message = msg.text().unwrap_or("");

                // Make sure that the length of the message is at least 1 symbol.
                if broadcast_message.chars().count() == 0 {
                    // It is impossible to broadcast such message.
                    bot.send_message(
                        msg.chat.id,
                        "It is impossible to broadcast an empty message, please send a new message",
                    )
                    .await?;

                    // Await a new message to broadcast.
                    return Ok(());
                } // end if

                // Get a connection to the database.
                let conn = &mut match DatabaseService::get_pool_connection(
                    &bot_state.pg_connection_pool,
                ) {
                    Ok(conn) => conn,
                    Err(error) => {
                        // An error occurred while trying to connect to the database.
                        log::error!("{}", error);

                        // Inform the user that there was an error while connecting to the database.
                        bot.send_message(msg.chat.id, ERROR_MESSAGE).await?;

                        // Terminate function execution.
                        return Ok(());
                    } // end Err
                }; // end match

                // Get all approved users.
                let approved_users = User::get_users_with_status(&StatusId::Approved, conn);

                // Make sure that the users were retrieved successfully.
                if approved_users.is_err() {
                    // An error occurred while retrieving the users from the database.
                    log::error!("{}", approved_users.err().unwrap().to_string());

                    // Inform the user about the error.
                    bot.send_message(msg.chat.id, ERROR_MESSAGE).await?;

                    // Terminate function execution.
                    return Ok(());
                } // end if

                // Remove the user from the state.
                bot_state
                    .user_dialog_states
                    .lock()
                    .await
                    .remove(&(user.id.0 as i64));

                // Inform the user that the message is being broadcasted.
                bot.send_message(msg.chat.id, "The message is being broadcasted...")
                    .await?;

                // Safely unwrap the retrieved users.
                let approved_users = approved_users.unwrap();

                // A mapping of users with messages to broadcast.
                let mut users_messages: Vec<(i64, String)> = Vec::new();

                // Create a mapping of users and messages to broadcast.
                for approved_user in approved_users {
                    // Check that the user is not the sender of the message.
                    if approved_user.telegram_id != user.id.0 as i64 {
                        users_messages
                            .push((approved_user.telegram_id, broadcast_message.to_string()));
                    } // end if
                } // end for

                // Broadcast the message to all users.
                Self::broadcast_messages(bot.clone(), users_messages).await;

                // Inform the user that the message was broadcasted.
                bot.send_message(msg.chat.id, "The message was broadcasted")
                    .await?;
            } // end DialogState::Broadcast
        } // end match

        return Ok(());
    } // end fn handle_dialog

    /// This function is a handler for bot commands sent by a general user or higher.
    async fn handle_user_command(
        bot_state: BotState,
        bot: Throttle<Bot>,
        msg: Message,
        cmd: UserCommands,
    ) -> ResponseResult<()> {
        // Get the user ID of the message sender.
        let user = MessageService::get_user(&msg);

        // Check if the user exists.
        if user.is_none() {
            // Do not continue answering on this request.
            return Ok(());
        } // end if

        // Safely unwrap the user.
        let user = user.unwrap();

        // Reset dialog, if started.
        // Reset the dialog only if it is not a command to skip the setup step.
        if cmd != UserCommands::Skip {
            bot_state
                .user_dialog_states
                .lock()
                .await
                .remove(&(user.id.0 as i64));
        } // end if

        // Run all the necessary checks regarding user status and
        // get a pooled database connection.
        let (registered_user, mut conn) =
            match Self::run_user_checks_and_get_db_conn(&bot, &msg, bot_state.clone()).await? {
                Ok((registered_user, conn)) => (registered_user, conn),
                Err(_error) => return Ok(()),
            }; // end match

        // Shadow connection as mutable reference for convenience.
        let conn = &mut conn;

        // Handle a specific command.
        match cmd {
            UserCommands::Start => {
                // Check if the user is already registered.
                if registered_user.is_some() {
                    // Safely unwrap the registered user.
                    let mut registered_user = registered_user.unwrap();

                    // Check whether or not the user telegram username was changed.
                    if user.username != registered_user.telegram_username {
                        // Update the username.
                        registered_user.telegram_username = user.username.clone();

                        // Check if the user removed their username, then
                        // set it to NULL.
                        if registered_user.telegram_username.is_none() {
                            // Save the changes to the database.
                            match User::set_username_to_null(&registered_user, conn) {
                                Ok(registered_user) => registered_user,
                                Err(error) => {
                                    // Inform the user that an error occurred.
                                    bot.send_message(msg.chat.id, ERROR_MESSAGE)
                                        .parse_mode(ParseMode::Html)
                                        .await?;

                                    log::error!("{}", error);

                                    // Exit the command execution.
                                    return Ok(());
                                } // end Err
                            }; // end match
                        } else {
                            // Save the changes to the database.
                            match User::update(&registered_user, conn) {
                                Ok(registered_user) => registered_user,
                                Err(error) => {
                                    // Inform the user that an error occurred.
                                    bot.send_message(msg.chat.id, ERROR_MESSAGE)
                                        .parse_mode(ParseMode::Html)
                                        .await?;

                                    log::error!("{}", error);

                                    // Exit the command execution.
                                    return Ok(());
                                } // end Err
                            }; // end match
                        } // end if
                    } // end if

                    // Great the user.
                    bot.send_message(
                        msg.chat.id,
                        "Welcome to <b>OPEN-AI-TRADER</b> bot!\n\n\
                    Now I'll help you to set up everything)\n\n\
                    Just follow the steps and if you want to skip a step in a setup, \
                    use /skip command",
                    )
                    .parse_mode(ParseMode::Html)
                    .await?;

                    // Switch the user dialog to the next step.
                    bot_state
                        .user_dialog_states
                        .lock()
                        .await
                        .insert(user.id.0 as i64, DialogState::ReceiveDeposit);

                    // Ask the user the first question.
                    // Query the deposit.
                    bot.send_message(
                        msg.chat.id,
                        "At first, what is your trading deposit in USDT?\n(Provide only number)\n\n\
                        <b>Example:</b> 1000\n\n\
                        It means that your trading deposit is 1000 USDT",
                    )
                    .parse_mode(ParseMode::Html)
                    .await?;

                    return Ok(());
                } // end if

                // If the user is not registered, then register them.

                // Register the user in the database.
                // Put the user in a pending mode.
                match User::insert_into_db(
                    &User {
                        telegram_id: (user.id.0 as i64),
                        telegram_username: user.username.clone(),
                        status_id: StatusId::Pending as i32,
                        exchange_api_key: None,
                        exchange_api_secret: None,
                        exchange_api_data_validated: false,
                        deposit: None,
                        risk: None,
                        bot_active: false,
                        role_id: 1,
                    },
                    conn,
                ) {
                    Ok(_) => {}
                    // An error occurred while inserting the user into the database.
                    Err(error) => {
                        bot.send_message(msg.chat.id, ERROR_MESSAGE)
                            .parse_mode(ParseMode::Html)
                            .await?;

                        log::error!("{}", error);

                        return Ok(());
                    } // end Err
                }; // end User::insert_into_db

                // Inform the user that the request was sent.
                bot.send_message(msg.chat.id, format!("Hi! I added you to a waiting list, please wait for admin to review your application.\n\n\
                If you want to become my client, then buy subscription in the following bot and it will be automatically activated here \n\n\
                {}", *SUBSCRIPTION_BOT_URL))
                    .parse_mode(ParseMode::Html)
                    .await?;
            } // end UserCommands::Start
            UserCommands::Cancel => {
                // The dialog is already canceled, as any command cancels it.
                // Inform the user that the dialog is canceled.
                bot.send_message(
                    msg.chat.id,
                    "Ok, the dialog is canceled.\n\nTo start it again, use /start command",
                )
                .reply_markup(ReplyMarkup::KeyboardRemove(KeyboardRemove::new()))
                .parse_mode(ParseMode::Html)
                .await?;
            } // end UserCommands::Cancel
            UserCommands::Skip => {
                // Skip the setup step in the dialog.

                // Lock the mutex on dialog states.
                let dialog_states = bot_state.user_dialog_states.lock().await;

                // Get the current dialog state for the user.
                let cur_dialog_state = dialog_states.get(&(user.id.0 as i64));

                // Check if the dialog state is set for the user.
                if cur_dialog_state.is_none() {
                    // Current dialog state is not set for the user.
                    return Ok(());
                } // end if

                // Inform the user that the step in the dialog is skipped.
                bot.send_message(msg.chat.id, "Ok, skipping this step...")
                    .parse_mode(ParseMode::Html)
                    .await?;

                // Safely unwrap current dialog state.
                let cur_dialog_state = cur_dialog_state.unwrap().to_owned();

                // Drop the mutex lock to avoid a deadlock.
                drop(dialog_states);

                // Get the next step in a dialog.
                let next_step = match cur_dialog_state {
                    DialogState::Start => DialogState::Start,
                    DialogState::ReceiveDeposit => {
                        // Prompt a user for risk.
                        bot.send_message(msg.chat.id, MESSAGE_REQUEST_RISK)
                            .parse_mode(ParseMode::Html)
                            .await?;
                        DialogState::ReceiveRisk {
                            deposit: Default::default(),
                        }
                    } // end DialogState::ReceiveDeposit
                    DialogState::ReceiveRisk { deposit } => {
                        // Prompt a user for API key.
                        bot.send_message(msg.chat.id, MESSAGE_REQUEST_API_KEY)
                            .parse_mode(ParseMode::Html)
                            .await?;
                        DialogState::ReceiveApiExchangeKey {
                            deposit,
                            risk: Default::default(),
                        }
                    } // end DialogState::ReceiveRisk
                    DialogState::ReceiveApiExchangeKey { deposit, risk } => {
                        // Prompt a user for API secret.
                        bot.send_message(msg.chat.id, MESSAGE_REQUEST_API_SECRET)
                            .parse_mode(ParseMode::Html)
                            .await?;
                        DialogState::ReceiveApiExchangeSecret {
                            deposit,
                            risk,
                            exchange_api_key: Default::default(),
                        }
                    } // end DialogState::ReceiveApiExchangeKey
                    DialogState::ReceiveApiExchangeSecret {
                        deposit,
                        risk,
                        exchange_api_key,
                    } => {
                        // Prompt a user for permission to manage asset.
                        bot.send_message(msg.chat.id, MESSAGE_REQUEST_BOT_ALLOWED_TO_TRADE)
                            .parse_mode(ParseMode::Html)
                            .reply_markup(KeyboardMarkup::new(vec![
                                vec![KeyboardButton::new("Yes, sure!")],
                                vec![KeyboardButton::new("No, not right now")],
                            ]))
                            .await?;
                        DialogState::ReceiveBotAllowedToTrade {
                            deposit,
                            risk,
                            exchange_api_key,
                            exchange_api_secret: Default::default(),
                        }
                    } // end DialogState::ReceiveApiExchangeSecret
                    DialogState::ReceiveBotAllowedToTrade {
                        deposit: _,
                        risk: _,
                        exchange_api_key: _,
                        exchange_api_secret: _,
                    } => {
                        // Congratulate the user with the end of account setup.
                        Self::congratulate_and_guide_user(bot, msg.clone(), bot_state.clone())
                            .await?;
                        DialogState::Start
                    } // end DialogState::ReceiveBotAllowedToTrade
                    DialogState::Broadcast => {
                        // This is a superuser state that allows to broadcast
                        // a message to all approve users.
                        DialogState::Start
                    } // end DialogState::Broadcast
                }; // end match

                // Lock the mutex on dialog states.
                let mut dialog_states = bot_state.user_dialog_states.lock().await;

                // If the next step is starting step, then remove the dialog state.
                // Otherwise, update the dialog state.
                if next_step == DialogState::Start {
                    dialog_states.remove(&(user.id.0 as i64));
                } else {
                    dialog_states.insert(user.id.0 as i64, next_step);
                } // end if
            } // end UserCommands::Skip
            UserCommands::Help => {
                // Verify the user.
                let registered_user = match Self::get_user_and_verify_role(
                    &bot,
                    &msg,
                    registered_user,
                    vec![RoleId::User, RoleId::Admin, RoleId::Superuser],
                )
                .await?
                {
                    Some(registered_user) => registered_user,
                    None => return Ok(()),
                }; // end match

                // Assemble command documentation according to user role.
                let manual = Self::assemble_user_manual(RoleId::from(registered_user.role_id));

                // If there was an error, inform a user about it.
                if manual.is_err() {
                    log::error!("{}", manual.unwrap_err());

                    // Send the help instructions to the user.
                    bot.send_message(msg.chat.id, ERROR_MESSAGE)
                        .parse_mode(ParseMode::Html)
                        .await?;

                    return Ok(());
                } // end if

                // Safely unwrap the manual.
                let manual = manual.unwrap();

                // Send the help instructions to the user, depending on their role.
                for cur_manual in manual {
                    bot.send_message(msg.chat.id, cur_manual)
                        .parse_mode(ParseMode::Html)
                        .await?;
                } // end for
            } // end UserCommands::Help
            UserCommands::SetDeposit(amount) => {
                // Verify the user.
                let mut registered_user = match Self::get_user_and_verify_role(
                    &bot,
                    &msg,
                    registered_user,
                    vec![RoleId::User, RoleId::Admin, RoleId::Superuser],
                )
                .await?
                {
                    Some(registered_user) => registered_user,
                    None => return Ok(()),
                }; // end match

                // Update the deposit to calculate risk from.
                registered_user.deposit = Some(amount);

                // Save the changes to the database.
                match User::update(&registered_user, conn) {
                    Ok(registered_user) => registered_user,
                    Err(error) => {
                        // Inform the user that an error occurred.
                        bot.send_message(msg.chat.id, ERROR_MESSAGE)
                            .parse_mode(ParseMode::Html)
                            .await?;

                        log::error!("{}", error);

                        // Exit the command execution.
                        return Ok(());
                    } // end Err
                }; // end match

                bot.send_message(
                    msg.chat.id,
                    format!(
                        "💰 Deposit is set to <b>${}</b>!",
                        registered_user.deposit.unwrap_or_default()
                    ),
                )
                .parse_mode(ParseMode::Html)
                .await?;
            } // end Command::SetDeposit
            UserCommands::SetRisk(risk) => {
                // Verify the user.
                let mut registered_user = match Self::get_user_and_verify_role(
                    &bot,
                    &msg,
                    registered_user,
                    vec![RoleId::User, RoleId::Admin, RoleId::Superuser],
                )
                .await?
                {
                    Some(registered_user) => registered_user,
                    None => return Ok(()),
                }; // end match

                // Update the risk per trade.
                registered_user.risk = Some(risk);

                // Save the changes to the database.
                match User::update(&registered_user, conn) {
                    Ok(registered_user) => registered_user,
                    Err(error) => {
                        // Inform the user that an error occurred.
                        bot.send_message(msg.chat.id, ERROR_MESSAGE)
                            .parse_mode(ParseMode::Html)
                            .await?;

                        log::error!("{}", error);

                        // Exit the command execution.
                        return Ok(());
                    } // end Err
                }; // end match

                bot.send_message(
                    msg.chat.id,
                    format!(
                        "❗️ Risk is set to <b>{}%</b>",
                        registered_user.risk.unwrap_or_default()
                    ),
                )
                .parse_mode(ParseMode::Html)
                .await?;
            } // end Command::SetRisk
            UserCommands::SetExchangeApiKey(exchange_api_key) => {
                // Verify the user.
                let mut registered_user = match Self::get_user_and_verify_role(
                    &bot,
                    &msg,
                    registered_user,
                    vec![RoleId::User, RoleId::Admin, RoleId::Superuser],
                )
                .await?
                {
                    Some(registered_user) => registered_user,
                    None => return Ok(()),
                }; // end match

                // Update the risk per trade.
                registered_user.exchange_api_key = Some(exchange_api_key);

                // Check whether or not the exchange API secret is set.
                if registered_user.exchange_api_secret.is_some() {
                    // Try to connect to the user's account on the exchange.

                    // Get user credentials.
                    let api_key = registered_user.exchange_api_key.clone().unwrap();
                    let api_secret = registered_user.exchange_api_secret.clone().unwrap();

                    // Get an exchange client.
                    let client = ExchangeService::get_client(api_key, api_secret);
                    let res = ExchangeService::get_uta_total_equity(&client).await;

                    // If the result is OK, then the account is reachable.
                    if res.is_ok() {
                        registered_user.exchange_api_data_validated = true;
                    } else {
                        // The account is not reachable.
                        registered_user.exchange_api_data_validated = false;
                    } // end if
                } // end if

                // Save the changes to the database.
                match User::update(&registered_user, conn) {
                    Ok(registered_user) => registered_user,
                    Err(error) => {
                        // Inform the user that an error occurred.
                        bot.send_message(msg.chat.id, ERROR_MESSAGE)
                            .parse_mode(ParseMode::Html)
                            .await?;

                        log::error!("{}", error);

                        // Exit the command execution.
                        return Ok(());
                    } // end Err
                }; // end match

                // Remove the message with sensitive information.
                bot.delete_message(msg.chat.id, msg.id).await?;

                bot.send_message(msg.chat.id, format!("🔑  Exchange API key is set!"))
                    .parse_mode(ParseMode::Html)
                    .await?;
            } // end Command::SetExchangeApiKey
            UserCommands::SetExchangeApiSecret(exchange_api_secret) => {
                // Verify the user.
                let mut registered_user = match Self::get_user_and_verify_role(
                    &bot,
                    &msg,
                    registered_user,
                    vec![RoleId::User, RoleId::Admin, RoleId::Superuser],
                )
                .await?
                {
                    Some(registered_user) => registered_user,
                    None => return Ok(()),
                }; // end match

                // Update the risk per trade.
                registered_user.exchange_api_secret = Some(exchange_api_secret);

                // Check whether or not the exchange API key is set.
                if registered_user.exchange_api_key.is_some() {
                    // Try to connect to the user's account on the exchange.

                    // Get user credentials.
                    let api_key = registered_user.exchange_api_key.clone().unwrap();
                    let api_secret = registered_user.exchange_api_secret.clone().unwrap();

                    // Get an exchange client.
                    let client = ExchangeService::get_client(api_key, api_secret);
                    let res = ExchangeService::get_uta_total_equity(&client).await;

                    // If the result is OK, then the account is reachable.
                    if res.is_ok() {
                        registered_user.exchange_api_data_validated = true;
                    } else {
                        // The account is not reachable.
                        registered_user.exchange_api_data_validated = false;
                    } // end if
                } // end if

                // Save the changes to the database.
                match User::update(&registered_user, conn) {
                    Ok(registered_user) => registered_user,
                    Err(error) => {
                        // Inform the user that an error occurred.
                        bot.send_message(msg.chat.id, ERROR_MESSAGE)
                            .parse_mode(ParseMode::Html)
                            .await?;

                        log::error!("{}", error);

                        // Exit the command execution.
                        return Ok(());
                    } // end Err
                }; // end match

                // Remove the message with sensitive information.
                bot.delete_message(msg.chat.id, msg.id).await?;

                bot.send_message(msg.chat.id, format!("㊙️  Exchange API secret is set!"))
                    .parse_mode(ParseMode::Html)
                    .await?;
            } // end Command::SetExchangeApiSecret
            UserCommands::SetIsActive(bot_active) => {
                // Verify the user.
                let mut registered_user = match Self::get_user_and_verify_role(
                    &bot,
                    &msg,
                    registered_user,
                    vec![RoleId::User, RoleId::Admin, RoleId::Superuser],
                )
                .await?
                {
                    Some(registered_user) => registered_user,
                    None => return Ok(()),
                }; // end match

                // Check whether or not the pair of API key and secret is
                // verified for switching the bot on.
                if bot_active && !registered_user.exchange_api_data_validated {
                    // Inform the user that it is impossible to switch on the
                    // bot as their credentials are not verified.
                    bot.send_message(msg.chat.id, "Your API credentials are not verified, \
                    if you are sure they are correct, please use <code>/checkexchangeapiconnection</code> command \
                    to verify them").parse_mode(ParseMode::Html).await?;

                    return Ok(());
                } // end if

                // Update the deposit to calculate risk from.
                registered_user.bot_active = bot_active;

                // Save the changes to the database.
                match User::update(&registered_user, conn) {
                    Ok(registered_user) => registered_user,
                    Err(error) => {
                        // Inform the user that an error occurred.
                        bot.send_message(msg.chat.id, ERROR_MESSAGE)
                            .parse_mode(ParseMode::Html)
                            .await?;

                        log::error!("{}", error);

                        // Exit the command execution.
                        return Ok(());
                    } // end Err
                }; // end match

                // Check if the bot becomes active or inactive.
                if registered_user.bot_active {
                    // Inform a user that the bot will be trading for them.
                    bot.send_message(msg.chat.id, "🔥 Alright, I <b>WILL</b> now trade for you!")
                        .parse_mode(ParseMode::Html)
                        .await?;
                } else {
                    bot.send_message(
                        msg.chat.id,
                        "🔥 Alright, I <b>WILL NOT</b> trade for you anymore",
                    )
                    .parse_mode(ParseMode::Html)
                    .await?;
                } // end if
            } // end Command::SetIsActive
            UserCommands::GetSettings => {
                // Verify the user.
                let registered_user = match Self::get_user_and_verify_role(
                    &bot,
                    &msg,
                    registered_user,
                    vec![RoleId::User, RoleId::Admin, RoleId::Superuser],
                )
                .await?
                {
                    Some(registered_user) => registered_user,
                    None => return Ok(()),
                }; // end match

                // Convert some optional fields into convenient-to-read format.

                // Check whether or not the deposit is set.
                let deposit = if registered_user.deposit.is_none() {
                    String::from("Not set")
                } else {
                    String::from("$") + registered_user.deposit.unwrap().to_string().as_str()
                }; // end if

                // Check whether or not the risk is set.
                let risk = if registered_user.risk.is_none() {
                    String::from("Not set")
                } else {
                    registered_user.risk.unwrap().to_string() + "%"
                }; // end if

                // Check whether or not the exchange API key is set.
                let exchange_api_key_info = if registered_user.exchange_api_key.is_none() {
                    String::from("Not set")
                } else {
                    String::from("Set, but unsafe to show")
                }; // end if

                // Check whether or not the exchange API secret is set.
                let exchange_api_secret_info = if registered_user.exchange_api_secret.is_none() {
                    String::from("Not set")
                } else {
                    String::from("Set, but unsafe to show")
                }; // end if

                // Get the days before API key expires, if all the data provided.
                let api_expiration_days = if registered_user.exchange_api_key.is_some()
                    && registered_user.exchange_api_secret.is_some()
                {
                    // Retrieve credentials.
                    let api_key = registered_user.exchange_api_key.clone().unwrap();
                    let api_secret = registered_user.exchange_api_secret.clone().unwrap();

                    // Get an exchange client.
                    let client = ExchangeService::get_client(api_key, api_secret);

                    let api_key_deadline_days =
                        ExchangeService::get_api_deadline_days(&client).await;

                    // Check whether or not it was possible to get the API info.
                    if api_key_deadline_days.is_err() {
                        // An error occurred while accessing user exchange account.
                        log::error!("{}", api_key_deadline_days.err().unwrap().to_string());

                        String::from("@")
                    } else {
                        // Otherwise, safely unwrap the info and display it to the user.
                        let api_key_deadline_days = api_key_deadline_days.unwrap();

                        api_key_deadline_days.to_string()
                    } // end if
                } else {
                    String::from("@")
                }; // end if

                // Get a status whether or not the bot is able to access the exchange
                // with the provided keys.
                let exchange_api_data_validated =
                    if registered_user.exchange_api_data_validated && api_expiration_days != "@" {
                        String::from("✅ API data is <b>valid</b>!")
                    } else {
                        String::from("❌ API data is <b>invalid</b>")
                    }; // end if

                // Generate a message with current user settings.
                let cur_settings_message = format!(
                    "<b>⚙️Settings⚙️</b>\n\n<b>💰 Deposit:</b> {}\n<b>❗️ Risk:</b> {}\n\n<b>🔑 API Exchange key:</b> <em>{}</em>\n<b>㊙️ API Exchange secret:</b> <em>{}</em>\n\n{}\n\n⏳ API key expires in <b>{} days</b>\n\n<b>🔥 Bot allowed to trade:</b> {}",
                        deposit, risk, exchange_api_key_info, exchange_api_secret_info, exchange_api_data_validated, api_expiration_days, registered_user.bot_active
                );

                // Send the current settings to the user.
                bot.send_message(msg.chat.id, cur_settings_message)
                    .parse_mode(ParseMode::Html)
                    .await?;
            } // end UserCommands::GetSettings
            UserCommands::CheckExchangeApiConnection => {
                // Verify the user.
                let mut registered_user = match Self::get_user_and_verify_role(
                    &bot,
                    &msg,
                    registered_user,
                    vec![RoleId::User, RoleId::Admin, RoleId::Superuser],
                )
                .await?
                {
                    Some(registered_user) => registered_user,
                    None => return Ok(()),
                }; // end match

                // Check if exchange API key is set.
                if registered_user.exchange_api_key.is_none() {
                    // The exchange API key is not set.
                    // Inform a user about that.
                    bot.send_message(msg.chat.id, "You did not set <b>API key</b>, please, do it")
                        .parse_mode(ParseMode::Html)
                        .await?;
                    return Ok(());
                } // end if

                // Check if exchange API secret is set.
                if registered_user.exchange_api_secret.is_none() {
                    // The exchange API secret is not set.
                    // Inform a user about that.
                    bot.send_message(
                        msg.chat.id,
                        "You did not set <b>API secret</b>, please, do it",
                    )
                    .parse_mode(ParseMode::Html)
                    .await?;
                    return Ok(());
                } // end if

                // Inform the user that there is an attempt to access their account.
                bot.send_message(
                    msg.chat.id,
                    "Please, wait a little bit.\n\
                I'm trying to access your account...",
                )
                .await?;

                // Get user credentials.
                let api_key = registered_user.exchange_api_key.clone().unwrap();
                let api_secret = registered_user.exchange_api_secret.clone().unwrap();

                // Get an exchange client.
                let client = ExchangeService::get_client(api_key, api_secret);

                // This variable indicates whether or not it was possible to reach out the
                // user's account on the exchange.
                let account_is_accessible: bool;

                // Try to get the deposit of the user on the exchange via
                // provided API keys.
                match ExchangeService::get_uta_total_equity(&client).await {
                    Ok(balance) => {
                        // It was possible to get the user balance.
                        account_is_accessible = true;

                        // Inform the user about that.

                        bot.send_message(
                            msg.chat.id,
                            format!(
                                "<b>I'm able to access your account!🥳🥳🥳</b>\n\n\
                        You have <b>${}</b> for trading :)",
                                balance.floor()
                            ),
                        )
                        .parse_mode(ParseMode::Html)
                        .await?;
                    } // end Ok
                    Err(error) => {
                        log::error!("{}", error);

                        // There was an issue while accessing the user account.
                        account_is_accessible = false;

                        // Inform a user about the issue.
                        bot.send_message(msg.chat.id, "I'm not able to access you account...😔😔😔\nPlease, make sure that you provided the right API key and API secret.").parse_mode(ParseMode::Html).await?;
                    } // end Err.
                }; // end match

                registered_user.exchange_api_data_validated = account_is_accessible;

                // Save the changes to the database.
                match User::update(&registered_user, conn) {
                    Ok(registered_user) => registered_user,
                    Err(error) => {
                        // Inform the user that an error occurred.
                        bot.send_message(msg.chat.id, ERROR_MESSAGE)
                            .parse_mode(ParseMode::Html)
                            .await?;

                        log::error!("{}", error);

                        // Exit the command execution.
                        return Ok(());
                    } // end Err
                }; // end match

                return Ok(());
            } // end Command::CheckExchangeApiConnection
        } // end match

        Ok(())
    } // end fn handle_user_command

    /// This function is a handler for bot commands sent by an admin or higher.
    async fn handle_admin_command(
        bot_state: BotState,
        bot: Throttle<Bot>,
        msg: Message,
        cmd: AdminCommands,
    ) -> ResponseResult<()> {
        // Run all the necessary checks regarding user status and
        // get a pooled database connection.
        let (registered_user, mut conn) =
            match Self::run_user_checks_and_get_db_conn(&bot, &msg, bot_state.clone()).await? {
                Ok((registered_user, conn)) => (registered_user, conn),
                Err(_error) => return Ok(()),
            }; // end match

        // Shadow connection as mutable reference for convenience.
        let conn = &mut conn;

        // Verify the user.
        let registered_user = match Self::get_user_and_verify_role(
            &bot,
            &msg,
            registered_user,
            vec![RoleId::Admin, RoleId::Superuser],
        )
        .await?
        {
            Some(registered_user) => registered_user,
            None => return Ok(()),
        }; // end match

        // Handle an admin command
        match cmd {
            AdminCommands::GetPendingUsers => {
                Self::display_users_by_status(
                    &bot,
                    &msg,
                    StatusId::Pending,
                    RoleId::from(registered_user.role_id),
                    conn,
                )
                .await?;
            } // end AdminCommands::GetPendingUsers
            AdminCommands::GetBannedUsers => {
                Self::display_users_by_status(
                    &bot,
                    &msg,
                    StatusId::Banned,
                    RoleId::from(registered_user.role_id),
                    conn,
                )
                .await?;
            } // end AdminCommands::GetBannedUsers
            AdminCommands::GetApprovedUsers => {
                Self::display_users_by_status(
                    &bot,
                    &msg,
                    StatusId::Approved,
                    RoleId::from(registered_user.role_id),
                    conn,
                )
                .await?;
            } // end AdminCommands::GetApprovedUsers
            AdminCommands::GetRejectedUsers => {
                Self::display_users_by_status(
                    &bot,
                    &msg,
                    StatusId::Rejected,
                    RoleId::from(registered_user.role_id),
                    conn,
                )
                .await?;
            } // end AdminCommands::GetRejectedUsers
            AdminCommands::GetExpiredUsers => {
                Self::display_users_by_status(
                    &bot,
                    &msg,
                    StatusId::Expired,
                    RoleId::from(registered_user.role_id),
                    conn,
                )
                .await?;
            } // end AdminCommands::GetExpiredUsers
            AdminCommands::GetAdmins => {
                Self::display_users_by_role(&bot, &msg, RoleId::Admin, conn).await?
            } // end AdminCommands::GetAdmins
            AdminCommands::MakeAdmin(user_id) => {
                // If the user is trying to modify themselves, then reject the request.
                if !Self::protect_from_self_modification(
                    &bot,
                    &msg,
                    registered_user.telegram_id,
                    user_id,
                )
                .await?
                {
                    // The user cannot modify themselves.
                    return Ok(());
                } // end if

                // Get the user to make an admin from the database.
                let mut edited_user = match User::get_from_db_by_id(user_id, conn) {
                    Ok(edited_user) => edited_user,
                    Err(error) => {
                        log::error!("{}", error);

                        // Inform the user that an error occurred.
                        bot.send_message(msg.chat.id, ERROR_MESSAGE).await?;

                        return Ok(());
                    } // end Err
                }; // end match

                // Check that the user is not an admin already.
                if edited_user.role_id == (RoleId::Admin as i32) {
                    // Inform a user that the user is already an admin.
                    bot.send_message(msg.chat.id, "This user is already an admin")
                        .await?;

                    return Ok(());
                } // end if

                // If the role of the current user is lower than the role of the edited user,
                // than reject the operation.
                if !Self::is_higher_or_same_role(
                    &RoleId::from(registered_user.role_id),
                    &RoleId::from(edited_user.role_id),
                ) {
                    // Inform a user that the user is higher than them.
                    bot.send_message(
                        msg.chat.id,
                        "This user is higher in rank than you, you cannot edit them",
                    )
                    .await?;

                    return Ok(());
                } // end if

                // Change the user role.
                edited_user.role_id = RoleId::Admin as i32;

                // Save changes into the database.
                match User::update(&edited_user, conn) {
                    Ok(_) => {}
                    Err(error) => {
                        log::error!("{}", error);

                        // Inform a user that an error occurred.
                        bot.send_message(msg.chat.id, ERROR_MESSAGE).await?;

                        return Ok(());
                    } // end Err
                } // end match

                // Inform about success.
                bot.send_message(msg.chat.id, "Success!").await?;

                // Inform an affected user about the changes.
                bot.send_message(
                    ChatId(edited_user.telegram_id),
                    "You were made an <b>Admin</b>",
                )
                .parse_mode(ParseMode::Html)
                .await?;
            } // end AdminCommands::MakeAdmin
            AdminCommands::MakeUser(user_id) => {
                // If the user is trying to modify themselves, then reject the request.
                if !Self::protect_from_self_modification(
                    &bot,
                    &msg,
                    registered_user.telegram_id,
                    user_id,
                )
                .await?
                {
                    // The user cannot modify themselves.
                    return Ok(());
                } // end if

                // Get the user to make a user from the database.
                let mut edited_user = match User::get_from_db_by_id(user_id, conn) {
                    Ok(edited_user) => edited_user,
                    Err(error) => {
                        log::error!("{}", error);

                        // Inform the user that an error occurred.
                        bot.send_message(msg.chat.id, ERROR_MESSAGE).await?;

                        return Ok(());
                    } // end Err
                }; // end match

                // Check that the user is not a user already.
                if edited_user.role_id == (RoleId::User as i32) {
                    // Inform a user that the user is already a user.
                    bot.send_message(msg.chat.id, "This user is already a user")
                        .await?;

                    return Ok(());
                } // end if

                // If the role of the current user is lower than the role of the edited user,
                // than reject the operation.
                if !Self::is_higher_or_same_role(
                    &RoleId::from(registered_user.role_id),
                    &RoleId::from(edited_user.role_id),
                ) {
                    // Inform a user that the user is higher than them.
                    bot.send_message(
                        msg.chat.id,
                        "This user is higher in rank than you, you cannot edit them",
                    )
                    .await?;

                    return Ok(());
                } // end if

                // Change the user role.
                edited_user.role_id = RoleId::User as i32;

                // Save changes into the database.
                match User::update(&edited_user, conn) {
                    Ok(_) => {}
                    Err(error) => {
                        log::error!("{}", error);

                        // Inform a user that an error occurred.
                        bot.send_message(msg.chat.id, ERROR_MESSAGE).await?;

                        return Ok(());
                    } // end Err
                } // end match

                // Inform about success.
                bot.send_message(msg.chat.id, "Success!").await?;

                // Inform an affected user about the changes.
                bot.send_message(
                    ChatId(edited_user.telegram_id),
                    "You were made a <b>User</b>",
                )
                .parse_mode(ParseMode::Html)
                .await?;
            } // end AdminCommands::MakeUser
            AdminCommands::Approve(user_id) => {
                // If the user is trying to modify themselves, then reject the request.
                if !Self::protect_from_self_modification(
                    &bot,
                    &msg,
                    registered_user.telegram_id,
                    user_id,
                )
                .await?
                {
                    // The user cannot modify themselves.
                    return Ok(());
                } // end if

                // Get the user to approve from the database.
                let mut edited_user = match User::get_from_db_by_id(user_id, conn) {
                    Ok(edited_user) => edited_user,
                    Err(error) => {
                        log::error!("{}", error);

                        // Inform the user that an error occurred.
                        bot.send_message(msg.chat.id, ERROR_MESSAGE).await?;

                        return Ok(());
                    } // end Err
                }; // end match

                // Check that the user is not approved yet.
                if edited_user.status_id == (StatusId::Approved as i32) {
                    // Inform the message sender that the user is already
                    // approved.
                    bot.send_message(msg.chat.id, "The user is already approved")
                        .await?;

                    return Ok(());
                } // end if

                // If the role of the current user is lower than the role of the edited user,
                // than reject the operation.
                if !Self::is_higher_or_same_role(
                    &RoleId::from(registered_user.role_id),
                    &RoleId::from(edited_user.role_id),
                ) {
                    // Inform a user that the user is higher than them.
                    bot.send_message(
                        msg.chat.id,
                        "This user is higher in rank than you, you cannot edit them",
                    )
                    .await?;

                    return Ok(());
                } // end if

                // Approve the user.
                edited_user.status_id = StatusId::Approved as i32;

                // Save changes into the database.
                match User::update(&edited_user, conn) {
                    Ok(_) => {}
                    Err(error) => {
                        log::error!("{}", error);

                        // Inform a user that an error occurred.
                        bot.send_message(msg.chat.id, ERROR_MESSAGE).await?;

                        return Ok(());
                    } // end Err
                } // end match

                // Inform about success.
                bot.send_message(msg.chat.id, "Success!").await?;

                // Inform an affected user about the changes.
                bot.send_message(
                    ChatId(edited_user.telegram_id),
                    "I'm thrilled to tell you, that you were <b>approved</b> to use me!🥳🥳🥳\n\
                    Use <code>/help</code> command to see how to use me 😜",
                )
                .parse_mode(ParseMode::Html)
                .await?;
            } // end AdminCommands::Approve
            AdminCommands::Reject(user_id) => {
                // If the user is trying to modify themselves, then reject the request.
                if !Self::protect_from_self_modification(
                    &bot,
                    &msg,
                    registered_user.telegram_id,
                    user_id,
                )
                .await?
                {
                    // The user cannot modify themselves.
                    return Ok(());
                } // end if

                // Get the user to reject from the database.
                let mut edited_user = match User::get_from_db_by_id(user_id, conn) {
                    Ok(edited_user) => edited_user,
                    Err(error) => {
                        log::error!("{}", error);

                        // Inform the user that an error occurred.
                        bot.send_message(msg.chat.id, ERROR_MESSAGE).await?;

                        return Ok(());
                    } // end Err
                }; // end match

                // Check that the user is not approved yet.
                if edited_user.status_id == (StatusId::Rejected as i32) {
                    // Inform the message sender that the user is already
                    // rejected.
                    bot.send_message(msg.chat.id, "The user is already rejected")
                        .await?;

                    return Ok(());
                } // end if

                // If the role of the current user is lower than the role of the edited user,
                // than reject the operation.
                if !Self::is_higher_or_same_role(
                    &RoleId::from(registered_user.role_id),
                    &RoleId::from(edited_user.role_id),
                ) {
                    // Inform a user that the user is higher than them.
                    bot.send_message(
                        msg.chat.id,
                        "This user is higher in rank than you, you cannot edit them",
                    )
                    .await?;

                    return Ok(());
                } // end ifs

                // Reject the user.
                edited_user.status_id = StatusId::Rejected as i32;

                // Save changes into the database.
                match User::update(&edited_user, conn) {
                    Ok(_) => {}
                    Err(error) => {
                        log::error!("{}", error);

                        // Inform a user that an error occurred.
                        bot.send_message(msg.chat.id, ERROR_MESSAGE).await?;

                        return Ok(());
                    } // end Err
                } // end match

                // Inform about success.
                bot.send_message(msg.chat.id, "Success!").await?;

                // Inform an affected user about the changes.
                bot.send_message(
                    ChatId(edited_user.telegram_id),
                    "I' sorry to inform you, that your request was <b>rejected</b>",
                )
                .parse_mode(ParseMode::Html)
                .await?;
            }
            AdminCommands::Ban(user_id) => {
                // If the user is trying to modify themselves, then reject the request.
                if !Self::protect_from_self_modification(
                    &bot,
                    &msg,
                    registered_user.telegram_id,
                    user_id,
                )
                .await?
                {
                    // The user cannot modify themselves.
                    return Ok(());
                } // end if

                // Get the user to approve from the database.
                let mut edited_user = match User::get_from_db_by_id(user_id, conn) {
                    Ok(edited_user) => edited_user,
                    Err(error) => {
                        log::error!("{}", error);

                        // Inform the user that an error occurred.
                        bot.send_message(msg.chat.id, ERROR_MESSAGE).await?;

                        return Ok(());
                    } // end Err
                }; // end match

                // Check that the user is not banned yet.
                if edited_user.status_id == (StatusId::Banned as i32) {
                    // Inform the message sender that the user is already
                    // banned.
                    bot.send_message(msg.chat.id, "The user is already banned")
                        .await?;

                    return Ok(());
                } // end if

                // If the role of the current user is lower than the role of the edited user,
                // than reject the operation.
                if !Self::is_higher_or_same_role(
                    &RoleId::from(registered_user.role_id),
                    &RoleId::from(edited_user.role_id),
                ) {
                    // Inform a user that the user is higher than them.
                    bot.send_message(
                        msg.chat.id,
                        "This user is higher in rank than you, you cannot edit them",
                    )
                    .await?;

                    return Ok(());
                } // end if

                // Ban the user.
                edited_user.status_id = StatusId::Banned as i32;

                // Save changes into the database.
                match User::update(&edited_user, conn) {
                    Ok(_) => {}
                    Err(error) => {
                        log::error!("{}", error);

                        // Inform a user that an error occurred.
                        bot.send_message(msg.chat.id, ERROR_MESSAGE).await?;

                        return Ok(());
                    } // end Err
                } // end match

                // Inform about success.
                bot.send_message(msg.chat.id, "Success!").await?;

                // Inform an affected user about the changes.
                bot.send_message(
                    ChatId(edited_user.telegram_id),
                    "I'm sorry to inform you that you were <b>banned</b>",
                )
                .parse_mode(ParseMode::Html)
                .await?;
            }
            AdminCommands::Expire(user_id) => {
                // If the user is trying to modify themselves, then reject the request.
                if !Self::protect_from_self_modification(
                    &bot,
                    &msg,
                    registered_user.telegram_id,
                    user_id,
                )
                .await?
                {
                    // The user cannot modify themselves.
                    return Ok(());
                } // end if

                // Get the user from the database whose subscription to make expired.
                let mut edited_user = match User::get_from_db_by_id(user_id, conn) {
                    Ok(edited_user) => edited_user,
                    Err(error) => {
                        log::error!("{}", error);

                        // Inform the user that an error occurred.
                        bot.send_message(msg.chat.id, ERROR_MESSAGE).await?;

                        return Ok(());
                    } // end Err
                }; // end match

                // Check that the user subscription is not expired yet.
                if edited_user.status_id == (StatusId::Expired as i32) {
                    // Inform the message sender that the user subscription has already
                    // expired.
                    bot.send_message(msg.chat.id, "The user subscription has already expired")
                        .await?;

                    return Ok(());
                } // end if

                // If the role of the current user is lower than the role of the edited user,
                // than reject the operation.
                if !Self::is_higher_or_same_role(
                    &RoleId::from(registered_user.role_id),
                    &RoleId::from(edited_user.role_id),
                ) {
                    // Inform a user that the user is higher than them.
                    bot.send_message(
                        msg.chat.id,
                        "This user is higher in rank than you, you cannot edit them",
                    )
                    .await?;

                    return Ok(());
                } // end if

                // Ban the user.
                edited_user.status_id = StatusId::Expired as i32;

                // Save changes into the database.
                match User::update(&edited_user, conn) {
                    Ok(_) => {}
                    Err(error) => {
                        log::error!("{}", error);

                        // Inform a user that an error occurred.
                        bot.send_message(msg.chat.id, ERROR_MESSAGE).await?;

                        return Ok(());
                    } // end Err
                } // end match

                // Inform about success.
                bot.send_message(msg.chat.id, "Success!").await?;

                // Inform an affected user about the changes.
                bot.send_message(
                    ChatId(edited_user.telegram_id),
                    "I'm sorry to inform you that your subscription was made <b>expired</b>.\n\n\
                    From now on I will not trade for you anymore, until you renew your subscription.",
                )
                .parse_mode(ParseMode::Html)
                .await?;
            }
        } // end match

        Ok(())
    } // end fn handle_admin_command

    /// This function is a handler for bot commands sent by a superuser or higher.
    async fn handle_superuser_command(
        bot_state: BotState,
        bot: Throttle<Bot>,
        msg: Message,
        cmd: SuperuserCommands,
    ) -> ResponseResult<()> {
        // Run all the necessary checks regarding user status and
        // get a pooled database connection.
        let (registered_user, mut conn) =
            match Self::run_user_checks_and_get_db_conn(&bot, &msg, bot_state.clone()).await? {
                Ok((registered_user, conn)) => (registered_user, conn),
                Err(_error) => return Ok(()),
            }; // end match

        // Shadow connection as mutable reference for convenience.
        let conn = &mut conn;

        // Verify the user.
        let registered_user = match Self::get_user_and_verify_role(
            &bot,
            &msg,
            registered_user,
            vec![RoleId::Superuser],
        )
        .await?
        {
            Some(registered_user) => registered_user,
            None => return Ok(()),
        }; // end match

        // Handle superuser command.
        match cmd {
            SuperuserCommands::MakeSuperuser(user_id) => {
                // If the user is trying to modify themselves, then reject the request.
                if !Self::protect_from_self_modification(
                    &bot,
                    &msg,
                    registered_user.telegram_id,
                    user_id,
                )
                .await?
                {
                    // The user cannot modify themselves.
                    return Ok(());
                } // end if

                // Get the user to make a superuser from the database.
                let mut edited_user = match User::get_from_db_by_id(user_id, conn) {
                    Ok(edited_user) => edited_user,
                    Err(error) => {
                        log::error!("{}", error);

                        // Inform the user that an error occurred.
                        bot.send_message(msg.chat.id, ERROR_MESSAGE).await?;

                        return Ok(());
                    } // end Err
                }; // end match

                // Check that the user is not a superuser already.
                if edited_user.role_id == (RoleId::Superuser as i32) {
                    // Inform a user that the user is already an admin.
                    bot.send_message(msg.chat.id, "This user is already a superuser")
                        .await?;

                    return Ok(());
                } // end if

                // If the role of the current user is lower than the role of the edited user,
                // than reject the operation.
                if !Self::is_higher_or_same_role(
                    &RoleId::from(registered_user.role_id),
                    &RoleId::from(edited_user.role_id),
                ) {
                    // Inform a user that the user is higher than them.
                    bot.send_message(
                        msg.chat.id,
                        "This user is higher in rank than you, you cannot edit them",
                    )
                    .await?;

                    return Ok(());
                } // end if

                // Make the user a superuser.
                edited_user.role_id = RoleId::Superuser as i32;

                // Save changes into the database.
                match User::update(&edited_user, conn) {
                    Ok(_) => {}
                    Err(error) => {
                        log::error!("{}", error);

                        // Inform a user that an error occurred.
                        bot.send_message(msg.chat.id, ERROR_MESSAGE).await?;

                        return Ok(());
                    } // end Err
                } // end match

                // Inform about success.
                bot.send_message(msg.chat.id, "Success!").await?;

                // Inform an affected user about the changes.
                bot.send_message(
                    ChatId(edited_user.telegram_id),
                    "You were made a <b>Superuser</b>",
                )
                .parse_mode(ParseMode::Html)
                .await?;
            } // end SuperuserCommands::MakeSuperuser
            SuperuserCommands::GetSuperusers => {
                Self::display_users_by_role(&bot, &msg, RoleId::Superuser, conn).await?;
            } // end SuperuserCommands::GetSuperusers
            SuperuserCommands::Broadcast => {
                // Broadcast a message to all approved users.

                // Add a user to the broadcast dialog state.
                bot_state
                    .user_dialog_states
                    .lock()
                    .await
                    .insert(registered_user.telegram_id, DialogState::Broadcast);

                // Request a message to broadcast.
                bot.send_message(msg.chat.id, "Ok, send me a message to broadcast")
                    .await?;
            } // end SuperuserCommands::Broadcast
        } // end match

        Ok(())
    } // end fn handle_superuser_command

    /// This function reads incoming posts from a trusted channel and reacts to them.
    async fn read_channel_post(
        bot: Throttle<Bot>,
        msg: Message,
        bot_state: BotState,
    ) -> ResponseResult<()> {
        // Get the connection to the database.
        let conn = &mut match DatabaseService::get_pool_connection(&bot_state.pg_connection_pool) {
            Ok(conn) => conn,
            Err(error) => {
                log::error!("{}", error);

                // Inform a user about the error that occurred.
                bot.send_message(
                    ChatId(bot_state.log_channel_id),
                    "Failed to connect to the database to save the post",
                )
                .await?;

                return Ok(());
            } // end Err
        }; // end match

        // Get the ID of the message the current message replies to.
        let response_message_id = if let Some(response_msg) = msg.reply_to_message() {
            // If the current message is a reply to any other message in the channel.
            Some(response_msg.id.0)
        } else {
            // If this message does not reply to any other message in the channel.
            None
        }; // end if

        // Get the message text.
        let message_text = if let Some(msg_text) = msg.text() {
            // The message contains some text.
            Some(msg_text.to_string())
        } else {
            // The message does not contain any text.
            None
        }; // end if

        // Save the post to database to be able to restore the message history in the future.
        match ChannelHistory::insert_into_db(
            &ChannelHistory {
                message_id: msg.id.0,
                response_message_id,
                message_text,
                message_timestamp: Utc::now(),
            },
            conn,
        ) {
            Ok(_) => (),
            Err(error) => {
                log::error!("{}", error);

                // Log the error into the log channel.
                bot.send_message(
                    ChatId(bot_state.log_channel_id),
                    "Failed to save a new post to the database...",
                )
                .await?;

                return Ok(());
            } // end Err
        } // end match

        // Handle a new post.
        match MessageService::handle_message(msg, bot_state, &bot).await {
            Ok(_) => {
                log::info!("Successfully processed a post!");
            }
            Err(error) => {
                log::error!("{}", error);
            } // end match
        } // end match

        Ok(())
    } // end fn read_channel_post

    /// This function runs various checks for the user
    /// and extracts a database connection.
    ///
    /// The function makes all the checks regarding user status.
    /// The function also retrieves the user from the database.
    /// The function gets a pooled connection to the database.
    ///
    /// The function returns the user if they are registered.
    /// The function returns outer error, if there was a problem with
    /// sending a message to the user in telegram.
    /// The function returns inner error, if the database connection could
    /// not be retrieved or if the user is suspicious.
    async fn run_user_checks_and_get_db_conn(
        bot: &Throttle<Bot>,
        msg: &Message,
        bot_state: BotState,
    ) -> ResponseResult<
        Result<
            (
                Option<User>,
                PooledConnection<ConnectionManager<PgConnection>>,
            ),
            (),
        >,
    > {
        // Get the user ID of the message sender.
        let user = MessageService::get_user(msg);

        // Check if the user exists.
        if user.is_none() {
            // Do not continue answering on this request.
            return Ok(Err(()));
        } // end if

        // Safely unwrap the user.
        let user = user.unwrap();

        // Check if the user is a channel or a bot,
        // then do not react to the message.
        if user.is_bot || user.is_channel() || user.is_anonymous() {
            return Ok(Err(()));
        } // end if

        // Try to get a connection to the database.
        let conn = match DatabaseService::get_pool_connection(&bot_state.pg_connection_pool) {
            Ok(conn) => Some(conn),
            Err(error) => {
                log::error!("{}", error);
                None
            } // end Err
        }; // end match

        // Check if the connection was retrieved successfully.
        if conn.is_none() {
            // Inform a user about an error.
            bot.send_message(msg.chat.id, ERROR_MESSAGE)
                .parse_mode(ParseMode::Html)
                .await?;

            return Ok(Err(()));
        } // end if

        // Otherwise safely unwrap the connection.
        let mut conn = conn.unwrap();

        // Try retrieve the user from the database.
        let registered_user: Option<User> =
            match User::get_from_db_by_id(user.id.0 as i64, &mut conn) {
                Ok(user) => Some(user),
                Err(error) => {
                    // Check if the user is registered in the database.
                    // If not, then the error is something different.
                    if error == diesel::result::Error::NotFound {
                        // The user is not registered in the database.
                        None
                    } else {
                        // Inform a user about an error.
                        bot.send_message(msg.chat.id, ERROR_MESSAGE)
                            .parse_mode(ParseMode::Html)
                            .await?;

                        log::error!("{}", error);

                        return Ok(Err(()));
                    } // end if
                } // end Err
            }; // end match

        // Check if the user is registered.
        if let Some(registered_user) = registered_user {
            // Check if the user is allowed to use the bot.
            match StatusId::from(registered_user.status_id) {
                // The user is in pending mode.
                StatusId::Pending => {
                    // Inform a user that they are in pending mode.
                    bot.send_message(msg.chat.id, "You are not approved for using this bot yet.\nAn admin should review your application soon.")
                            .parse_mode(ParseMode::Html)
                            .await?;

                    return Ok(Err(()));
                } // end StatusId::Pending
                // The user's request was rejected.
                StatusId::Rejected => {
                    // Inform a user that they are rejected.
                    bot.send_message(
                        msg.chat.id,
                        "I'm sorry to inform you that your request was rejected.",
                    )
                    .parse_mode(ParseMode::Html)
                    .await?;

                    return Ok(Err(()));
                } // end StatusId::Rejected
                // The user is approved and can use the bot.
                StatusId::Approved => return Ok(Ok((Some(registered_user), conn))),
                // The user is banned.
                StatusId::Banned => {
                    // Inform a user that they are banned.
                    bot.send_message(msg.chat.id, "I'm sorry to inform you that you were banned.")
                        .parse_mode(ParseMode::Html)
                        .await?;

                    return Ok(Err(()));
                } // end StatusId::Banned
                // User subscription to the bot expired.
                StatusId::Expired => {
                    // Inform a user that the subscription expired.
                    bot.send_message(msg.chat.id, "Your subscription expired, please renew it.")
                        .parse_mode(ParseMode::Html)
                        .await?;

                    return Ok(Err(()));
                } // end StatusId::Expired
            } // end match
        } // end if

        return Ok(Ok((None, conn)));
    } // end run_user_checks_and_get_db_conn

    /// This function verifies that the user is registered.
    /// Then function verifies the user role.
    /// Returns None if the user did not pass verification.
    /// Returns User otherwise.
    async fn get_user_and_verify_role(
        bot: &Throttle<Bot>,
        msg: &Message,
        user: Option<User>,
        allowed_roles: Vec<RoleId>,
    ) -> ResponseResult<Option<User>> {
        // Check if the user is registered.
        if user.is_none() {
            // The user is not registered in the service.
            // Ask a user to register with '/start' command.
            bot.send_message(
                msg.chat.id,
                "You are not registered, please use <code>/start</code> command to sign up.",
            )
            .parse_mode(ParseMode::Html)
            .await?;

            return Ok(None);
        } // end if

        // Safely unwrap the user.
        let user = user.unwrap();

        // The user is registered.
        // Traverse each role and find out if the user is assigned
        // one of the allowed roles.
        for rl in allowed_roles {
            // Check if the user is assigned a particular role.
            if user.role_id == i32::from(rl) {
                // The user is allowed to proceed.
                return Ok(Some(user));
            } // end if
        } // end for

        // Otherwise the user is not allowed to proceed.
        // The user did not pass the verification.
        return Ok(None);
    } // end fn verify_role

    /// This function displays a list of users by status in a formatted way.
    ///
    /// caller_role - a role of the user that calls the function.
    /// (This is necessary not to disclose the information about higher ranked users)
    async fn display_users_by_status(
        bot: &Throttle<Bot>,
        msg: &Message,
        status_id: StatusId,
        caller_role: RoleId,
        conn: &mut PgConnection,
    ) -> ResponseResult<()> {
        // Retrieve all users with a particular status from the database.
        let retrieved_users_raw = match User::get_users_with_status(&status_id, conn) {
            Ok(retrieved_users) => retrieved_users,
            Err(error) => {
                log::error!("{}", error);

                // Inform a user about an error.
                bot.send_message(msg.chat.id, ERROR_MESSAGE).await?;

                return Ok(());
            } // end Err
        }; // end match

        // Get the list of users who are the same or lower rank than the caller.
        let mut retrieved_users: Vec<User> = Vec::new();

        // Traverse each user and check whether or not there rank is higher than
        // the caller rank.
        for retrieved_user in retrieved_users_raw {
            // Check if the rank of the caller is the same or higher than
            // the rank of the current user.
            if Self::is_higher_or_same_role(&caller_role, &RoleId::from(retrieved_user.role_id)) {
                retrieved_users.push(retrieved_user);
            } // end if
        } // end for

        // A header for a list.
        let list_header = &format!("{} {} {}\n", "#", "Username", "ID");

        // If there are no retrieved users, then inform the user
        // about it.
        if retrieved_users.is_empty() {
            // Find a proper adjective to describe the user.
            let user_description = match status_id {
                StatusId::Pending => "pending",
                StatusId::Rejected => "rejected",
                StatusId::Approved => "approved",
                StatusId::Banned => "banned",
                StatusId::Expired => "expired",
            }; // end match

            // Inform a user that no users were found.
            bot.send_message(msg.chat.id, format!("No {} users found", user_description))
                .await?;

            return Ok(());
        } // end if

        // Prepare a message to display the users.
        let mut reply = list_header.clone();

        // Traverse each user and add them to the reply message.
        for (idx, usr) in retrieved_users.into_iter().enumerate() {
            // Display banned users (10 per message).
            if idx % 10 == 0 && idx > 0 {
                // Send a message to the user.
                bot.send_message(msg.chat.id, &reply)
                    .parse_mode(ParseMode::Html)
                    .await?;

                // Reset reply message.
                reply = list_header.clone();
            } // end if

            reply += &format!(
                "{}. <b>{}</b>......<code>{}</code>\n",
                idx + 1,
                usr.telegram_username.unwrap_or(String::from("None")),
                usr.telegram_id
            );
        } // end for

        // Display the rest of the users.
        if !reply.is_empty() {
            bot.send_message(msg.chat.id, &reply)
                .parse_mode(ParseMode::Html)
                .await?;
        } // end if

        Ok(())
    } // end display_users_by_status

    /// This function assembles and returns a user manual on
    /// how to use the bot, taking into account the user role in the system.
    fn assemble_user_manual(role_id: RoleId) -> Result<Vec<String>, std::io::Error> {
        // Load the raw data from a manual JSON file.
        let raw_manual = std::fs::read_to_string(PATH_TO_BOT_MANUAL)?;

        // Deserialize the JSON data
        let commands: Commands = serde_json::from_str(&raw_manual)?;

        // Returned manual.
        let mut manual: Vec<String> = Vec::new();

        // The current manual.
        let mut cur_manual = String::from("<b>These commands are supported:</b>\n\n");

        // Assemble commands for a regular user.
        for command in commands.user {
            cur_manual += &Self::get_command_description(command);
        } // end for

        // Set up the first message containing only user commands.
        manual.push(cur_manual);

        // Reset the current manual.
        cur_manual = String::new();

        // If user's rank is higher or equal to admin, then add some more
        // instructions.
        if role_id == RoleId::Admin || role_id == RoleId::Superuser {
            cur_manual += "<b>These commands are supported for admins:</b>\n\n";

            // Assemble commands for a admin user.
            for command in commands.admin {
                cur_manual += &Self::get_command_description(command);
            } // end for
        } // end if

        // Set up the second message containing only admin commands.
        manual.push(cur_manual);

        // Reset the current manual.
        cur_manual = String::new();

        // If user's rank is higher or equal to superuser, then add some more
        // instructions.
        if role_id == RoleId::Admin || role_id == RoleId::Superuser {
            cur_manual += "<b>These commands are supported for superusers:</b>\n\n";

            // Assemble commands for a superuser user.
            for command in commands.superuser {
                cur_manual += &Self::get_command_description(command);
            } // end for
        } // end if

        // Set up the third message containing only superuser commands.
        manual.push(cur_manual);

        Ok(manual)
    } // end fn assemble_user_manual

    fn get_command_description(command: Command) -> String {
        format!("🖥 Command: <code>{}</code>\n📜 Description: <em>{}</em>\n\n🧿 Example:\n\n👤 You: <code>{}</code>\n🤖 Bot: {}\n\n\n\n", command.command, command.description, command.example.command, command.example.response)
    } // end fn get_command_description

    /// This function displays a list of users by role in a formatted way.
    async fn display_users_by_role(
        bot: &Throttle<Bot>,
        msg: &Message,
        role_id: RoleId,
        conn: &mut PgConnection,
    ) -> ResponseResult<()> {
        // Retrieve all users with a particular status from the database.
        let retrieved_users = match User::get_users_with_role(&role_id, conn) {
            Ok(retrieved_users) => retrieved_users,
            Err(error) => {
                log::error!("{}", error);

                // Inform a user about an error.
                bot.send_message(msg.chat.id, ERROR_MESSAGE).await?;

                return Ok(());
            } // end Err
        };

        // A header for a list.
        let list_header = &format!("{} {} {}\n", "#", "Username", "ID");

        // If there are no retrieved users, then inform the user
        // about it.
        if retrieved_users.is_empty() {
            // Find a proper adjective to describe the user.
            let user_description = match role_id {
                RoleId::User => "user",
                RoleId::Admin => "admin",
                RoleId::Superuser => "superuser",
            }; // end match

            // Inform a user that no users were found.
            bot.send_message(msg.chat.id, format!("No {} users found", user_description))
                .await?;

            return Ok(());
        } // end if

        // Prepare a message to display the users.
        let mut reply = list_header.clone();

        // Traverse each user and add them to the reply message.
        for (idx, usr) in retrieved_users.into_iter().enumerate() {
            // Display banned users (10 per message).
            if idx % 10 == 0 && idx > 0 {
                // Send a message to the user.
                bot.send_message(msg.chat.id, &reply).await?;

                // Reset reply message.
                reply = list_header.clone();
            } // end if

            reply += &format!(
                "{}. <b>{}</b>......<code>{}</code>\n",
                idx + 1,
                usr.telegram_username.unwrap_or(String::from("None")),
                usr.telegram_id
            );
        } // end for

        // Display the rest of the users.
        if !reply.is_empty() {
            bot.send_message(msg.chat.id, &reply)
                .parse_mode(ParseMode::Html)
                .await?;
        } // end if

        Ok(())
    } // end display_users_by_role

    /// This function returns
    ///     true - if the caller role is the same or higher than the role of
    ///            other user.
    ///     false - otherwise.
    fn is_higher_or_same_role(caller_role: &RoleId, other_role: &RoleId) -> bool {
        // Convert roles to numbers.
        let caller_role_num = caller_role.clone() as i32;
        let other_role_num = other_role.clone() as i32;

        // Compare two roles as numbers and return the result.
        caller_role_num >= other_role_num
    } // end fn is_higher_or_same_role

    /// This function protects a user from self-modification.
    /// It disallows a user to change their own role or status for safety reasons.
    ///
    /// The function returns:
    ///     true - if the user is not trying to modify themselves.
    ///     false - otherwise.
    async fn protect_from_self_modification(
        bot: &Throttle<Bot>,
        msg: &Message,
        caller_id: i64,
        other_id: i64,
    ) -> ResponseResult<bool> {
        // If the user is trying to modify themselves, then reject the request.
        if caller_id == other_id {
            // Inform the user that it is impossible to do so.
            bot.send_message(msg.chat.id, "You cannot modify yourself")
                .parse_mode(ParseMode::Html)
                .await?;

            return Ok(false);
        } // end if

        Ok(true)
    } // end protect_from_self_modification

    /// This function allows to broadcast notifications to users with respect to the telegram limits.
    pub async fn broadcast_messages(bot: Throttle<Bot>, users_messages: Vec<(i64, String)>) {
        // Traverse all the messages and send them to the users.
        for (telegram_id, message) in users_messages {
            // Send a message to a user.
            // Make several attempts to inform a client before skipping them.
            for i in 0..MAX_BROADCAST_ATTEMPTS {
                // Try to send a message to the user and get the result.
                let res = Self::inform_client(&bot, telegram_id, &message).await;

                // Check if the result is success, then get out of the loop.
                if res.is_ok() {
                    break;
                } // end if

                // Get the error.
                let req_error = res.err().unwrap();

                // The time to wait before repeating the request.
                let mut time_to_wait: tokio::time::Duration = tokio::time::Duration::from_secs(1);

                // Handle the error.
                match req_error {
                    teloxide::RequestError::RetryAfter(wait_time) => time_to_wait = wait_time,
                    teloxide::RequestError::Api(api_error) => match api_error {
                        teloxide::ApiError::BotBlocked => break,
                        teloxide::ApiError::NotFound => break,
                        teloxide::ApiError::ChatNotFound => break,
                        teloxide::ApiError::BotKicked => break,
                        teloxide::ApiError::CantInitiateConversation => break,
                        teloxide::ApiError::MessageIsTooLong => break,
                        _ => {
                            // Otherwise log the error.
                            // Safely unwrap the error before logging.
                            log::error!("{:#?}", api_error.to_string());
                        }
                    },
                    _ => {
                        // Otherwise log the error.
                        // Safely unwrap the error before logging.
                        log::error!("{:#?}", req_error.to_string());
                    }
                } // end match

                // Check if it was the last attempt to broadcast a message to the user,
                // then get out of the loop.
                if i == MAX_BROADCAST_ATTEMPTS - 1 {
                    break;
                } // end if

                // Wait for some time before making the next attempt.
                tokio::time::sleep(time_to_wait).await;
            } // end for
        } // end for
    } // end broadcast_messages
} // end impl BotService
