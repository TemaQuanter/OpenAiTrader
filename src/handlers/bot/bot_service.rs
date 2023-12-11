// This file contains all the functionality necessary to interact
// with a user via Telegram bot.

use std::sync::Arc;

use chatgpt::prelude::Conversation;
use chrono::Utc;
use diesel::{
    r2d2::{ConnectionManager, Pool, PooledConnection},
    PgConnection,
};
use rust_decimal::Decimal;

use teloxide::{
    adaptors::Throttle,
    dispatching::{HandlerExt, UpdateFilterExt},
    dptree,
    payloads::SendMessageSetters,
    prelude::Dispatcher,
    requests::{Requester, ResponseResult},
    types::{ChatId, Message, ParseMode, Update},
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

/// This structure contains a common state for bot message
/// handler.
#[derive(Clone)]
pub struct BotState {
    /// Database connection pool.
    pub pg_connection_pool: Arc<Pool<ConnectionManager<PgConnection>>>,
    /// ChatGPT conversation.
    pub chatgpt_conversation: Arc<Mutex<Conversation>>,
    /// This is a trusted channel ID that the bot
    /// gets trading signals from.
    pub trusted_channel_id: i64,
    /// This is a channel where the bot sends its logs.
    pub log_channel_id: i64,
} // end struct BotConfig

/// These are all commands that the bot can handle for general user.
#[derive(BotCommands, Clone)]
#[command(
    rename_rule = "lowercase",
    description = "These commands are supported:"
)]
enum UserCommands {
    #[command(description = "start interaction with the bot.")]
    Start,
    #[command(description = "display this text.")]
    Help,
    #[command(description = "set a new deposit amount (for risk calculation)")]
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
} // end enum SuperuserCommands

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
            );

        // A handler that determines the behavior of the bot in regards to
        // channel posts.
        let channel_handler = Update::filter_channel_post().branch(
            // Allow posts only from a trusted channel.
            dptree::filter(move |msg: Message| msg.chat.id == ChatId(bot_state.trusted_channel_id))
                .endpoint(Self::read_channel_post),
        ); // end Update::filter_message

        // Main handler that joins all handlers into one.
        let handler = dptree::entry().branch(chat_handler).branch(channel_handler);

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
                    bot.send_message(msg.chat.id, "Welcome to <b>OPEN-AI-TRADER</b> bot!")
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
                bot.send_message(msg.chat.id, "Hi! I added you to a waiting list, please wait for admin to review your application")
                    .parse_mode(ParseMode::Html)
                    .await?
            } // end UserCommands::Start
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
                let mut reply = UserCommands::descriptions().to_string();

                // If the user has Admin or Superuser role, then add more commands.
                if vec![RoleId::Admin, RoleId::Superuser]
                    .contains(&RoleId::from(registered_user.role_id))
                {
                    // Add admin commands as well.
                    reply += &format!("\n\n{}", AdminCommands::descriptions().to_string());
                } // end if

                // If the user has Superuser role, then add more commands.
                if vec![RoleId::Superuser].contains(&RoleId::from(registered_user.role_id)) {
                    // Add superuser commands as well.
                    reply += &format!("\n\n{}", SuperuserCommands::descriptions().to_string());
                } // end if

                // Send the help instructions to the user.
                bot.send_message(msg.chat.id, reply)
                    .parse_mode(ParseMode::Html)
                    .await?
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
                        "💰 Deposit amount is set to <b>${}</b>!",
                        registered_user.deposit.unwrap_or_default()
                    ),
                )
                .parse_mode(ParseMode::Html)
                .await?
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
                .await?
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

                bot.send_message(
                    msg.chat.id,
                    format!(
                        "🔑  Exchange API key is set to <b>{}</b>",
                        registered_user.exchange_api_key.unwrap_or_default()
                    ),
                )
                .parse_mode(ParseMode::Html)
                .await?
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

                bot.send_message(
                    msg.chat.id,
                    format!(
                        "㊙️  Exchange API secret is set to <b>{}</b>",
                        registered_user.exchange_api_secret.unwrap_or_default()
                    ),
                )
                .parse_mode(ParseMode::Html)
                .await?
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
                    bot.send_message(
                        msg.chat.id,
                        "🔥 Alright, bot <b>WILL</b> now trade for you!",
                    )
                    .parse_mode(ParseMode::Html)
                    .await?
                } else {
                    bot.send_message(
                        msg.chat.id,
                        "🔥 Alright, bot <b>WILL NOT</b> trade for you anymore",
                    )
                    .parse_mode(ParseMode::Html)
                    .await?
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

                // Get a status whether or not the bot is able to access the exchange
                // with the provided keys.
                let exchange_api_data_validated = if registered_user.exchange_api_data_validated {
                    String::from("✅ API data is <b>valid</b>!")
                } else {
                    String::from("❌ API data is <b>invalid</b>")
                }; // end if

                // Generate a message with current user settings.
                let cur_settings_message = format!(
                    "<b>⚙️Settings⚙️</b>\n\n<b>💰 Deposit:</b> {}\n<b>❗️ Risk:</b> {}\n\n<b>🔑 API Exchange key:</b> <em>{}</em>\n<b>㊙️ API Exchange secret:</b> <em>{}</em>\n\n{}\n\n<b>🔥 Bot allowed to trade:</b> {}",
                        deposit, risk, exchange_api_key_info, exchange_api_secret_info, exchange_api_data_validated, registered_user.bot_active
                );

                // Send the current settings to the user.
                bot.send_message(msg.chat.id, cur_settings_message)
                    .parse_mode(ParseMode::Html)
                    .await?
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

                        bot.send_message(msg.chat.id, format!("<b>I'm able to access your account!🥳🥳🥳</b>\n\nYou have <b>${}</b> for trading :)", balance.floor())).parse_mode(ParseMode::Html).await?;
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
        }; // end match

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
    } // end display_users_by_status

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
} // end impl BotService
