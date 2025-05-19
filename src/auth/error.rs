use thiserror::Error;

/// Authentication error type
#[derive(Debug, Error)]
pub enum AuthError {
    /// User declined authentication
    #[error("User declined authentication")]
    UserDeclined,

    /// Invalid credentials provided
    #[error("Invalid credentials")]
    InvalidCredentials,

    /// Authentication method not supported
    #[error("Authentication method not supported: {0}")]
    UnsupportedMethod(String),

    /// Authentication timed out
    #[error("Authentication timed out")]
    Timeout,

    /// Failed to interact with OS authentication
    #[error("Failed to interact with OS authentication: {0}")]
    OsAuthFailure(String),

    /// Failed to load credentials from keyring
    #[error("Failed to load credentials: {0}")]
    KeyringError(#[from] keyring::Error),

    /// Authentication blocked by system policy
    #[error("Authentication blocked by system policy")]
    PolicyBlocked,

    /// Error from the protocol layer
    #[error("Protocol error: {0}")]
    Protocol(#[from] crate::protocol::ProtocolError),

    /// Other authentication error
    #[error("Authentication error: {0}")]
    Other(String),
}
