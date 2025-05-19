use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fmt;
use uuid::Uuid;

/// Types of messages in the RCP protocol
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MessageType {
    /// Authentication message
    Auth,

    /// Command message
    Command,

    /// Response message
    Response,

    /// Event message
    Event,

    /// Error message
    Error,

    /// Ping message (heartbeat)
    Ping,

    /// Pong message (heartbeat response)
    Pong,
}

impl fmt::Display for MessageType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MessageType::Auth => write!(f, "auth"),
            MessageType::Command => write!(f, "command"),
            MessageType::Response => write!(f, "response"),
            MessageType::Event => write!(f, "event"),
            MessageType::Error => write!(f, "error"),
            MessageType::Ping => write!(f, "ping"),
            MessageType::Pong => write!(f, "pong"),
        }
    }
}

/// A message in the RCP protocol
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    /// Unique ID for this message
    pub id: Uuid,

    /// Message type
    #[serde(rename = "type")]
    pub message_type: MessageType,

    /// Timestamp when the message was created
    pub timestamp: u64,

    /// Message payload
    pub payload: Value,
}

impl Message {
    /// Create a new message with the given type and payload
    pub fn new(message_type: MessageType, payload: Value) -> Self {
        Self {
            id: Uuid::new_v4(),
            message_type,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            payload,
        }
    }

    /// Create a new authentication message
    pub fn auth(username: &str, credentials: &[u8], method: &str) -> Self {
        Self::new(
            MessageType::Auth,
            serde_json::json!({
                "username": username,
                "credentials": credentials,
                "method": method,
            }),
        )
    }

    /// Create a new command message
    pub fn command(command: &str, params: Value) -> Self {
        Self::new(
            MessageType::Command,
            serde_json::json!({
                "command": command,
                "params": params,
            }),
        )
    }

    /// Create a new response message
    pub fn response(request_id: Uuid, success: bool, data: Value) -> Self {
        Self::new(
            MessageType::Response,
            serde_json::json!({
                "request_id": request_id,
                "success": success,
                "data": data,
            }),
        )
    }

    /// Create a new error message
    pub fn error(request_id: Option<Uuid>, code: u32, message: &str) -> Self {
        Self::new(
            MessageType::Error,
            serde_json::json!({
                "request_id": request_id,
                "code": code,
                "message": message,
            }),
        )
    }

    /// Create a new ping message
    pub fn ping() -> Self {
        Self::new(MessageType::Ping, serde_json::json!({}))
    }

    /// Create a new pong message
    pub fn pong(ping_id: Uuid) -> Self {
        Self::new(
            MessageType::Pong,
            serde_json::json!({
                "ping_id": ping_id,
            }),
        )
    }
}
