// This file contains all the necessary services to work with
// ChatGPT.

use std::{fs, io::Read};

use chatgpt::prelude::{ChatGPT, Conversation};

use super::types_and_constants::{Result, CHAT_GPT_HISTORY_FILE, CHAT_GPT_SEED_PROMPT_FILE};

/// This structure contains all the functionality to work
/// with ChatGPT.
pub struct ChatGPTService;

/// Implement all the necessary functionality for
/// working with ChatGPT.
impl ChatGPTService {
    /// This function loads a conversation or creates a new one
    /// if there is no conversation found.
    pub async fn get_conversation(chatgpt_client: &ChatGPT) -> Result<Conversation> {
        // Initialize a variable with conversation.
        let conversation = match chatgpt_client
            .restore_conversation_json(CHAT_GPT_HISTORY_FILE)
            .await
        {
            // A conversation has been loaded successfully.
            Ok(conversation) => conversation,
            Err(_error) => {
                // There was a problem with loading a conversation.
                // Create a new one.

                // Load a file with a seed prompt.
                let mut prompt_file = fs::File::open(CHAT_GPT_SEED_PROMPT_FILE)?;

                // Load a starting prompt from a file.
                let mut seed_prompt = String::new();
                prompt_file.read_to_string(&mut seed_prompt)?;

                // Create a new conversation with a seed prompt.
                chatgpt_client.new_conversation_directed(seed_prompt)
            } // end Err
        }; // end match

        Ok(conversation)
    } // end fn get_conversation

    /// This function saves a conversation with ChatGPT.
    /// ATTENTION: This function is currently out of use,
    /// as it might be really expensive to use ChatGPT history.
    pub async fn _save_conversation(conversation: Conversation) -> Result<()> {
        conversation
            .save_history_json(CHAT_GPT_HISTORY_FILE)
            .await?;

        Ok(())
    } // end fn save_conversation
} // end impl ChatGPTService
