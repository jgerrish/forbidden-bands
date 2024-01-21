//! Error types for working with 8-bit strings
#![warn(missing_docs)]
#![warn(unsafe_code)]

use std::fmt::{Debug, Display, Formatter};

/// The types of errors we can return
pub enum ErrorKind {
    /// Generic error type
    // TODO: More error types
    Message(String),
}

/// It's an error type, with tons of info
pub struct Error {
    kind: ErrorKind,
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        match &self.kind {
            ErrorKind::Message(m) => write!(f, "Some error occurred: {:?}", m),
        }
    }
}

impl Debug for Error {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        write!(f, "{}", self)
    }
}

impl From<serde_json::Error> for Error {
    fn from(e: serde_json::Error) -> Self {
        Error {
            kind: ErrorKind::Message(e.to_string()),
        }
    }
}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Error {
            kind: ErrorKind::Message(e.to_string()),
        }
    }
}
