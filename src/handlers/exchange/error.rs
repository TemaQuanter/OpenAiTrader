// This file contains a parsing error that should be used in case
// there is a problem with converting results from ChatGPT into
// actual orders on crypto exchange.

use std::error::Error;
use std::fmt;

/// This error is triggered when there is a problem
/// to unambiguously parse the data to send an order to
/// an exchange to execute it.
#[derive(Debug)]
pub struct ExchangeError {
    details: String,
} // end struct ExchangeError

/// Implement all the functionality to create an use an error.
impl ExchangeError {
    /// This function creates an error from a message.
    pub fn new(details: &str) -> Self {
        ExchangeError {
            details: details.to_string(),
        } // end ExchangeError
    } // end fn new
} // end impl ExchangeError

/// Implement Display trait to make it possible to display
/// the error messages.
impl fmt::Display for ExchangeError {
    /// This function defines the behavior of proper error
    /// message printing.
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.details)
    } // end fn fmt
} // end impl fmt::Display for ExchangeError

/// Implement the default Error for ExchangeError
impl Error for ExchangeError {}
