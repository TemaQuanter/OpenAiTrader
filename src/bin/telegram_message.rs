use teloxide::{
    adaptors::throttle::Limits,
    dispatching::{Dispatcher, UpdateFilterExt},
    dptree,
    requests::{RequesterExt, ResponseResult},
    types::{Message, Update},
    Bot,
};
use trading_bot::utils::credential_security::CredentialSecurity;

#[tokio::main]
async fn main() {
    // Get necessary credentials for telegram bot.
    let tg_bot_api: String = CredentialSecurity::decode_env_var("TG_BOT_API").unwrap();

    // Initialize a telegram bot.
    let bot = Bot::new(&tg_bot_api).throttle(Limits::default());

    // A handler that determines the behavior of the bot in regards to
    // channel posts.
    let channel_handler = Update::filter_channel_post().branch(
        // Allow posts only from a trusted channel.
        dptree::filter(move |msg: Message| {
            println!(
                "ChatID: {}\nChatName: {:#?}",
                msg.chat.id,
                format!("{}", msg.chat.title().unwrap())
            );

            true
        })
        .endpoint(empty_func),
    ); // end Update::filter_message

    // Main handler that joins all handlers into one.
    let handler = dptree::entry().branch(channel_handler);

    Dispatcher::builder(bot, handler)
        .enable_ctrlc_handler()
        .build()
        .dispatch()
        .await;
} // end async main

async fn empty_func() -> ResponseResult<()> {
    Ok(())
}
