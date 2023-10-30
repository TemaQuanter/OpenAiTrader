mod handlers;
mod utils;

use std::str::FromStr;

use chatgpt::prelude::{ChatGPT, ChatGPTEngine, ModelConfigurationBuilder, Url};
use flexi_logger::{Age, Cleanup, Criterion, FileSpec, Logger, Naming, WriteMode};
use utils::chat_gpt_service::ChatGPTService;
use utils::credential_security::CredentialSecurity;

use crate::handlers::client::message_handler::ClientService;
use crate::handlers::{bot::bot_service::BotService, exchange::exchange_service::ExchangeService};
use crate::utils::{sign_in_tg::SignInTg, types_and_constants::Result};

#[tokio::main]
async fn main() -> Result<()> {
    // Set up logger.
    let _logger = Logger::try_with_str("info")?
        .log_to_file(FileSpec::default().directory("./data/logs").basename("log"))
        .write_mode(WriteMode::BufferAndFlush)
        .rotate(
            Criterion::Age(Age::Day),
            Naming::Timestamps,
            Cleanup::KeepLogFiles(10),
        )
        .append()
        .start()?;

    log::info!("Starting...");

    // Get all the necessary credentials for using Telegram.
    let api_id: i32 = CredentialSecurity::decode_env_var("TG_API_ID")
        .unwrap()
        .parse::<i32>()
        .unwrap();
    let api_hash: String = CredentialSecurity::decode_env_var("TG_API_HASH").unwrap();
    let phone: String = CredentialSecurity::decode_env_var("TG_API_PHONE").unwrap();
    let password: String = CredentialSecurity::decode_env_var("TG_API_PASSWORD").unwrap();

    // Get necessary credentials for telegram bot.
    let tg_bot_api: String = CredentialSecurity::decode_env_var("TG_BOT_API").unwrap();

    // Get crypto exchange credentials.
    let exchange_api: String = CredentialSecurity::decode_env_var("EXCHANGE_API").unwrap();
    let exchange_secret: String = CredentialSecurity::decode_env_var("EXCHANGE_SECRET").unwrap();

    // A trusted channel that we want to read the messages from.
    let trusted_channel_id: i64 = CredentialSecurity::decode_env_var("TG_TRUSTED_CHANNEL_ID")
        .unwrap()
        .parse::<i64>()
        .unwrap();

    // A trusted user that is allowed to use the bot.
    let trusted_user_id: i64 = CredentialSecurity::decode_env_var("TG_TRUSTED_USER_ID")
        .unwrap()
        .parse::<i64>()
        .unwrap();

    // Get all the necessary credentials for ChatGPT.
    let chat_gpt_api: String = CredentialSecurity::decode_env_var("CHAT_GPT_API").unwrap();
    let chat_gpt_url: String = CredentialSecurity::env_var("CHAT_GPT_URL").unwrap();

    log::info!("Environmental variables loaded");

    // Initialize crypto exchange client.
    let exchange_client = ExchangeService::get_client(exchange_api, exchange_secret)?;

    // Initialize a telegram bot.
    let bot = BotService::new_bot(&tg_bot_api).unwrap();

    // Initialize a channel to communicate between concurrent tasks.
    let (tx_telegram_code, rx_telegram_code) = tokio::sync::mpsc::channel::<String>(100);
    let (tx_exchange_monitoring, rx_exchange_monitoring) =
        tokio::sync::mpsc::channel::<String>(100);

    // Set up bot listener (start listening for incoming commands)
    let bot_command_handler =
        BotService::set_listener(&bot, trusted_user_id, tx_telegram_code).await;

    // Initialize a telegram client.
    let tg_client = SignInTg::get_client(api_id, &api_hash)
        .await
        .expect("Failed to get a telegram client");

    log::info!("Got a telegram client");

    // Initialize a telegram user.
    let _user = SignInTg::get_user_from_client(
        &bot,
        trusted_user_id,
        &tg_client,
        &phone,
        api_id,
        &api_hash,
        &password,
        rx_telegram_code,
    )
    .await
    .unwrap();

    log::info!("Got a telegram user");

    // Initialize ChatGPT client.
    let chatgpt_client = ChatGPT::new_with_config(
        chat_gpt_api,
        ModelConfigurationBuilder::default()
            .api_url(Url::from_str(&chat_gpt_url).unwrap())
            .temperature(0.5)
            .engine(ChatGPTEngine::Gpt4)
            .build()
            .unwrap(),
    )
    .unwrap();

    log::info!("Got ChatGPT client");

    // Initialize or load a conversation with ChatGPT.
    let chatgpt_conversation = ChatGPTService::get_conversation(&chatgpt_client)
        .await
        .unwrap();

    log::info!("Got ChatGPT conversation");
    log::info!("Start listening for incoming messages");

    // Start handling messages from the trusted channel.
    let client_listener = ClientService::listen_for_incoming_messages(
        tg_client,
        chatgpt_conversation,
        trusted_channel_id,
        &bot,
        trusted_user_id,
        &exchange_client,
        tx_exchange_monitoring,
    ); // end client_listener

    // Run the program till it is cancelled
    tokio::select! {
        _ = async move {
            // Run a separate task with bot command listener.
            bot_command_handler.await.unwrap();
        } => {},
        _ = async move {
            // Run a separate task with Telegram client.
            client_listener.await.unwrap();
        } => {},
        _ = async {
            // Run a monitoring of the legacy orders on
            // the exchange.
            match ExchangeService::monitor_and_clear_legacy_conditional_orders(&exchange_client, rx_exchange_monitoring).await {
                Ok(_) => (),
                // Ignore any occurring errors.
                Err(_) => ()
            }; // end match
        } => {}
        _ = tokio::signal::ctrl_c() => {}
    } // end tokio::select!

    Ok(())
} // end main
