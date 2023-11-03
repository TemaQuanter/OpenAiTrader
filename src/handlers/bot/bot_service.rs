// This file contains all the functionality necessary to interact
// with a user via Telegram bot.

use diesel::{
    r2d2::{ConnectionManager, Pool},
    PgConnection,
};
use rust_decimal::Decimal;

use teloxide::{
    dispatching::{HandlerExt, UpdateFilterExt},
    dptree,
    payloads::SendMessageSetters,
    prelude::Dispatcher,
    repls::CommandReplExt,
    requests::{Requester, ResponseResult},
    types::{Message, ParseMode, Update},
    utils::command::BotCommands,
    Bot,
};
use tokio::task::JoinHandle;

use crate::{
    db_objects::{role_model::RoleId, status_model::StatusId, user_model::User},
    utils::{
        database_service::DatabaseService,
        types_and_constants::{OneResult, ERROR_MESSAGE},
    },
};

use super::message_service::MessageService;

/// This structure contains a common state for bot message
/// handler.
#[derive(Clone)]
pub struct BotState {
    /// Database connection pool.
    pub pg_connection_pool: Pool<ConnectionManager<PgConnection>>,
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
} // end enum UserCommands

// These are all commands that the bot can handle for admin user.
#[derive(BotCommands, Clone)]
#[command(
    rename_rule = "lowercase",
    description = "These commands are supported for admins:"
)]
enum AdminCommands {} // end enum AdminCommand

// These are all commands that the bot can handle for superuser.
#[derive(BotCommands, Clone)]
#[command(
    rename_rule = "lowercase",
    description = "These commands are supported for admins:"
)]
enum SuperuserCommands {} // end enum SuperuserCommands

/// This structure contains all the functionality
/// necessary to drive a Telegram bot.
pub struct BotService;

/// Implement all the functionality necessary to drive a Telegram
/// bot.
impl BotService {
    /// This function initializes a telegram bot.
    pub fn new_bot(tg_bot_api: &String) -> OneResult<Bot> {
        let bot = Bot::new(tg_bot_api);

        Ok(bot)
    } // end fn new_bot

    /// This function sets up a bot listener that handles every incoming message,
    /// whether it is a command, common message or anything other type.
    pub async fn set_listener(bot: Bot, bot_state: BotState) -> JoinHandle<()> {
        // A handler that determines the behavior of the bot.
        let chat_handler = Update::filter_message()
            // Handle commands sent by a general user.
            .branch(
                dptree::entry()
                    .filter_command::<UserCommands>()
                    .endpoint(Self::handle_user_command),
            )
            // Handle commands sent by an admin.
            .branch(
                dptree::filter(|| true)
                    .filter_command::<AdminCommands>()
                    .endpoint(|| async { Ok(()) }),
            )
            // Handle commands sent by a superuser.
            .branch(
                dptree::filter(|| true)
                    .filter_command::<SuperuserCommands>()
                    .endpoint(|| async { Ok(()) }),
            );

        let channel_handler = Update::filter_channel_post().branch(
            // Filtering allow you to filter updates by some condition.
            dptree::filter(|| true)
                // An endpoint is the last update handler.
                .endpoint(|bot: Bot, msg: Message| async move {
                    bot.send_dice(msg.chat.id).await?;

                    Ok(())
                }),
        ); // end Update::filter_message

        // Main handler that joins all handlers into one.
        let handler = dptree::entry().branch(chat_handler).branch(channel_handler);

        let mut bot_chat_service_job = Dispatcher::builder(bot, handler)
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
            bot_chat_service_job.dispatch().await;
        });

        listener
    } // end fn set_listener

    /// This function is a handler for bot commands.
    async fn handle_user_command(
        bot_state: BotState,
        bot: Bot,
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

        // Check if the user is a channel or a bot,
        // then do not react to the message.
        if user.is_bot || user.is_channel() || user.is_anonymous() {
            return Ok(());
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

            return Ok(());
        } // end if

        // Otherwise safely unwrap the connection.
        let conn = &mut conn.unwrap();

        // Try retrieve the user from the database.
        let registered_user: Option<User> = match User::get_from_db_by_id(user.id.0 as i64, conn) {
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

                    return Ok(());
                } // end if
            } // end Err
        }; // end match

        // Check if the user is registered.
        if let Some(registered_user) = &registered_user {
            // Check if the user is allowed to use the bot.
            match StatusId::from(registered_user.status_id) {
                // The user is in pending mode.
                StatusId::Pending => {
                    // Inform a user that they are in pending mode.
                    bot.send_message(msg.chat.id, "You are not approved for using this bot yet.\nAn admin should review your application soon.")
                    .parse_mode(ParseMode::Html)
                    .await?;

                    return Ok(());
                } // end StatusId::Pending
                // The user's request was rejected.
                StatusId::Rejected => {
                    // Inform a user that they are rejected.
                    bot.send_message(
                        msg.chat.id,
                        "We are sorry to inform you that your request was rejected.",
                    )
                    .parse_mode(ParseMode::Html)
                    .await?;

                    return Ok(());
                } // end StatusId::Rejected
                // The user is approved and can use the bot.
                StatusId::Approved => {}
                // The user is banned.
                StatusId::Banned => {
                    // Inform a user that they are banned.
                    bot.send_message(
                        msg.chat.id,
                        "We are sorry to inform you that you were banned.",
                    )
                    .parse_mode(ParseMode::Html)
                    .await?;

                    return Ok(());
                } // end StatusId::Banned
            } // end match
        } // end if

        // Handle a specific command.
        match cmd {
            UserCommands::Start => {
                // Check if the user is already registered.
                if registered_user.is_some() {
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
                let _registered_user = match Self::get_user_and_verify_role(
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

                bot.send_message(msg.chat.id, UserCommands::descriptions().to_string())
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
                        registered_user.exchange_api_key.unwrap_or_default()
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
                let deposit = if registered_user.deposit.is_none() {
                    String::from("None")
                } else {
                    String::from("$") + registered_user.deposit.unwrap().to_string().as_str()
                }; // end if

                let risk = if registered_user.risk.is_none() {
                    String::from("None")
                } else {
                    registered_user.risk.unwrap().to_string() + "%"
                }; // end if

                // Generate a message with current user settings.
                let cur_settings_message = format!(
                    "<b>⚙️Settings⚙️</b>\n\n<b>💰 Deposit:</b> {}\n<b>❗️ Risk:</b> {}\n\n<b>🔑 API Exchange key:</b> <em>Unsafe to show</em>\n<b>㊙️ API Exchange secret:</b> <em>Unsafe to show</em>\n\n<b>🔥 Bot allowed to trade:</b> {}",
                        deposit, risk, registered_user.bot_active
                );

                // Send the current settings to the user.
                bot.send_message(msg.chat.id, cur_settings_message)
                    .parse_mode(ParseMode::Html)
                    .await?
            } // end Command::GetSettings
        }; // end match

        Ok(())
    } // end fn exec_command

    /// This function verifies that the user is registered.
    /// Then function verifies the user role.
    /// Returns None if the user did not pass verification.
    /// Returns User otherwise.
    async fn get_user_and_verify_role(
        bot: &Bot,
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

    /// This function reads incoming messages.
    async fn read_message(bot: Bot, msg: Message) -> ResponseResult<()> {
        println!("Got message!");
        bot.send_dice(msg.chat.id).await?;
        Ok(())
    } // end fn read_message
} // end impl BotService
