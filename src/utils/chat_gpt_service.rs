// This file contains all the necessary services to work with
// ChatGPT.

use std::{fs, io::Read};

use chatgpt::{
    prelude::{ChatGPT, Conversation},
    types::ChatMessage,
};

use super::app_error::AppError;

/// This structure contains all the functionality to work
/// with ChatGPT.
pub struct ChatGPTService;

/// Implement all the necessary functionality for
/// working with ChatGPT.
impl ChatGPTService {
    /// This function creates a new conversation with ChatGPT.
    pub async fn get_conversation(
        chatgpt_client: &ChatGPT,
        seed_prompt_file: &str,
    ) -> Result<Conversation, AppError> {
        // Load a file with a seed prompt.
        let mut prompt_file =
            fs::File::open(seed_prompt_file).map_err(|error| AppError::new(error.to_string()))?;

        // Load a starting prompt from a file.
        let mut seed_prompt = String::new();
        prompt_file
            .read_to_string(&mut seed_prompt)
            .map_err(|error| AppError::new(error.to_string()))?;

        // Create a new conversation with a seed prompt.
        let mut conversation = chatgpt_client.new_conversation_directed(seed_prompt);

        // Append ChatGPT reply on a system message.
        conversation.history.push(ChatMessage {
            role: chatgpt::types::Role::Assistant,
            content: "OK".to_string(),
        });

        Ok(conversation)
    } // end fn get_conversation
} // end impl ChatGPTService
