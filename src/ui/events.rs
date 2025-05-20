
/// GUI Application events
#[derive(Debug, Clone)]
pub enum AppEvent {
    /// Connect to server
    Connect,
    /// Disconnect from server
    Disconnect,
    /// Connection succeeded
    ConnectionSucceeded, // Removed protocol::Client
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
    /// Disconnection Confirmed
    DisconnectedConfirmed,
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
    /// Save configuration
    SaveConfig,
    /// Update status with a message
    StatusUpdate(String),
}
