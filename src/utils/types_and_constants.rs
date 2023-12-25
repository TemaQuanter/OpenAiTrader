use rust_decimal::Decimal;
use rust_decimal_macros::dec;

// The max length of ChatGPT history.
// Currently out of use.
pub const _CHAT_GPT_HISTORY_LENGTH: usize = 200;

// The location of a file with seed prompt for ChatGPT
pub const CHAT_GPT_SEED_PROMPT_FILE: &str = "data/chatgpt_seed_prompt.txt";

// Error message for telegram users.
pub const ERROR_MESSAGE: &str = "Something went wrong with me, please, try again later";

// A slippage tolerance when adjusting stop loss on.
pub const SLIPPAGE_PERCENT: Decimal = dec!(0.1);

// Server address and port.
pub const SERVER_ADDRESS: &str = "0.0.0.0:8080";
