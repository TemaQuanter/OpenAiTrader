// This file contains all the necessary services to work with
// ChatGPT.

use std::{fs, io::Read};

use chatgpt::prelude::{ChatGPT, Conversation};

use super::types_and_constants::{Result, CHAT_GPT_SEED_PROMPT_FILE};

/// This structure contains all the functionality to work
/// with ChatGPT.
pub struct ChatGPTService;

/// Implement all the necessary functionality for
/// working with ChatGPT.
impl ChatGPTService {
    /// This function creates a new conversation with ChatGPT.
    pub async fn get_conversation(chatgpt_client: &ChatGPT) -> Result<Conversation> {
        // Load a file with a seed prompt.
        let mut prompt_file = fs::File::open(CHAT_GPT_SEED_PROMPT_FILE)?;

        // Load a starting prompt from a file.
        let mut seed_prompt = String::new();
        prompt_file.read_to_string(&mut seed_prompt)?;

        // Create a new conversation with a seed prompt.
        let conversation = chatgpt_client.new_conversation_directed(seed_prompt);

        Ok(conversation)
    } // end fn get_conversation
} // end impl ChatGPTService
