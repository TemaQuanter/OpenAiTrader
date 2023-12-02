use chrono::Utc;
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
        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
        println!("{}", Utc::now() - msg.date);
        println!("{}", Utc::now() - msg.date > chrono::Duration::seconds(5));
        bot.send_dice(msg.chat.id).await?;
        Ok(())
    })
    .await;
} // end async main
