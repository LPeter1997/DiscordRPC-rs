//! The RPC client based on a `Connection`.

use std::time::Duration;
use std::sync::Arc;
use std::collections::HashMap;
use std::pin::Pin;
use tokio::sync::Mutex;
use tokio::task::JoinHandle;
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
    reader: Box<dyn AsyncRead>,
    writer: Box<dyn AsyncWrite>,
    messages: Shared<HashMap<String, Message>>,
}

impl Client {
    /// Creates a new `Client` from the given `Connection`.
    pub async fn from_connection(connection: impl Connection) -> Self {
        let (reader, writer) = connection.split();
        let (reader, writer) = (Box::new(reader), Box::new(writer));

        let messages = Arc::new(Mutex::new(HashMap::new()));

        Self{ reader, writer, messages }
    }

    /// Tries to build a `Connection` for all the possible Discord servers and
    /// create a client from it. An optional timeout can be given for each
    /// trial.
    pub async fn build_connection<C: Connection>(timeout: Option<Duration>) -> Result<Self> {
        for i in 0..10 {
            if let Ok(c) = C::connect(i, timeout).await {
                return Ok(Self::from_connection(c).await);
            }
        }
        Err(Error::DiscordNotRunning)
    }

    /// Tries to build an `IpcConnection` with `build_connection`.
    pub async fn build_ipc_connection(timeout: Option<Duration>) -> Result<Self> {
        Self::build_connection::<IpcConnection>(timeout).await
    }
}

impl Client {
    /*/// Creates a new `Client` from a connection. For most use-cases, please use
    /// `connect` instead.
    pub fn new(connection: C) -> Self {
        let (reader, writer) = connection.split();
        //let writer = Arc::new(Mutex::new(writer));
        let messages = Arc::new(Mutex::new(HashMap::new()));

        let reader_thread = {
            let messages = messages.clone();
            let mut reader = reader;
            tokio::spawn(async move {
                // Reader loop
                // TODO: This will never terminate!
                loop {
                    if let Ok(msg) = Message::decode_from(Pin::new(&mut reader)).await {
                        if let Some(nonce) = msg.nonce() {
                            messages.lock().await.insert(nonce.into(), msg);
                        }
                    }
                }
            })
        };

        Self{
            writer,
            messages,
            reader_thread,
        }
    }

    /// Tries to connect to the locally running Discord RPC server. An optional
    /// timeout can be given.
    pub fn connect(timeout: Option<time::Duration>) -> Result<Self> {
        for i in 0..10 {
            if let Ok(connection) = C::connect(i, timeout) {
                return Ok(Self::new(connection));
            }
        }
        Err(Error::DiscordNotRunning)
    }*/

    // TODO
    /*pub fn close(self) {
        unimplemented!();
    }*/

    // TODO: Return token, document
    /*pub async fn authorize(&mut self) -> Result<Message> {
        self.request(MessageType::Handshake, serde_json::json!{{
            "client_id": "192741864418312192",
            "v": 1
        }}).await
    }*/

    /*// TODO: Response await, document
    async fn request(&mut self, msg_ty: MessageType, mut json: serde_json::Value) -> Result<Message> {
        // Slap on an identifier, we expect this same identifier on the result
        let nonce = nonce();
        json["nonce"] = serde_json::Value::String(nonce.clone());
        let msg = Message::new(msg_ty, json);
        // Write it
        println!("Before send");
        let writer = Pin::new(&mut self.writer);
        msg.encode_to(writer).await?;
        println!("After send");
        // TODO: This should be nice and async
        loop {
            if let Some(msg) = self.messages.lock().await.remove(&nonce) {
                return Ok(msg);
            }
            tokio::task::yield_now().await;
        }
    }*/
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
