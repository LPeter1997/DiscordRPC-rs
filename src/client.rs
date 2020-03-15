//! The RPC client based on a `Connection`.

use std::time::Duration;
use std::pin::Pin;
use tokio::task;
use tokio::time;
use tokio::io::AsyncWrite;
use tokio::sync::watch;
use super::connection::{Connection, IpcConnection};
use crate::message::*;
use crate::{Result, Error};

impl Drop for Client {
    fn drop(&mut self) {
        println!("Drop");
    }
}

/// Represents an RPC client with a `Connection`.
pub struct Client {
    writer: Box<dyn AsyncWrite + Unpin>,
    rx: watch::Receiver<std::result::Result<Message, ()>>,
}

impl Client {
    /// Creates a new `Client` from the given `Connection`.
    pub async fn from_connection(connection: impl Connection) -> Result<Self> {
        let (mut reader, writer) = connection.split();
        let writer = Box::new(writer);
        let (tx, rx) = watch::channel(Err(()));

        // TODO: A way to stop this
        task::spawn(async move {
            let mut reader = Pin::new(&mut reader);
            loop {
                if let Ok(msg) = Message::decode_from(&mut reader).await {
                    tx.broadcast(Ok(msg));
                }
                else {
                    // TODO: What to do with bad messages?
                }
                task::yield_now().await;
            }
        });

        Ok(Self{ writer, rx })
    }

    /// Tries to build a `Connection` for all the possible Discord servers and
    /// create a client from it. An optional timeout can be given for each
    /// trial.
    pub async fn build_connection<C: Connection>(timeout: Option<Duration>) -> Result<Self> {
        for i in 0..10 {
            if let Ok(conn) = C::connect(i, timeout).await {
                if let Ok(client) = Self::from_connection(conn).await {
                    return Ok(client);
                }
            }
        }
        Err(Error::DiscordNotRunning)
    }

    /// Tries to build an `IpcConnection` with `build_connection`.
    pub async fn build_ipc_connection(timeout: Option<Duration>) -> Result<Self> {
        Self::build_connection::<IpcConnection>(timeout).await
    }

    /// Sends a request that awaits for a message response. An optional
    /// completion token and timeout can be given. If there's no completion
    /// token given, then the response is expected to have no `"nonce"`.
    async fn request_internal(&mut self, msg_ty: MessageType, mut json: serde_json::Value,
        nonce: Option<String>, timeout: Option<Duration>) -> Result<Message> {

        if let Some(nonce) = nonce.as_ref() {
            // Slap on an identifier, we expect this same identifier on the result
            json["nonce"] = serde_json::Value::String(nonce.clone());
        }
        let msg = Message::new(msg_ty, json);
        // Write it
        let writer = Pin::new(&mut self.writer);
        msg.encode_to(writer).await?;

        // We loop to wait for a response
        let mut rx = self.rx.clone();
        let join = task::spawn(async move {
            let nonce = nonce.as_ref().map(|n| n.as_str());
            while let Some(msg) = rx.recv().await {
                if let Ok(msg) = msg {
                    if msg.nonce() == nonce {
                        return Ok(msg);
                    }
                }
            }
            return Err(Error::ConnectionClosed);
        });

        // Wrap it into a timeout if needed
        if let Some(timeout) = timeout {
            let join = time::timeout(timeout, join);
            join.await??
        }
        else {
            join.await?
        }
    }

    // TODO: Client ID
    /// Sends an authorization request. An optional timeout can be given.
    pub async fn authorize(&mut self, timeout: Option<Duration>) -> Result<Message> {
        self.request_internal(MessageType::Handshake, serde_json::json!{{
            "client_id": "292341863318585192",
            "v": 1
        }}, None, timeout).await
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
