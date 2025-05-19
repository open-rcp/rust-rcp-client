use thiserror::Error;

/// Errors that can occur in the protocol
#[derive(Debug, Error)]
pub enum ProtocolError {
    /// The message payload was malformed
    #[error("Malformed message payload: {0}")]
    MalformedPayload(String),

    /// The transport layer encountered an error
    #[error("Transport error: {0}")]
    Transport(String),

    /// Authentication failed
    #[error("Authentication failed: {0}")]
    AuthenticationFailed(String),

    /// Server error
    #[error("Server error: {0}")]
    ServerError(String),

    /// Channel closed
    #[error("Channel closed")]
    ChannelClosed,

    /// Timeout
    #[error("Operation timed out")]
    Timeout,

    /// Other error
    #[error("Protocol error: {0}")]
    Other(String),
}
