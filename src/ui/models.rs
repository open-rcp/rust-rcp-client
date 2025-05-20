use std::time::SystemTime;
use serde::{Deserialize, Serialize};

/// Application state
#[derive(Debug, Clone, Default)] // Added Default
pub struct AppState {
    pub is_connected: bool,
    pub connecting: bool,
    pub connection_status: String, // Ensure this field exists
    pub password: String,
    pub show_password: bool,
    pub last_validated_address: Option<String>,
    pub connection_time: Option<SystemTime>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            is_connected: false,
            connecting: false,
            connection_status: "Disconnected".to_string(), // Initialize
            password: String::new(),
            show_password: false,
            last_validated_address: None,
            connection_time: None,
        }
    }
    
    /// Record the connection time
    pub fn set_connected(&mut self, connected: bool) {
        self.is_connected = connected;
        if connected {
            self.connection_time = Some(SystemTime::now());
            self.connection_status = "Connected".to_string();
        } else {
            self.connection_time = None;
            self.connection_status = "Disconnected".to_string();
        }
    }
}

/// Connection history entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionEntry {
    /// Server address
    pub address: String,
    /// Server port
    pub port: String,
    /// Username (optional)
    pub username: Option<String>,
    /// Authentication method
    pub auth_method: String,
    /// Last connection time
    pub last_connected: SystemTime,
    /// Connection was successful
    pub successful: bool,
}

impl ConnectionEntry {
    /// Create a new connection history entry
    pub fn new(address: &str, port: &str, username: Option<&str>, auth_method: &str) -> Self {
        Self {
            address: address.to_string(),
            port: port.to_string(),
            username: username.map(|s| s.to_string()),
            auth_method: auth_method.to_string(),
            last_connected: SystemTime::now(),
            successful: false,
        }
    }
    
    /// Mark connection as successful
    pub fn mark_successful(&mut self) {
        self.successful = true;
        self.last_connected = SystemTime::now();
    }
    
    /// Format connection as a display string
    pub fn display_string(&self) -> String {
        if let Some(ref username) = self.username {
            format!("{}@{}:{}", username, self.address, self.port)
        } else {
            format!("{}:{}", self.address, self.port)
        }
    }
}
