//! Error handling for this API.

/// The result-type for this library.
pub type Result<T> = std::result::Result<T, Error>;

/// The different kinds of errors this library can produce.
#[derive(Debug)]
pub enum Error {
    /// An `std::io::Error`.
    IoError(std::io::Error),
    /// A timeout from `tokio`.
    TimeoutError(tokio::time::Elapsed),
    /// An error while decoding a UTF8 string.
    Utf8Error(std::string::FromUtf8Error),
    /// A problem with `serde_json` serialization.
    SerdeJsonError(serde_json::error::Error),
    /// An invalid message type identifier while reading from the socket.
    MessageTypeError(u32),
    /// Could not connect to any Discord server on this machine.
    DiscordNotRunning,
}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Self::IoError(e)
    }
}

impl From<tokio::time::Elapsed> for Error {
    fn from(e: tokio::time::Elapsed) -> Self {
        Self::TimeoutError(e)
    }
}

impl From<std::string::FromUtf8Error> for Error {
    fn from(e: std::string::FromUtf8Error) -> Self {
        Self::Utf8Error(e)
    }
}

impl From<serde_json::error::Error> for Error {
    fn from(e: serde_json::error::Error) -> Self {
        Self::SerdeJsonError(e)
    }
}
