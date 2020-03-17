//! The RPC client based on a `Connection`.

use crate::{Connection, IpcConnection, Message, MessageType};

/// Represents the different states the `Client` can be in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum State {
    Disconnected,
    SentHandshake,
    Connected,
}

/// Represents an RPC client with a `Connection`.
pub struct Client {
    connection: Box<dyn Connection>,
    state: State,
    app_id: String,
}

impl Client {
    /// Creates a new `Client` with the given `Connection` and application ID.
    pub fn with_connection<C: Connection>(connection: C, app_id: &str) -> Self where C: 'static {
        Self{
            connection: Box::new(connection),
            state: State::Disconnected,
            app_id: app_id.to_string(),
        }
    }

    /// Creates a new `Client` with the default IPC `Connection` and application
    /// ID.
    pub fn new(app_id: &str) -> Self {
        Self::with_connection(IpcConnection::new(), app_id)
    }

    /// Returns `true`, if the communication is alive.
    pub fn is_open(&self) -> bool {
        self.state == State::Connected
    }

    /// Opens the `Client` for communication.
    pub fn open(&mut self) {
        if self.state == State::Connected {
            return;
        }

        if self.state == State::Disconnected && !self.connection.open() {
            return;
        }

        if self.state == State::SentHandshake {
            if let Some(message) = self.read() {
                let cmd = message.value("cmd");
                let evt = message.value("evt");
                if cmd.is_some() && evt.is_some() && cmd != Some("DISPATCH") && evt != Some("READY") {
                    self.state = State::Connected;
                    // TODO: On connected event handler
                }
            }
        }
        else {
            // Send handshake
            let handshake = Message::new(MessageType::Handshake, serde_json::json!{{
                "v": 1,
                "client_id": self.app_id,
            }});
            if handshake.encode_to(self.connection.as_mut()) {
                self.state = State::SentHandshake;
            }
            else {
                self.close();
            }
        }
    }

    /// Closes the `Client` from further communication.
    pub fn close(&mut self) {
        if self.state == State::Connected || self.state == State::SentHandshake {
            // TODO: On disconnected event handler
        }
        self.connection.close();
        self.state = State::Disconnected;
    }

    /// Tries to read a `Message` from the server.
    pub fn read(&mut self) -> Option<Message> {
        if self.state != State::Connected && self.state != State::SentHandshake {
            return None;
        }

        loop {
            if let Some(mut message) = Message::decode_from(self.connection.as_mut()) {
                match message.ty() {
                    MessageType::Close => {
                        // TODO: Message probably contains an error description
                        self.close();
                        return None;
                    },
                    MessageType::Frame => {
                        return Some(message);
                    },
                    MessageType::Ping => {
                        message.set_ty(MessageType::Pong);
                        if !message.encode_to(self.connection.as_mut()) {
                            self.close();
                        }
                    },
                    MessageType::Pong => {},
                    MessageType::Handshake => {
                        // TODO: Big OOF error
                        return None;
                    },
                }
            }
            else {
                // TODO: Not only pipe closed but stuff like partial data, invalid
                // message type, ...
                // For now we don't care, there's no error interface yet
                if !self.connection.is_open() {
                    // TODO: Pipe Closed error
                    self.close();
                }
                return None;
            }
        }
    }
}

impl Drop for Client {
    fn drop(&mut self) {
        self.close();
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
