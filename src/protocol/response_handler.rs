use crate::protocol::{Message, MessageType, ProtocolError};
use anyhow::Result;
use serde_json::{json, Value};
use uuid::Uuid;

/// Handle an RCP response message
pub async fn handle_response(response: &Message, request_id: &Uuid) -> Result<Value> {
    if response.message_type != MessageType::Response {
        return Err(ProtocolError::Other(format!(
            "Expected response message, got {}",
            response.message_type
        ))
        .into());
    }

    let payload = &response.payload;

    // Check if the response is for our request
    if let Some(response_request_id) = payload
        .get("request_id")
        .and_then(|v| v.as_str())
        .and_then(|s| Uuid::parse_str(s).ok())
    {
        if response_request_id != *request_id {
            return Err(ProtocolError::Other(format!(
                "Response for wrong request: expected {}, got {}",
                request_id, response_request_id
            ))
            .into());
        }
    }

    // Check if the response indicates success
    if let Some(success) = payload.get("success").and_then(|v| v.as_bool()) {
        if !success {
            let message = payload
                .get("message")
                .and_then(|v| v.as_str())
                .unwrap_or("Unknown error");
            return Err(ProtocolError::ServerError(message.to_string()).into());
        }
    }

    // Return the response data
    if let Some(data) = payload.get("data") {
        return Ok(data.clone());
    }

    // If there's no data, return an empty object
    Ok(json!({}))
}
