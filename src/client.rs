//! The RPC client based on a `Connection`.

use crate::{Result, Error, Connection, IpcConnection};

/// Represents an RPC client with a `Connection`.
pub struct Client {
    connection: Box<dyn Connection>,
}

impl Client {
    /// Creates a new `Client` from the given `Connection`.
    pub fn from_connection<C>(connection: C) -> Self where C: Connection + 'static {
        Self{ connection: Box::new(connection) }
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
