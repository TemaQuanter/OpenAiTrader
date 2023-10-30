// This program connects to a telegram client and retrieves
// all chat names and IDs.

use dotenv::dotenv;
use grammers_client::SignInError;
use trading_bot::utils::{
    credential_security::CredentialSecurity, sign_in_tg::SignInTg,
    types_and_constants::SESSION_FILE,
};

#[tokio::main]
async fn main() {
    // Load the environmental variables.
    dotenv().ok();

    // Get all the necessary credentials for using Telegram.
    let api_id: i32 = CredentialSecurity::decode_env_var("TG_API_ID")
        .unwrap()
        .parse::<i32>()
        .unwrap();
    let api_hash: String = CredentialSecurity::decode_env_var("TG_API_HASH").unwrap();
    let phone: String = CredentialSecurity::decode_env_var("TG_API_PHONE").unwrap();
    let password: String = CredentialSecurity::decode_env_var("TG_API_PASSWORD").unwrap();

    // Initialize a telegram client.
    let tg_client = SignInTg::get_client(api_id, &api_hash)
        .await
        .expect("Failed to get a telegram client");

    let token = tg_client
        .request_login_code(&phone, api_id, &api_hash)
        .await
        .unwrap();

    let code = get_code();

    // Initialize a telegram user.
    let _user = match tg_client.sign_in(&token, &code).await {
        Err(SignInError::PasswordRequired(password_token)) => tg_client
            .check_password(password_token, password.as_bytes())
            .await
            .unwrap(), // end Err
        Ok(user) => user,
        Err(err) => {
            panic!("Failed to sign in as a user :(\n{err}");
        } // end Err
    }; // end match

    tg_client.session().save_to_file(SESSION_FILE).unwrap();

    // Traverse all the chats when there is a new message in them and display their names and IDs.
    // while let Some(update) = match tg_client.next_update().await {
    //     Ok(update) => update,
    //     Err(_error) => panic!(),
    // } {
    //     if let Update::NewMessage(msg) = update {
    //         println!();
    //         println!("ChatName: {}", msg.chat().name());
    //         println!("ChatId: {}", msg.chat().id());
    //         println!("ChatLastMessage: {}", msg.text());
    //         println!();
    //     } // end if
    // } // end for

    // Get all messages from all chats that pass the filter.
    let mut messages = tg_client.search_all_messages().query("Bybit. FLMUSDT.");

    // Traverse all the messages found by a filter and display some
    // information about them.
    while let Some(message) = messages.next().await.unwrap() {
        println!("ChatName: {}", message.chat().name());
        println!("ChatId: {}", message.chat().id());
        println!("ChatMessage: {}", message.text());
    } // end for
} // end main

/// This function prompts a user for a verification code.
fn get_code() -> String {
    println!("Enter verification code:");

    let mut code = String::new();
    std::io::stdin().read_line(&mut code).unwrap();

    code
} // end fn get_code
