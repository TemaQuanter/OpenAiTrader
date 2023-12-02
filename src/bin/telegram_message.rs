use teloxide::{
    adaptors::{throttle::Limits, Throttle},
    requests::{Requester, RequesterExt},
    types::Message,
    Bot,
};
use trading_bot::utils::credential_security::CredentialSecurity;

#[tokio::main]
async fn main() {
    // Get necessary credentials for telegram bot.
    let tg_bot_api: String = CredentialSecurity::decode_env_var("TG_BOT_API").unwrap();

    // Initialize a telegram bot.
    let bot = Bot::new(&tg_bot_api).throttle(Limits::default());

    teloxide::repl(bot, |bot: Throttle<Bot>, msg: Message| async move {
        println!("{}", msg.date);
        bot.send_dice(msg.chat.id).await?;
        Ok(())
    })
    .await;
} // end async main
