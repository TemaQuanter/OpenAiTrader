// This file contains constants that must be calculated once, but dynamically.

use lazy_static::lazy_static;

use crate::utils::credential_security::CredentialSecurity;

lazy_static! {
    // Url of subscription bot.
    pub static ref SUBSCRIPTION_BOT_URL: String =
        CredentialSecurity::env_var("SUBSCRIPTION_BOT_URL")
            .expect("SUBSCRIPTION_BOT_URL not found in .env");

    // The custom trained model of ChatGPT for trading on perpetual swaps.
    pub static ref CHAT_GPT_MODEL_PERPETUAL: &'static str = Box::leak(CredentialSecurity::env_var("CHAT_GPT_MODEL_PERPETUAL")
        .expect("CHAT_GPT_MODEL not found in .env").into_boxed_str());

    // The custom trained model of ChatGPT for trading on spot.
    pub static ref CHAT_GPT_MODEL_SPOT: &'static str = Box::leak(CredentialSecurity::env_var("CHAT_GPT_MODEL_SPOT")
        .expect("CHAT_GPT_MODEL not found in .env").into_boxed_str());
} // end lazy_static
