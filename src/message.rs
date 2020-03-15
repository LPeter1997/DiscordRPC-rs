//! Messages that can be sent through connections.

use std::convert::{TryFrom, TryInto};
use std::io::{Read, Write};
use crate::{Result, Error};

/// The different message types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MessageType {
    Handshake,
    Frame,
    Close,
    Ping,
    Pong,
}

impl Into<u32> for MessageType {
    fn into(self) -> u32 {
        match self {
            Self::Handshake => 0,
            Self::Frame => 1,
            Self::Close => 2,
            Self::Ping => 3,
            Self::Pong => 4,
        }
    }
}

impl TryFrom<u32> for MessageType {
    type Error = Error;

    fn try_from(n: u32) -> Result<Self> {
        match n {
            0 => Ok(Self::Handshake),
            1 => Ok(Self::Frame),
            2 => Ok(Self::Close),
            3 => Ok(Self::Ping),
            4 => Ok(Self::Pong),
            x => Err(Error::MessageTypeError(x)),
        }
    }
}

/// Represents a message with a given `MessageType` and JSON payload.
#[derive(Debug, Clone)]
pub struct Message {
    msg_type: MessageType,
    payload: serde_json::Value,
}

impl Message {
    /// Creates a `Message` with the given `MessageType` and payload.
    pub fn new(msg_type: MessageType, payload: serde_json::Value) -> Self {
        Self{ msg_type, payload }
    }

    /// Returns the value under the key `"nonce"`, if found.
    pub fn nonce(&self) -> Option<&str> {
        self.payload["nonce"].as_str()
    }

    /// Tries to encode this `Message` to the given writer.
    pub fn encode_to(&self, mut writer: impl Write) -> Result<()> {
        let payload = self.payload.to_string();
        let mut buffer = Vec::with_capacity(8 + payload.len());

        let ty: u32 = self.msg_type.into();
        let payload_len = payload.len() as u32;
        buffer.extend_from_slice(&ty.to_le_bytes());
        buffer.extend_from_slice(&payload_len.to_le_bytes());
        buffer.extend_from_slice(payload.as_bytes());

        writer.write(&buffer)?;

        writer.flush()?;
        Ok(())
    }

    /// Tries to decode a `Message` from the given reader.
    pub async fn decode_from(mut reader: impl Read) -> Result<Self> {
        let mut ty = [0u8; 4];
        let mut len = [0u8; 4];
        reader.read_exact(&mut ty)?;
        let ty = u32::from_le_bytes(ty);
        let ty: MessageType = ty.try_into()?;
        reader.read_exact(&mut len)?;
        let len = u32::from_le_bytes(len);
        let mut payload = vec![0u8; len as usize];
        reader.read_exact(&mut payload)?;
        let payload = String::from_utf8(payload)?;
        let payload = serde_json::from_str(&payload)?;
        Ok(Self::new(ty, payload))
    }
}
