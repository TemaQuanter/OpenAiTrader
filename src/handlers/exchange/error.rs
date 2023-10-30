// This file contains a parsing error that should be used in case
// there is a problem with converting results from ChatGPT into
// actual orders on crypto exchange.

use std::error::Error;
use std::fmt;

/// This error is triggered when there is a problem
/// to unambiguously parse the data to send an order to
/// an exchange to execute it.
#[derive(Debug)]
pub struct ParseError {
    details: String,
} // end struct ParseError

/// Implement all the functionality to create an use an error.
impl ParseError {
    /// This function creates an error from a message.
    pub fn new(details: &str) -> Self {
        ParseError {
            details: details.to_string(),
        } // end ParseError
    } // end fn new
} // end impl ParseError

/// Implement Display trait to make it possible to display
/// the error messages.
impl fmt::Display for ParseError {
    /// This function defines the behavior of proper error
    /// message printing.
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.details)
    } // end fn fmt
} // end impl fmt::Display for ParseError

/// Implement the default Error for ParseError
impl Error for ParseError {}
