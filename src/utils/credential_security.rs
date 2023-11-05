// This file contains functions that secure vulnerable data.

use std::{env, ffi::OsStr};

use base64::Engine;
use dotenv::dotenv;

use super::app_error::AppError;

/// This struct contains all the functionality to provide
/// some security level to the credentials.
pub struct CredentialSecurity;

/// Implement all the required functionality for the struct.
impl CredentialSecurity {
    /// This function decodes environmental variables
    /// that are stored in base64 encoded format.
    pub fn decode_env_var(name: impl AsRef<OsStr>) -> Result<String, AppError> {
        // Load .env file.
        dotenv().ok();

        // Get the variable itself.
        let env_var = env::var(name).map_err(|error| AppError::new(error.to_string()))?;

        // Decode the variable.
        let env_var = base64::engine::general_purpose::STANDARD
            .decode(env_var)
            .map_err(|error| AppError::new(error.to_string()))?;

        // Convert the variable to string.
        let env_var =
            String::from_utf8(env_var).map_err(|error| AppError::new(error.to_string()))?;

        Ok(env_var)
    } // end encode_env_var

    /// This function simply gets non-encoded environmental variables
    pub fn env_var(name: impl AsRef<OsStr>) -> Result<String, AppError> {
        // Load .env file.
        dotenv().ok();

        // Get the variable itself.
        let env_var = env::var(name).map_err(|error| AppError::new(error.to_string()))?;

        Ok(env_var)
    } // end fn env_var
} // end impl CredentialSecurity
