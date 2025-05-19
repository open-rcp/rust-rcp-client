use crate::protocol;

/// GUI Application events
pub enum AppEvent {
    /// Connect to server
    Connect,
    /// Disconnect from server
    Disconnect,
    /// Connection succeeded
    ConnectionSucceeded(protocol::Client),
    /// Connection failed
    ConnectionFailed(String),
    /// Authentication succeeded
    AuthenticationSucceeded,
    /// Authentication failed
    AuthenticationFailed(String),
    /// Config saved
    ConfigSaved,
    /// Config save failed
    ConfigSaveFailed(String),
    /// Update UI status
    UpdateStatus(String),
    /// Update connection state
    UpdateConnectionState(bool),
    /// Connection in progress
    SetConnecting(bool),
    /// Update connection history
    UpdateConnectionHistory(String, String, Option<String>, String, bool),
    /// Save credentials
    SaveCredentials,
    /// Clear credentials
    ClearCredentials,
    /// Validate input
    ValidateInput(String),
}
