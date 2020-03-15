//! The RPC client based on a `Connection`.

use std::time::Duration;
use std::sync::Arc;
use std::collections::HashMap;
use std::pin::Pin;
use tokio::sync::Mutex;
use tokio::task;
use tokio::time;
use tokio::io::{AsyncRead, AsyncWrite};
use super::connection::{Connection, IpcConnection};
use crate::message::*;
use crate::{Result, Error};

type Shared<T> = Arc<Mutex<T>>;

impl Drop for Client {
    fn drop(&mut self) {
        println!("Drop");
    }
}

/// Represents an RPC client with a `Connection`.
pub struct Client {
    writer: Box<dyn AsyncWrite + Unpin>,
    messages: Shared<HashMap<String, Message>>,
}

impl Client {
    /// Creates a new `Client` from the given `Connection`.
    pub async fn from_connection(connection: impl Connection) -> Result<Self> {
        let (mut reader, writer) = connection.split();
        let writer = Box::new(writer);

        let messages = Arc::new(Mutex::new(HashMap::new()));

        let ms = messages.clone();
        // TODO: A way to stop this?
        task::spawn(async move {
            let mut reader = Pin::new(&mut reader);
            loop {
                println!("Read X");
                if let Ok(msg) = Message::decode_from(&mut reader).await {
                    println!("Read Y");
                    if let Some(nonce) = msg.nonce() {
                        println!("Read Z");
                        ms.lock().await.insert(nonce.to_string(), msg);
                    }
                    else {
                        // TODO: What to do with messages without `"nonce"`?
                    }
                }
                else {
                    // TODO: What to do with bad messages?
                }
                task::yield_now().await;
            }
        });

        Ok(Self{ writer, messages })
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

    /// Sends a request that awaits for a message response. An optional timeout
    /// can be given.
    pub async fn request(&mut self, msg_ty: MessageType, mut json: serde_json::Value,
        timeout: Option<Duration>) -> Result<Message> {

        // Slap on an identifier, we expect this same identifier on the result
        let nonce = nonce();
        json["nonce"] = serde_json::Value::String(nonce.clone());
        let msg = Message::new(msg_ty, json);
        // Write it
        let writer = Pin::new(&mut self.writer);
        msg.encode_to(writer).await?;

        // We loop to wait for a response
        let messages = self.messages.clone();
        let join = task::spawn(async move {
            // TODO: This could be improved with semaphores
            // When a read happens, notify all threads
            loop {
                println!("Before find");
                if let Some(msg) = messages.lock().await.remove(&nonce) {
                    println!("Found");
                    return Ok(msg);
                }
                println!("Did not find");
                task::yield_now().await;
            }
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
        self.request(MessageType::Handshake, serde_json::json!{{
            "client_id": "192741864418312192",
            "v": 1
        }}, timeout).await
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
