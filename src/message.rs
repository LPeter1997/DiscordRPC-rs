//! Messages that can be sent through connections.

use std::convert::{TryFrom, TryInto};
use std::time;
use serde_json as json;
use crate::{Connection, RichPresence, pid, nonce};

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
    type Error = ();

    fn try_from(n: u32) -> Result<Self, ()> {
        match n {
            0 => Ok(Self::Handshake),
            1 => Ok(Self::Frame),
            2 => Ok(Self::Close),
            3 => Ok(Self::Ping),
            4 => Ok(Self::Pong),
            _ => Err(()),
        }
    }
}

/// Represents a message with a given `MessageType` and JSON payload.
#[derive(Debug, Clone)]
pub struct Message {
    msg_type: MessageType,
    payload: json::Value,
}

impl Message {
    /// Creates a `Message` with the given `MessageType` and payload.
    pub fn new(msg_type: MessageType, payload: json::Value) -> Self {
        Self{ msg_type, payload }
    }

    /// Creates a `Message` for setting a `RichPresence`.
    pub fn rich_presence(rp: Option<RichPresence>) -> Self {
        // Helpers
        fn write_opt_string(json: &mut json::Value, key: &str, val: String) {
            if !val.is_empty() {
                json[key] = json::Value::String(val);
            }
        }

        fn time_to_u64(t: Option<time::SystemTime>) -> Option<u64> {
            t.ok_or(())
                .and_then(|t| t.duration_since(time::UNIX_EPOCH).map_err(|_| ()))
                .map(|t| t.as_secs()).ok()
        }

        let mut json = json::json!{{
            "nonce": nonce(),
            "cmd": "SET_ACTIVITY",
        }};
        let mut args = json::json!{{
            "pid": pid(),
        }};

        if let Some(rp) = rp {
            let mut activity = json::json!{{}};

            write_opt_string(&mut activity, "state", rp.state);
            write_opt_string(&mut activity, "details", rp.details);
            activity["instance"] = json::Value::Bool(rp.instance);

            let start_time = time_to_u64(rp.start_timestamp);
            let end_time = time_to_u64(rp.end_timestamp);

            if start_time.is_some() || end_time.is_some() {
                let mut timestamps = json::json!{{}};

                if let Some(start) = start_time {
                    timestamps["start"] = json::Value::Number(start.into());
                }

                if let Some(end) = end_time {
                    timestamps["end"] = json::Value::Number(end.into());
                }

                activity["timestamps"] = timestamps;
            }

            // TODO: Assets

            // TODO: Party

            // TODO: Secrets

            args["activity"] = activity;
        }

        json["args"] = args;
        Self::new(MessageType::Frame, json)
    }

    /// Returns the `MessageType` of this `Message`.
    pub fn ty(&self) -> MessageType {
        self.msg_type
    }

    /// Returns the value under a given key, if found.
    pub fn value(&self, key: &str) -> Option<&str> {
        self.payload[key].as_str()
    }

    /// Sets the `MessageType` of this `Message`.
    pub fn set_ty(&mut self, ty: MessageType) {
        self.msg_type = ty;
    }

    /// Tries to encode this `Message` to the given writer. Returns `true` on
    /// success.
    pub fn encode_to(&self, conn: &mut dyn Connection) -> bool {
        let payload = self.payload.to_string();
        let mut buffer = Vec::with_capacity(8 + payload.len());

        let ty: u32 = self.msg_type.into();
        let payload_len = payload.len() as u32;
        buffer.extend_from_slice(&ty.to_le_bytes());
        buffer.extend_from_slice(&payload_len.to_le_bytes());
        buffer.extend_from_slice(payload.as_bytes());

        conn.write(&buffer)
    }

    /// Tries to decode a `Message` from the given reader.
    pub fn decode_from(conn: &mut dyn Connection) -> Option<Self> {
        let mut ty = [0u8; 4];
        let mut len = [0u8; 4];
        if !conn.read(&mut ty) {
            return None;
        }
        let ty = u32::from_le_bytes(ty);
        if let Ok(ty) = ty.try_into() {
            if !conn.read(&mut len) {
                return None;
            }
            let len = u32::from_le_bytes(len);
            let mut payload = vec![0u8; len as usize];
            if !conn.read(&mut payload) {
                return None;
            }
            if let Ok(payload) = String::from_utf8(payload) {
                if let Ok(payload) = serde_json::from_str(&payload) {
                    return Some(Self::new(ty, payload));
                }
            }
        }
        None
    }
}
