//! The RPC client based on a `Connection`.

use std::fmt;
use crate::{Connection, IpcConnection, Message, MessageType, Error};

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
    // Event handlers
    on_connect: Box<dyn Fn() + Send>,
    on_error: Box<dyn Fn(Error) + Send>,
    on_disconnect: Box<dyn Fn() + Send>,
}

impl Client {
    /// Creates a new `Client` with the given `Connection` and application ID.
    pub fn with_connection<C: Connection>(connection: C, app_id: &str) -> Self where C: 'static {
        Self{
            connection: Box::new(connection),
            state: State::Disconnected,
            app_id: app_id.to_string(),

            on_connect: Box::new(|| {}),
            on_error: Box::new(|_| {}),
            on_disconnect: Box::new(|| {}),
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
                if cmd == Some("DISPATCH") && evt == Some("READY") {
                    self.state = State::Connected;
                    (self.on_connect)();
                }
            }
        }
        else {
            // Send handshake
            let handshake = Message::new(MessageType::Handshake, serde_json::json!{{
                "v": 1,
                "client_id": self.app_id,
            }});
            if self.write(handshake) {
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
            (self.on_disconnect)();
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
            let message = Message::decode_from(self.connection.as_mut());
            if message.is_err() {
                let err = message.unwrap_err();
                (self.on_error)(err);
                self.close();
                return None;
            }

            let message = message.unwrap();
            if let Some(mut message) = message {
                match message.ty() {
                    MessageType::Close => {
                        // Forced by server, read description, send error
                        let code = message.value("code")
                            .and_then(|s| s.parse::<i32>().ok()).unwrap_or(0);
                        let message = message.value("message").unwrap_or("<none>").to_string();
                        (self.on_error)(Error::ConnectionClosed{ code, message });
                        self.close();
                        return None;
                    },
                    MessageType::Frame => {
                        return Some(message);
                    },
                    MessageType::Ping => {
                        // Send pong
                        message.set_ty(MessageType::Pong);
                        if !self.write(message) {
                            // If we couldn't send Pong, close
                            self.close();
                        }
                    },
                    MessageType::Pong => {
                        // No-op
                    },
                    x => {
                        // Any other message type is invalid here
                        (self.on_error)(Error::InvalidMessage(format!(
                            "Message of type {:?} can't be sent by the server!", x)));
                        self.close();
                        return None;
                    },
                }
            }
            else {
                if !self.connection.is_open() {
                    // TODO: Can we get the reason?
                    (self.on_error)(Error::PipeClosed("Unknown reason".into()));
                    self.close();
                }
                return None;
            }
        }
    }

    /// Tries to write a `Message` to the server. Returns `false` on failure.
    pub fn write(&mut self, message: Message) -> bool {
        message.encode_to(self.connection.as_mut())
    }
}

impl Drop for Client {
    fn drop(&mut self) {
        self.close();
    }
}

// TODO: Can we do better?
impl fmt::Debug for Client {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Client")
            .field("state", &self.state)
            .field("app_id", &self.app_id)
            .finish()
    }
}
