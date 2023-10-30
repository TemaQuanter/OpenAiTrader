// This file contains all the functionality necessary to interact
// with a user via Telegram bot.

use rust_decimal::Decimal;
use tokio::sync::mpsc::Sender;

use teloxide::{
    payloads::SendMessageSetters,
    repls::CommandReplExt,
    requests::{Requester, ResponseResult},
    types::{ChatId, Message, ParseMode},
    utils::command::BotCommands,
    Bot,
};
use tokio::task::JoinHandle;

use crate::utils::{app_settings::AppSettingsService, types_and_constants::Result};

/// These are all commands that the bot can handle.
#[derive(BotCommands, Clone)]
#[command(
    rename_rule = "lowercase",
    description = "These commands are supported:"
)]
enum Command {
    #[command(description = "start interaction with the bot.")]
    Start,
    #[command(description = "display this text.")]
    Help,
    #[command(description = "pass a verification code for telegram client.")]
    Code(String),
    #[command(description = "set a new deposit amount (for risk calculation)")]
    SetDeposit(Decimal),
    #[command(description = "set a new risk per trade in percentage")]
    SetRisk(Decimal),
    #[command(description = "set whether or not the bot is allowed to trade")]
    SetIsActive(bool),
    #[command(description = "get the application settings")]
    GetSettings,
} // end enum Command

/// This structure contains all the functionality
/// necessary to drive a Telegram bot.
pub struct BotService;

/// Implement all the functionality necessary to drive a Telegram
/// bot.
impl BotService {
    /// This function initializes a telegram bot.
    pub fn new_bot(tg_bot_api: &String) -> Result<Bot> {
        let bot = Bot::new(tg_bot_api);

        Ok(bot)
    } // end fn get_client

    /// This function prompts a user for information.
    pub async fn inform_client(bot: &Bot, chat_id: i64, message: impl AsRef<str>) -> Result<()> {
        // Shadow chat ID for convenience.
        let chat_id = ChatId(chat_id);

        // Prompt a client for something.
        bot.send_message(chat_id, message.as_ref())
            .parse_mode(ParseMode::Html)
            .await?;

        Ok(())
    } // end fn inform_client

    /// This function sets up a listener for bot commands.
    pub async fn set_listener(
        bot: &Bot,
        trusted_user_id: i64,
        tx_telegram_code: Sender<String>,
    ) -> JoinHandle<()> {
        // Specify the async job to do.
        let async_job = Command::repl(bot.clone(), move |bot: Bot, msg: Message, cmd: Command| {
            Self::answer(bot, msg, cmd, trusted_user_id, tx_telegram_code.clone())
        });

        // Remember the reference to the async job.
        let listener = tokio::spawn(async {
            async_job.await;
        });

        listener
    } // end fn set_listener

    /// This function is a handler for bot commands.
    async fn answer(
        bot: Bot,
        msg: Message,
        cmd: Command,
        trusted_user_id: i64,
        tx_telegram_code: Sender<String>,
    ) -> ResponseResult<()> {
        // Check if it is a trusted user.
        if msg.chat.id != ChatId(trusted_user_id) {
            // This user has no rights to interact with the bot.
            bot.send_message(
                msg.chat.id,
                "Sorry, but you do not have a permission to interact with me",
            )
            .await?;

            return Ok(());
        } // end if

        // Handle a specific command.
        match cmd {
            Command::Start => {
                bot.send_message(msg.chat.id, "Welcome to <b>OPEN-AI-TRADER</b> bot!")
                    .parse_mode(ParseMode::Html)
                    .await?
            } // end Command::Start
            Command::Help => {
                bot.send_message(msg.chat.id, Command::descriptions().to_string())
                    .parse_mode(ParseMode::Html)
                    .await?
            } // end Command::Help
            Command::Code(code) => {
                // Send the verification code to the client authentication service.
                tx_telegram_code.send(code).await.unwrap();

                bot.send_message(msg.chat.id, "Got the code!\nTrying to log in...")
                    .parse_mode(ParseMode::Html)
                    .await?
            } // end Command::Code
            Command::SetDeposit(amount) => {
                // Update the deposit to calculate risk from.

                // Load current settings.
                let mut app_settings = AppSettingsService::load_settings().unwrap();

                // Update the deposit.
                app_settings.deposit = amount;

                // Save new settings.
                AppSettingsService::save_settings(app_settings).unwrap();

                bot.send_message(msg.chat.id, "💰Deposit amount is updated!💰")
                    .parse_mode(ParseMode::Html)
                    .await?
            } // end Command::SetDeposit
            Command::SetRisk(risk) => {
                // Update the risk per trade.

                // Load current settings.
                let mut app_settings = AppSettingsService::load_settings().unwrap();

                // Update the deposit.
                app_settings.risk = risk;

                // Save new settings.
                AppSettingsService::save_settings(app_settings).unwrap();

                bot.send_message(msg.chat.id, "❗️Risk is updated❗️")
                    .parse_mode(ParseMode::Html)
                    .await?
            } // end Command::SetRisk
            Command::SetIsActive(val) => {
                // Update the activity status of the bot.

                // Load current settings.
                let mut app_settings = AppSettingsService::load_settings().unwrap();

                // Update the deposit.
                app_settings.is_active = val;

                // Save new settings.
                AppSettingsService::save_settings(app_settings).unwrap();

                bot.send_message(msg.chat.id, "🔥Bot activity is changed🔥")
                    .parse_mode(ParseMode::Html)
                    .await?
            } // end Command::SetIsActive
            Command::GetSettings => {
                // Get the settings of the application.

                // Load the application settings.
                let application_settings = AppSettingsService::load_settings().unwrap();

                // Send the current settings to the user.
                bot.send_message(
                    msg.chat.id,
                    format!(
                        "⚙️ <b>Settings</b> ⚙️\n\n\
                <b>💰 Deposit:</b> {}\n\
                <b>❗️ Risk:</b> {}\n\
                <b>🔥 Bot active:</b> {}",
                        application_settings.deposit,
                        application_settings.risk,
                        application_settings.is_active
                    ),
                )
                .parse_mode(ParseMode::Html)
                .await?
            } // end Command::GetSettings
        }; // end match

        Ok(())
    } // end impl BotService
} // end fn set_listener
