// This file contains some custom types.

use std::error::Error;

// Universal result type.
pub type Result<T> = std::result::Result<T, Box<dyn Error>>;

// This is a session file with an authentication token for Telegram client.
pub const SESSION_FILE: &str = "data/tg.session";

// This is a conversation file that stores the history
// for ChatGPT.
pub const CHAT_GPT_HISTORY_FILE: &str = "data/chat_gpt_history.json";

// The max length of ChatGPT history.
// Currently out of use.
pub const _CHAT_GPT_HISTORY_LENGTH: usize = 200;

// The location of a file with seed prompt for ChatGPT
pub const CHAT_GPT_SEED_PROMPT_FILE: &str = "data/chatgpt_seed_prompt.txt";

// The location of the file with settings.
pub const APP_SETTINGS_FILE: &str = "data/settings.json";
