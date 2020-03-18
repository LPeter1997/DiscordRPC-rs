//! Error handling.

use std::fmt;
use std::error;

/// Errors during the communication.
#[derive(Debug)]
pub enum Error {
    /// The pipe was closed to the RPC server.
    PipeClosed(String),
    /// The connection was closed by the RPC server.
    ConnectionClosed{
        code: i32,
        message: String,
    },
    /// An invalid message type was sent by the server.
    InvalidMessage(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::PipeClosed(desc) =>
                write!(f, "Connection pipe closed: {}", desc),
            Self::ConnectionClosed{ code, message } =>
                write!(f, "Connection forced to close by server (code: {}): {}", code, message),
            Self::InvalidMessage(desc) =>
                write!(f, "Invalid message read: {}", desc),
        }
    }
}

impl error::Error for Error {}
