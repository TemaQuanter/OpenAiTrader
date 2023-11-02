// This file contains some custom types.

use std::error::Error;

// Universal result type.
pub type Result<T> = std::result::Result<T, Box<dyn Error>>;

// The max length of ChatGPT history.
// Currently out of use.
pub const _CHAT_GPT_HISTORY_LENGTH: usize = 200;

// The location of a file with seed prompt for ChatGPT
pub const CHAT_GPT_SEED_PROMPT_FILE: &str = "data/chatgpt_seed_prompt.txt";
