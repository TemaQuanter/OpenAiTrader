// This file contains constants that must be calculated once, but dynamically.

use lazy_static::lazy_static;

use crate::utils::credential_security::CredentialSecurity;

lazy_static! {
    // Url of subscription bot.
    pub static ref SUBSCRIPTION_BOT_URL: String =
        CredentialSecurity::env_var("SUBSCRIPTION_BOT_URL")
            .expect("SUBSCRIPTION_BOT_URL not found in .env");

    // Words or parts of words that might indicate that a message is not a serious
    // signal, therefore it might be necessary to ignore it.
    pub static ref JOKE_WORDS: Vec<String> = {
        // Retrieve the words and parts of words that might indicate a message
        // being a joke from environmental variables.
        let raw_joke_words = CredentialSecurity::env_var("JOKE_WORDS").expect("JOKE_WORDS environmental variable was not found");

        // A return array with all joke words.
        let mut joke_words: Vec<String> = Vec::new();

        // Split the words into separate words by semicolons.
        for word in raw_joke_words.split(';') {
            joke_words.push(word.to_string());
        } // end for

        joke_words
    }; // end static ref JOKE_WORDS

    // Words or parts of words that might indicate that a message is a potential
    // spot trade signal.
    pub static ref SPOT_TRADE_WORDS: Vec<String> = {
        // Retrieve the words and parts of words that might indicate a message
        // being a spot trade signal from environmental variables.
        let raw_spot_trade_words = CredentialSecurity::env_var("SPOT_TRADE_WORDS").expect("SPOT_TRADE_WORDS environmental variable was not found");

        // A return array with all call to action words.
        let mut spot_trade_words: Vec<String> = Vec::new();

        // Split the words into separate words by semicolons.
        for word in raw_spot_trade_words.split(';') {
            spot_trade_words.push(word.to_string());
        } // end for

        spot_trade_words
    }; // end static ref SPOT_TRADE_WORDS

    // The custom trained model of ChatGPT for trading on perpetual swaps.
    pub static ref CHAT_GPT_MODEL_PERPETUAL: &'static str = Box::leak(CredentialSecurity::env_var("CHAT_GPT_MODEL_PERPETUAL")
        .expect("CHAT_GPT_MODEL not found in .env").into_boxed_str());

    // The custom trained model of ChatGPT for trading on spot.
    pub static ref CHAT_GPT_MODEL_SPOT: &'static str = Box::leak(CredentialSecurity::env_var("CHAT_GPT_MODEL_SPOT")
        .expect("CHAT_GPT_MODEL not found in .env").into_boxed_str());

    // Delay between unsuccessful requests to the exchange.
    pub static ref DELAY_BETWEEN_UNSUCCESSFUL_REQUESTS: tokio::time::Duration = tokio::time::Duration::from_millis(300);
} // end lazy_static
