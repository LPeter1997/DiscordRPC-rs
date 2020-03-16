//! The RPC client based on a `Connection`.

use crate::{Result, Error, Connection, IpcConnection};
use crate::message::*;

/// Represents the different states the `Client` can be in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum State {
    Initial,
    Disconnected,
    SentHandshake,
    Connected,
}

/// Represents an RPC client with a `Connection`.
pub struct Client {
    connection: Box<dyn Connection>,
    state: State,
}

impl Client {
    /// Creates a new `Client` from the given `Connection`.
    pub fn from_connection<C: Connection>(connection: C) -> Self where C: 'static {
        Self{
            connection: Box::new(connection),
            state: State::Initial,
        }
    }

    /// Tries to build a `Connection` for all the possible Discord servers and
    /// create a client from it.
    pub fn build_connection<C: Connection>() -> Result<Self> where C: 'static {
        for i in 0..10 {
            if let Ok(conn) = C::connect(i) {
                return Ok(Self::from_connection(conn));
            }
        }
        Err(Error::DiscordNotRunning)
    }

    /// Tries to build an `IpcConnection` with `build_connection`.
    pub fn build_ipc_connection() -> Result<Self> {
        Self::build_connection::<IpcConnection>()
    }

    /// Tries to authenticate the application with the given client ID.
    pub fn open(&mut self, client_id: &str) {
        // If already past the initial state, don't even try
        if self.state != State::Initial {
            return;
        }

        // Send message
        let result = Message::new(MessageType::Handshake, serde_json::json!{{
            "client_id": client_id,
            "v": 1,
        }}).encode_to(&mut self.connection);
    }

}

/// Returns the current processes ID.
fn pid() -> u32 {
    std::process::id()
}

/// Returns a UUID `String`
fn nonce() -> String {
    use uuid::Uuid;
    Uuid::new_v4().to_string()
}
