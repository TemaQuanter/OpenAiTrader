mod db_objects;
mod handlers;
mod schema;
mod utils;

use std::str::FromStr;
use std::sync::Arc;

use chatgpt::prelude::{ChatGPT, ChatGPTEngine, ModelConfigurationBuilder, Url};
use flexi_logger::{Age, Cleanup, Criterion, FileSpec, Logger, Naming, WriteMode};
use teloxide::adaptors::throttle::Limits;
use teloxide::requests::RequesterExt;
use teloxide::Bot;
use tokio::sync::Mutex;
use trading_bot::utils::app_error::AppError;
use utils::chat_gpt_service::ChatGPTService;
use utils::credential_security::CredentialSecurity;

use crate::handlers::bot::bot_service::{BotService, BotState};
use crate::handlers::bot::message_service::MessageService;
use crate::handlers::server::run::run_server;
use crate::utils::database_service::DatabaseService;
use crate::utils::lazy_static::CHAT_GPT_MODEL;

#[tokio::main]
async fn main() -> Result<(), AppError> {
    // Set up logger.
    let _logger = Logger::try_with_str("info")
        .map_err(|error| AppError::new(error.to_string()))?
        .log_to_file(FileSpec::default().directory("./data/logs").basename("log"))
        .write_mode(WriteMode::BufferAndFlush)
        .rotate(
            Criterion::Age(Age::Day),
            Naming::Timestamps,
            Cleanup::KeepLogFiles(10),
        )
        .append()
        .start()
        .map_err(|error| AppError::new(error.to_string()))?;

    log::info!("Starting...");

    // Get necessary credentials for telegram bot.
    let tg_bot_api: String = CredentialSecurity::decode_env_var("TG_BOT_API").unwrap();

    // A trusted channel that we want to read the messages from.
    let trusted_channel_id: i64 = CredentialSecurity::decode_env_var("TG_TRUSTED_CHANNEL_ID")
        .unwrap()
        .parse::<i64>()
        .unwrap();

    // A log channel that bot uses to send logs to.
    let log_channel_id: i64 = CredentialSecurity::decode_env_var("TG_LOG_CHANNEL_ID")
        .unwrap()
        .parse::<i64>()
        .unwrap();

    // Get all the necessary credentials for ChatGPT.
    let chat_gpt_api: String = CredentialSecurity::decode_env_var("CHAT_GPT_API").unwrap();
    let chat_gpt_url: String = CredentialSecurity::env_var("CHAT_GPT_URL").unwrap();

    // Create a database connection pool.
    let pg_connection_pool = DatabaseService::create_database_connection_pool();

    log::info!("Environmental variables loaded");

    // Initialize a telegram bot.
    let bot = Bot::new(&tg_bot_api).throttle(Limits::default());

    // Initialize ChatGPT client.
    let chatgpt_client = ChatGPT::new_with_config(
        chat_gpt_api,
        ModelConfigurationBuilder::default()
            .api_url(Url::from_str(&chat_gpt_url).unwrap())
            .temperature(0.0)
            .engine(ChatGPTEngine::Custom(*CHAT_GPT_MODEL))
            .build()
            .unwrap(),
    )
    .unwrap();

    log::info!("Got ChatGPT client");

    // Initialize or load a conversation with ChatGPT.
    let chatgpt_conversation = ChatGPTService::get_conversation(&chatgpt_client)
        .await
        .unwrap();

    // Assemble a bot state.
    let bot_state = BotState {
        pg_connection_pool: Arc::new(pg_connection_pool),
        chatgpt_conversation: Arc::new(Mutex::new(chatgpt_conversation)),
        trusted_channel_id,
        log_channel_id,
    };

    // Cloned connection pool for server.
    let cloned_pool_for_server = (*bot_state.pg_connection_pool).clone();

    // Bot instance for a server.
    let cloned_bot_for_server = bot.clone();

    // Set up bot listener (start listening for incoming messages)
    let bot_service = BotService::set_listener(bot.clone(), bot_state.clone()).await;

    // Determine the age of an "old" post.
    let old_age = chrono::Duration::days(90);

    // Determine the frequency to remove "old" post with (in seconds).
    let wait_time_secs: u64 = 24 * 60 * 60;

    log::info!("Got ChatGPT conversation");
    log::info!("Start listening for incoming messages");

    // Run the program till it is cancelled
    tokio::select! {
        _ = async move {
            // Run a separate task with a bot service.
            bot_service.await.unwrap();
        } => {}
        _ = async move {
            // Run a separate task to remove "old" posts from the database.
            MessageService::remove_old_messages(bot_state.clone(), old_age, wait_time_secs, bot.clone()).await;
        } => {}
        _ = async move {
            // Run a server.
            run_server(cloned_pool_for_server, cloned_bot_for_server).await;
        } => {}

        // Handle Ctrl + C
        _ = tokio::signal::ctrl_c() => {}
    } // end tokio::select!

    Ok(())
} // end main
