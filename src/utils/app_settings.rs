// This file contains the functionality and utils to
// manipulate settings of the application.

use std::{
    fs,
    io::{Read, Write},
};

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

use super::types_and_constants::{Result, APP_SETTINGS_FILE};

/// This structure contains all the settings that it is possible
/// to modify.
#[derive(Serialize, Deserialize)]
pub struct AppSettings {
    /// The amount of money to calculate risk from.
    pub deposit: Decimal,
    /// The risk percentage.
    pub risk: Decimal,
    /// This parameter determines whether the bot is
    /// allowed to trade or not.
    pub is_active: bool,
} // end struct AppSettings

/// This struct contains all the functionality to manipulate
/// the app settings.
pub struct AppSettingsService;

/// Implement all the functionality to manipulate the app settings.
impl AppSettingsService {
    /// This function loads the settings from a file.
    pub fn load_settings() -> Result<AppSettings> {
        // Get a file with settings.
        let mut settings_file = fs::File::open(APP_SETTINGS_FILE)?;

        // Declare a buffer to read the result to.
        let mut buffer = String::new();

        // Read the file.
        settings_file.read_to_string(&mut buffer)?;

        // Parse the information into a settings structure.
        let app_settings = serde_json::from_str::<AppSettings>(&buffer)?;

        Ok(app_settings)
    } // end fn load_settings

    /// This function saves the app settings.
    pub fn save_settings(app_settings: AppSettings) -> Result<()> {
        // Open a file to write the data to.
        let mut settings_file = fs::File::create(APP_SETTINGS_FILE)?;

        // Parse the data into a string.
        let str_data = serde_json::to_string(&app_settings)?;

        // Save the data to file.
        settings_file.write_all(str_data.as_bytes())?;

        Ok(())
    } // end fn save_settings
} // end impl AppSettingsService
