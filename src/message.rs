//! Messages that can be sent through connections.

use std::convert::{TryFrom, TryInto};
use std::time;
use serde_json as json;
use crate::{Connection, RichPresence, Error, pid, nonce};

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

    fn try_from(n: u32) -> Result<Self, Self::Error> {
        match n {
            0 => Ok(Self::Handshake),
            1 => Ok(Self::Frame),
            2 => Ok(Self::Close),
            3 => Ok(Self::Ping),
            4 => Ok(Self::Pong),
            x => Err(Error::InvalidMessage(
                format!("Unknown message-type identifier {}!", x))),
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

            // Timestamps
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

            // Assets
            if !rp.large_image_key.is_empty() || !rp.large_image_text.is_empty()
            || !rp.small_image_key.is_empty() || !rp.small_image_text.is_empty() {
                let mut assets = json::json!{{}};

                write_opt_string(&mut assets, "large_image", rp.large_image_key);
                write_opt_string(&mut assets, "large_text", rp.large_image_text);
                write_opt_string(&mut assets, "small_image", rp.small_image_key);
                write_opt_string(&mut assets, "small_text", rp.small_image_text);

                activity["assets"] = assets;
            }

            // Party
            if !rp.party_id.is_empty() || rp.party_size > 0 || rp.party_max > 0 {
                let mut party = json::json!{{}};

                write_opt_string(&mut party, "id", rp.party_id);
                if rp.party_size > 0 && rp.party_max > 0 {
                    party["size"] = json::Value::Array(vec![
                        json::Value::Number(rp.party_size.into()),
                        json::Value::Number(rp.party_max.into()),
                    ]);
                }

                activity["party"] = party;
            }

            // Secrets
            if !rp.match_secret.is_empty()
            || !rp.join_secret.is_empty()
            || !rp.spectate_secret.is_empty() {
                let mut secrets = json::json!{{}};

                write_opt_string(&mut secrets, "match", rp.match_secret);
                write_opt_string(&mut secrets, "join", rp.join_secret);
                write_opt_string(&mut secrets, "spectate", rp.spectate_secret);

                activity["secrets"] = secrets;
            }

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
    pub fn decode_from(conn: &mut dyn Connection) -> Result<Option<Self>, Error> {
        let mut ty = [0u8; 4];
        let mut len = [0u8; 4];

        // Message type
        if !conn.read(&mut ty) {
            return Ok(None);
        }

        let ty = u32::from_le_bytes(ty);
        let ty: MessageType = ty.try_into()?;
        if !conn.read(&mut len) {
            return Err(Error::InvalidMessage("Could not read message length!".into()));
        }
        let len = u32::from_le_bytes(len);
        let mut payload = vec![0u8; len as usize];
        if !conn.read(&mut payload) {
            return Err(Error::InvalidMessage("Partially read message frame!".into()));
        }
        let payload =  String::from_utf8(payload);
        if payload.is_err() {
            return Err(Error::InvalidMessage(format!(
                "Invalid message frame encoding: {}", payload.unwrap_err())));
        }
        let payload: json::Result<json::Value> = json::from_str(&payload.unwrap());
        if payload.is_err() {
            return Err(Error::InvalidMessage(format!(
                "Invalid message frame json: {}", payload.unwrap_err())));
        }
        let payload = payload.unwrap();
        Ok(Some(Message::new(ty, payload)))
    }
}
