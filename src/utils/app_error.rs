// This file contains a custom error type for the application.

use teloxide::RequestError;

/// This structure represents a custom error for the application.
#[derive(Debug)]
pub struct AppError {
    /// Error reason.
    message: String,
} // end struct AppError

/// Implement the functionality for custom error.
impl AppError {
    /// This function creates a new App Error.
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        } // end Self
    } // end fn new
} // end impl AppError

/// Implement a conversion from RequestError into AppError.
impl From<RequestError> for AppError {
    /// A function that determines the algorithm for conversion
    /// from RequestError into custom AppError.
    fn from(value: RequestError) -> Self {
        Self {
            message: value.to_string(),
        } // end Self
    } // end fn from
} // end impl From<RequestError> for AppError

/// Implement an opportunity for an error to be displayed.
impl std::fmt::Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Simply write the message field to the formatter
        write!(f, "{}", self.message)
    } // end fn fmt
} // end impl std::fmt::Display for AppError
