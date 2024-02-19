use rust_decimal::Decimal;
use rust_decimal_macros::dec;

// The max length of ChatGPT history.
// Currently out of use.
pub const _CHAT_GPT_HISTORY_LENGTH: usize = 200;

// The location of a file with seed prompt for ChatGPT that is responsible for perpetual swaps.
pub const CHAT_GPT_SEED_PROMPT_PERPETUAL_FILE: &str = "data/chatgpt_seed_prompt_perpetual.txt";

// The location of a file with seed prompt for ChatGPT that is responsible for spot.
pub const CHAT_GPT_SEED_PROMPT_SPOT_FILE: &str = "data/chatgpt_seed_prompt_spot.txt";

// Error message for telegram users.
pub const ERROR_MESSAGE: &str = "Something went wrong with me, please, try again later";

// A slippage tolerance when adjusting stop loss on.
pub const SLIPPAGE_PERCENT: Decimal = dec!(0.1);

// Spot opened position bias tolerance.
pub const SPOT_OPENED_POSITION_BIAS_VALUE: Decimal = dec!(1.0);

// Server address and port.
pub const SERVER_ADDRESS: &str = "0.0.0.0:8080";

// Path to the bot manual.
pub const PATH_TO_BOT_MANUAL: &str = "data/commands_description.json";

// Number attempts to send a request to the exchange.
pub const NUMBER_ATTEMPTS_SEND_REQUEST: i32 = 3;
