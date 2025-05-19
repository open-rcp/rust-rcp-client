// filepath: /Volumes/EXT/repos/open-rcp/rust-rcp-client/src/ui/gui.rs
use crate::config::ClientConfig;
use crate::protocol;
use crate::auth;
use anyhow::Result;
use eframe::{self, egui};
use log::{info, error};
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex, oneshot};
use tokio::runtime::Handle;
use std::path::PathBuf;
use std::fs;
use serde::{Deserialize, Serialize};
use std::time::SystemTime;

fn load_connection_history() -> Vec<ConnectionEntry> {
    // Get history file path
    let history_path = get_history_file_path();
    
    // If file doesn't exist, return empty vector
    if !history_path.exists() {
        return Vec::new();
    }
    
    // Attempt to read and deserialize history file
    match fs::read_to_string(&history_path) {
        Ok(content) => {
            match serde_json::from_str::<Vec<ConnectionEntry>>(&content) {
                Ok(history) => history,
                Err(e) => {
                    error!("Failed to parse connection history: {}", e);
                    Vec::new()
                }
            }
        }
        Err(e) => {
            error!("Failed to read connection history: {}", e);
            Vec::new()
        }
    }
}

/// Save connection history to config
fn save_connection_history(history: &[ConnectionEntry]) {
    // Get history file path
    let history_path = get_history_file_path();
    
    // Ensure parent directory exists
    if let Some(parent) = history_path.parent() {
        if !parent.exists() {
            if let Err(e) = fs::create_dir_all(parent) {
                error!("Failed to create history directory: {}", e);
                return;
            }
        }
    }
    
    // Serialize and save history
    match serde_json::to_string_pretty(history) {
        Ok(content) => {
            if let Err(e) = fs::write(&history_path, content) {
                error!("Failed to write connection history: {}", e);
            }
        }
        Err(e) => {
            error!("Failed to serialize connection history: {}", e);
        }
    }
}

/// Get connection history file path
fn get_history_file_path() -> PathBuf {
    dirs::config_dir()
        .expect("Could not find config directory")
        .join("rcp_client")
        .join("connection_history.json")
}

/// Add or update connection in history
fn add_to_connection_history(
    history: &mut Vec<ConnectionEntry>,
    address: &str,
    port: &str,
    username: Option<&str>,
    auth_method: &str,
    successful: bool,
) {
    // Look for an existing entry
    let mut found = false;
    for entry in history.iter_mut() {
        if entry.address == address && entry.port == port {
            // Update existing entry
            if let Some(uname) = username {
                entry.username = Some(uname.to_string());
            }
            entry.auth_method = auth_method.to_string();
            entry.last_connected = SystemTime::now();
            if successful {
                entry.mark_successful();
            }
            found = true;
            break;
        }
    }
    
    // Add new entry if not found
    if !found {
        let mut entry = ConnectionEntry::new(address, port, username, auth_method);
        if successful {
            entry.mark_successful();
        }
        history.push(entry);
    }
    
    // Sort by last connected time (most recent first)
    history.sort_by(|a, b| b.last_connected.cmp(&a.last_connected));
    
    // Limit history to 10 entries
    if history.len() > 10 {
        history.truncate(10);
    }
    
    // Save updated history
    save_connection_history(history);
}

/// GUI Application events
enum AppEvent {
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

/// Connection history entry
#[derive(Debug, Clone, Serialize, Deserialize)]
struct ConnectionEntry {
    /// Server address
    address: String,
    /// Server port
    port: String,
    /// Username (optional)
    username: Option<String>,
    /// Authentication method
    auth_method: String,
    /// Last connection time
    last_connected: SystemTime,
    /// Connection was successful
    successful: bool,
}

impl ConnectionEntry {
    /// Create a new connection history entry
    fn new(address: &str, port: &str, username: Option<&str>, auth_method: &str) -> Self {
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
    fn mark_successful(&mut self) {
        self.successful = true;
        self.last_connected = SystemTime::now();
    }
    
    /// Format connection as a display string
    fn display_string(&self) -> String {
        if let Some(ref username) = self.username {
            format!("{}@{}:{}", username, self.address, self.port)
        } else {
            format!("{}:{}", self.address, self.port)
        }
    }
}

/// Application state
struct AppState {
    is_connected: bool,
    connecting: bool,
    connection_status: String,
}

impl AppState {
    fn new() -> Self {
        Self {
            is_connected: false,
            connecting: false,
            connection_status: "Disconnected".to_string(),
        }
    }
}

/// RCP Client GUI Application
pub struct RcpClientApp {
    /// Application config
    config: ClientConfig,
    /// Auto-connect on startup
    auto_connect: bool,
    /// Connection status
    connection_status: String,
    /// Connection state (true = connected)
    is_connected: bool,
    /// Connection in progress
    connecting: bool,
    /// Server address input
    server_address: String,
    /// Server port input
    server_port: String,
    /// Username input
    username: String,
    /// Authentication method
    auth_method: String,
    /// Remember credentials
    remember_credentials: bool,
    /// Connection history
    connection_history: Vec<ConnectionEntry>,
    /// Message channel
    event_tx: mpsc::Sender<AppEvent>,
    /// Client instance
    client: Arc<Mutex<Option<protocol::Client>>>,
    /// Status instance for thread-safe updates
    status: Arc<Mutex<String>>,
    /// App state for thread-safe updates
    app_state: Arc<Mutex<AppState>>,
    /// Tokio runtime handle
    rt_handle: Handle,
    /// Shutdown channel
    shutdown_tx: Option<oneshot::Sender<()>>,
}

impl RcpClientApp {
    /// Create a new application instance
    pub fn new(
        config: ClientConfig, 
        auto_connect: bool, 
        rt_handle: Handle, 
        shutdown_tx: oneshot::Sender<()>
    ) -> Self {
        let (event_tx, mut event_rx) = mpsc::channel(32);
        let client = Arc::new(Mutex::new(None));
        let status = Arc::new(Mutex::new("Disconnected".to_string()));
        let app_state = Arc::new(Mutex::new(AppState::new()));
        
        // Clone necessary values for the event handler
        let event_tx_clone = event_tx.clone();
        let client_clone = client.clone();
        let config_clone = config.clone();
        let status_clone = status.clone();
        let app_state_clone = app_state.clone();
        
        // Spawn async task to handle events on the runtime
        rt_handle.spawn(async move {
            while let Some(event) = event_rx.recv().await {
                match event {
                    AppEvent::Connect => {
                        let config = config_clone.clone();
                        let tx = event_tx_clone.clone();
                        
                        // Set connecting state
                        let _ = tx.send(AppEvent::SetConnecting(true)).await;
                        
                        // Attempt to connect to server
                        tokio::spawn(async move {
                            info!("Connecting to server {}:{}", config.server.address, config.server.port);
                            match protocol::Client::connect(&config.server.address, config.server.port).await {
                                Ok(client) => {
                                    let _ = tx.send(AppEvent::ConnectionSucceeded(client)).await;
                                }
                                Err(e) => {
                                    let _ = tx.send(AppEvent::ConnectionFailed(e.to_string())).await;
                                    // Reset connecting state
                                    let _ = tx.send(AppEvent::SetConnecting(false)).await;
                                    
                                    // Update connection history with failed attempt
                                    let _ = tx.send(AppEvent::UpdateConnectionHistory(
                                        config.server.address.clone(), 
                                        config.server.port.to_string(),
                                        config.auth.username.clone(),
                                        config.auth.method.clone(),
                                        false
                                    )).await;
                                }
                            }
                        });
                    }
                    AppEvent::ConnectionSucceeded(client) => {
                        // Store client in our shared state
                        let mut client_guard = client_clone.lock().await;
                        
                        // Get username
                        let username = config_clone.auth.username.clone().unwrap_or_else(|| {
                            std::env::var("USER")
                                .or_else(|_| std::env::var("USERNAME"))
                                .unwrap_or_else(|_| "user".to_string())
                        });
                        
                        // Get auth method
                        let auth_method = auth::AuthMethod::from_str(&config_clone.auth.method)
                            .unwrap_or(auth::AuthMethod::Password);
                        
                        // Create auth provider
                        let auth_provider = auth::create_provider(auth_method, &username);
                        
                        // Attempt authentication
                        match client.authenticate_with_provider(&*auth_provider).await {
                            Ok(true) => {
                                // Store the client if authentication succeeded
                                *client_guard = Some(client);
                                let _ = event_tx_clone.send(AppEvent::AuthenticationSucceeded).await;
                                
                                // Update connection state
                                let _ = event_tx_clone.send(AppEvent::UpdateConnectionState(true)).await;
                                
                                // Reset connecting state
                                let _ = event_tx_clone.send(AppEvent::SetConnecting(false)).await;
                                
                                // Add to connection history if successful
                                let address = config_clone.server.address.clone();
                                let port = config_clone.server.port.to_string();
                                let username = config_clone.auth.username.clone();
                                let auth_method = config_clone.auth.method.clone();
                                
                                let _ = event_tx_clone.send(AppEvent::UpdateConnectionHistory(
                                    address.clone(),
                                    port.clone(),
                                    username.clone(),
                                    auth_method.clone(),
                                    true
                                )).await;
                                
                                // Save credentials if requested
                                if config_clone.auth.save_credentials {
                                    let _ = event_tx_clone.send(AppEvent::SaveCredentials).await;
                                }
                                
                                // Update status
                                let _ = event_tx_clone.send(AppEvent::UpdateStatus(format!("Connected to {}:{}", address, port))).await;
                            }
                            Ok(false) => {
                                let _ = event_tx_clone.send(AppEvent::AuthenticationFailed("Authentication rejected".to_string())).await;
                                // Reset connecting state
                                let _ = event_tx_clone.send(AppEvent::SetConnecting(false)).await;
                                
                                // Update connection history with failed attempt
                                let _ = event_tx_clone.send(AppEvent::UpdateConnectionHistory(
                                    config_clone.server.address.clone(), 
                                    config_clone.server.port.to_string(),
                                    config_clone.auth.username.clone(),
                                    config_clone.auth.method.clone(),
                                    false
                                )).await;
                            }
                            Err(e) => {
                                let _ = event_tx_clone.send(AppEvent::AuthenticationFailed(e.to_string())).await;
                                // Reset connecting state
                                let _ = event_tx_clone.send(AppEvent::SetConnecting(false)).await;
                                
                                // Update connection history with failed attempt
                                let _ = event_tx_clone.send(AppEvent::UpdateConnectionHistory(
                                    config_clone.server.address.clone(), 
                                    config_clone.server.port.to_string(),
                                    config_clone.auth.username.clone(),
                                    config_clone.auth.method.clone(),
                                    false
                                )).await;
                            }
                        }
                    }
                    AppEvent::ConnectionFailed(error) => {
                        error!("Connection failed: {}", error);
                        // Update status
                        let _ = event_tx_clone.send(AppEvent::UpdateStatus(format!("Connection failed: {}", error))).await;
                    }
                    AppEvent::AuthenticationSucceeded => {
                        info!("Authentication succeeded");
                        // Update status
                        let _ = event_tx_clone.send(AppEvent::UpdateStatus("Connected and authenticated".to_string())).await;
                    }
                    AppEvent::AuthenticationFailed(error) => {
                        error!("Authentication failed: {}", error);
                        // Update status
                        let _ = event_tx_clone.send(AppEvent::UpdateStatus(format!("Authentication failed: {}", error))).await;
                    }
                    AppEvent::Disconnect => {
                        if let Some(_client) = client_clone.lock().await.take() {
                            info!("Disconnecting from server");
                            // Ideal implementation would call client.close() here
                            
                            // Update connection state
                            let _ = event_tx_clone.send(AppEvent::UpdateConnectionState(false)).await;
                            let _ = event_tx_clone.send(AppEvent::UpdateStatus("Disconnected".to_string())).await;
                        }
                    }
                    AppEvent::ConfigSaved => {
                        info!("Configuration saved successfully");
                        
                        // Update the UI status
                        let _ = event_tx_clone.send(AppEvent::UpdateStatus("Configuration saved".to_string())).await;
                    }
                    AppEvent::ConfigSaveFailed(error) => {
                        error!("Failed to save configuration: {}", error);
                        
                        // Update the UI status
                        let _ = event_tx_clone.send(AppEvent::UpdateStatus(format!("Config save failed: {}", error))).await;
                    }
                    AppEvent::UpdateStatus(status) => {
                        // Update the shared status
                        let mut status_guard = status_clone.lock().await;
                        *status_guard = status;
                    }
                    AppEvent::UpdateConnectionState(state) => {
                        // Update the app_state to reflect connection status
                        let mut app_state = app_state_clone.lock().await;
                        app_state.is_connected = state;
                    }
                    AppEvent::SetConnecting(connecting) => {
                        let mut app_state = app_state_clone.lock().await;
                        app_state.connecting = connecting;
                    }
                    AppEvent::UpdateConnectionHistory(address, port, username, auth_method, successful) => {
                        // Load existing history
                        let mut history = load_connection_history();
                        
                        // Add or update the connection
                        add_to_connection_history(
                            &mut history,
                            &address,
                            &port,
                            username.as_deref(),
                            &auth_method,
                            successful
                        );
                    }
                    AppEvent::SaveCredentials => {
                        // Update the config to save credentials
                        info!("Saving credentials for future use");
                        
                        // Update the config
                        let mut config = config_clone.clone();
                        config.auth.save_credentials = true;
                        
                        // Get config path
                        let config_path = dirs::config_dir()
                            .expect("Could not find config directory")
                            .join("rcp_client")
                            .join("config.toml");
                        
                        // Save config asynchronously
                        let tx = event_tx_clone.clone();
                        tokio::spawn(async move {
                            match crate::config::save_config(&config_path, &config).await {
                                Ok(_) => {
                                    info!("Credentials saved successfully");
                                    let _ = tx.send(AppEvent::UpdateStatus("Credentials will be saved".to_string())).await;
                                },
                                Err(e) => {
                                    error!("Failed to save credentials: {}", e);
                                    let _ = tx.send(AppEvent::UpdateStatus(format!("Failed to save credentials: {}", e))).await;
                                }
                            }
                        });
                    }
                    AppEvent::ClearCredentials => {
                        // Clear saved credentials by updating the config
                        info!("Clearing saved credentials");
                        
                        // Update the config
                        let mut config = config_clone.clone();
                        config.auth.save_credentials = false;
                        
                        // Get config path
                        let config_path = dirs::config_dir()
                            .expect("Could not find config directory")
                            .join("rcp_client")
                            .join("config.toml");
                        
                        // Save config asynchronously
                        let tx = event_tx_clone.clone();
                        tokio::spawn(async move {
                            match crate::config::save_config(&config_path, &config).await {
                                Ok(_) => {
                                    info!("Credentials cleared successfully");
                                    let _ = tx.send(AppEvent::UpdateStatus("Credentials will not be saved".to_string())).await;
                                },
                                Err(e) => {
                                    error!("Failed to clear credentials: {}", e);
                                    let _ = tx.send(AppEvent::UpdateStatus(format!("Failed to clear credentials: {}", e))).await;
                                }
                            }
                        });
                    }
                    AppEvent::ValidateInput(field) => {
                        // Perform async validation for inputs that need it
                        match field.as_str() {
                            "server_address" => {
                                // Get the server address from the config
                                let address = config_clone.server.address.clone();
                                let tx = event_tx_clone.clone();
                                
                                // Don't validate empty addresses
                                if address.is_empty() {
                                    return;
                                }
                                
                                // Spawn an async task to validate the address (DNS lookup or ping)
                                tokio::spawn(async move {
                                    // Simple DNS resolution test
                                    use tokio::net::lookup_host;
                                    
                                    // Try with default port for testing
                                    let addr_with_port = format!("{}:0", address);
                                    
                                    match lookup_host(addr_with_port).await {
                                        Ok(_) => {
                                            // Successfully resolved
                                            let _ = tx.send(AppEvent::UpdateStatus(format!("Address '{}' validated", address))).await;
                                        },
                                        Err(e) => {
                                            // Failed to resolve
                                            let _ = tx.send(AppEvent::UpdateStatus(format!("Address validation: {}", e))).await;
                                        }
                                    }
                                });
                            },
                            _ => {
                                // Other field validations could be added here
                            }
                        }
                    }
                }
            }
        });
        
        // Initialize UI state
        let connection_history = load_connection_history();
        
        let app = Self {
            server_address: config.server.address.clone(),
            server_port: config.server.port.to_string(),
            username: config.auth.username.clone().unwrap_or_default(),
            auth_method: config.auth.method.clone(),
            connection_status: "Disconnected".to_string(),
            is_connected: false,
            connecting: false,
            remember_credentials: config.auth.save_credentials,
            connection_history,
            config,
            auto_connect,
            event_tx,
            client,
            status,
            app_state,
            rt_handle,
            shutdown_tx: Some(shutdown_tx),
        };
        
        // Auto-connect if configured
        if auto_connect {
            app.connect();
        }
        
        app
    }
    
    /// Connect to server
    fn connect(&self) {
        let tx = self.event_tx.clone();
        // Update status immediately in UI
        if let Ok(mut status_guard) = self.status.try_lock() {
            *status_guard = "Connecting...".to_string();
        }
        self.rt_handle.spawn(async move {
            let _ = tx.send(AppEvent::Connect).await;
        });
    }
    
    /// Disconnect from server
    fn disconnect(&self) {
        let tx = self.event_tx.clone();
        // Update status immediately in UI
        if let Ok(mut status_guard) = self.status.try_lock() {
            *status_guard = "Disconnecting...".to_string();
        }
        self.rt_handle.spawn(async move {
            let _ = tx.send(AppEvent::Disconnect).await;
        });
    }
    
    /// Update connection status
    fn update_status(&mut self, status: String) {
        // Update the shared status for thread safety
        if let Ok(mut status_guard) = self.status.try_lock() {
            *status_guard = status.clone();
        }
        // Also update local status for immediate UI refresh
        self.connection_status = status;
    }
    
    /// Save current configuration
    fn save_config(&mut self) -> Result<()> {
        // Update config from UI values
        self.config.server.address = self.server_address.clone();
        self.config.server.port = self.server_port.parse().unwrap_or(8717);
        self.config.auth.method = self.auth_method.clone();
        self.config.auth.save_credentials = self.remember_credentials;
        
        if self.username.is_empty() {
            self.config.auth.username = None;
        } else {
            self.config.auth.username = Some(self.username.clone());
        }
        
        // Get config path (this should use the same logic as in main.rs)
        let config_path = dirs::config_dir()
            .expect("Could not find config directory")
            .join("rcp_client")
            .join("config.toml");
        
        // Instead of using block_on, we'll spawn a task to save the config
        let config = self.config.clone();
        let tx = self.event_tx.clone(); // For reporting results
        
        self.rt_handle.spawn(async move {
            match crate::config::save_config(&config_path, &config).await {
                Ok(_) => {
                    let _ = tx.send(AppEvent::ConfigSaved).await;
                }
                Err(e) => {
                    let error_msg = e.to_string();
                    error!("Failed to save config: {}", error_msg);
                    let _ = tx.send(AppEvent::ConfigSaveFailed(error_msg)).await;
                }
            }
        });
        
        // Return immediately - the UI will be updated when the event is processed
        Ok(())
    }
}



impl eframe::App for RcpClientApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Check for status updates - using try_lock to avoid blocking
        if let Ok(status_guard) = self.status.try_lock() {
            // If we can get the lock without blocking, update the UI status
            if self.connection_status != *status_guard {
                self.connection_status = status_guard.clone();
            }
        }
        
        // Update the local state from shared state
        if let Ok(app_state_guard) = self.app_state.try_lock() {
            self.is_connected = app_state_guard.is_connected;
            self.connecting = app_state_guard.connecting;
        }
        
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("RCP Client");
            
            // Connection status with colored indicator
            ui.horizontal(|ui| {
                ui.label("Status:");
                
                let status_color = if self.is_connected {
                    egui::Color32::GREEN
                } else if self.connecting {
                    egui::Color32::YELLOW
                } else {
                    egui::Color32::RED
                };
                
                ui.colored_label(status_color, &self.connection_status);
            });
            
            ui.add_space(10.0); // Add space for better layout
            
            // Server configuration in a collapsing section
            egui::CollapsingHeader::new("Server Configuration")
                .default_open(true)
                .show(ui, |ui| {
                    // Connection history dropdown
                    if !self.connection_history.is_empty() {
                        ui.horizontal(|ui| {
                            ui.label("Recent:");
                            egui::ComboBox::from_label("")
                                .selected_text("Select a recent connection")
                                .show_ui(ui, |ui| {
                                    for entry in &self.connection_history {
                                        let display_text = entry.display_string();
                                        let status_icon = if entry.successful {
                                            "✓ " // Checkmark for successful connections
                                        } else {
                                            "⚠ " // Warning for failed connections
                                        };
                                        let full_text = format!("{}{}", status_icon, display_text);
                                        
                                        if ui.selectable_label(false, full_text).clicked() {
                                            self.server_address = entry.address.clone();
                                            self.server_port = entry.port.clone();
                                            if let Some(ref username) = entry.username {
                                                self.username = username.clone();
                                            }
                                            self.auth_method = entry.auth_method.clone();
                                        }
                                    }
                                });
                                
                            if ui.button("🗑").on_hover_text("Clear connection history").clicked() {
                                self.connection_history.clear();
                                save_connection_history(&self.connection_history);
                            }
                        });
                        ui.add_space(5.0);
                    }
                    
                    // Server address with tooltip and validation
                    ui.horizontal(|ui| {
                        ui.label("Address:");
                        let response = ui.text_edit_singleline(&mut self.server_address);
                        
                        // Use methods directly on the response, but only call each method once
                        ui.label("").on_hover_text("Enter server hostname or IP address (Tab to navigate between fields)");
                        let changed = response.changed();
                        let lost_focus = response.lost_focus();
                        
                        // Validate address and trigger async validation if needed
                        if !self.server_address.is_empty() {
                            let valid_address = !self.server_address.contains(' ');
                            if valid_address {
                                ui.colored_label(egui::Color32::GREEN, "✓");
                                static mut LAST_VALIDATED: Option<String> = None;
                                unsafe {
                                    if changed {
                                        if LAST_VALIDATED.as_deref() != Some(&self.server_address) {
                                            let tx = self.event_tx.clone();
                                            let address = self.server_address.clone();
                                            LAST_VALIDATED = Some(address.clone());
                                            self.rt_handle.spawn(async move {
                                                let _ = tx.send(AppEvent::ValidateInput("server_address".to_string())).await;
                                            });
                                        }
                                    }
                                }
                            } else {
                                ui.colored_label(egui::Color32::RED, "⚠");
                                ui.label("Invalid server address");
                            }
                        }
                        // Allow Enter to advance to next field
                        // let lost_focus = server_addr_response.lost_focus();
                        let lost_focus = response.lost_focus();
                        if lost_focus && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                            ui.memory_mut(|mem| mem.request_focus(ui.next_auto_id()));
                        }
                    });
                    
                    // Port with validation
                    ui.horizontal(|ui| {
                        ui.label("Port:");
                        let port_edit = ui.text_edit_singleline(&mut self.server_port)
                            .on_hover_text("Enter server port (usually 8717)");
                        
                        // Validate port
                        if !self.server_port.is_empty() {
                            match self.server_port.parse::<u16>() {
                                Ok(port) => {
                                    if port > 0 {
                                        ui.colored_label(egui::Color32::GREEN, "✓")
                                            .on_hover_text(format!("Valid port: {}", port));
                                    } else {
                                        ui.colored_label(egui::Color32::RED, "⚠");
                                        ui.label("Port must be greater than 0");
                                    }
                                }
                                Err(_) => {
                                    ui.colored_label(egui::Color32::RED, "⚠");
                                    ui.label("Port must be a number between 1-65535");
                                }
                            }
                        }
                        
                        // Allow Enter to advance to next field
                        if port_edit.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                            // Move focus to the use TLS checkbox
                            ui.memory_mut(|mem| mem.request_focus(ui.next_auto_id()));
                        }
                    });
                    
                    // Option to use TLS
                    ui.horizontal(|ui| {
                        ui.label("Use TLS encryption");
                        let checkbox = ui.checkbox(&mut self.config.server.use_tls, "");
                        ui.label("🔒").on_hover_text("Secure the connection with TLS encryption");
                        
                        // Add more detailed explanation based on state
                        if self.config.server.use_tls {
                            ui.label("(Connection will be encrypted)")
                                .on_hover_text("TLS provides secure, encrypted communication with the server");
                        } else {
                            ui.label("(Connection will be unencrypted)")
                                .on_hover_text("Warning: Unencrypted connections may expose sensitive data");
                        }
                        
                        // Allow keyboard navigation
                        if checkbox.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                            // Find the Username field in the Authentication section and focus it
                            ui.memory_mut(|mem| mem.request_focus(ui.auto_id_with("username_field")));
                        }
                    });
                });
                
            ui.add_space(5.0);
            
            // Authentication configuration in a collapsing section
            egui::CollapsingHeader::new("Authentication")
                .default_open(true)
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label("Username:");
                        let response = ui.text_edit_singleline(&mut self.username);
                        ui.label("").on_hover_text("Enter your username for authentication");
                        let changed = response.changed();
                        let focus_lost = response.lost_focus();
                        
                        if !self.username.is_empty() {
                            if self.username.len() >= 3 && !self.username.contains(char::is_whitespace) {
                                ui.colored_label(egui::Color32::GREEN, "✓");
                            } else {
                                ui.colored_label(egui::Color32::RED, "⚠");
                                ui.label("Username must be at least 3 characters with no spaces");
                            }
                        }
                        
                        if focus_lost && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                            ui.memory_mut(|mem| mem.request_focus(ui.next_auto_id()));
                        }
                    });
                    
                    ui.horizontal(|ui| {
                        ui.label("Method:");
                        let _auth_dropdown = egui::ComboBox::from_label("")
                            .selected_text(&self.auth_method)
                            .show_ui(ui, |ui| {
                                ui.selectable_value(&mut self.auth_method, "password".to_string(), "Password");
                                ui.selectable_value(&mut self.auth_method, "psk".to_string(), "Pre-Shared Key");
                                ui.selectable_value(&mut self.auth_method, "native".to_string(), "Native OS");
                            });
                            
                        // Add info tooltip based on selected auth method
                        let info_text = match self.auth_method.as_str() {
                            "password" => "Standard password authentication",
                            "psk" => "Pre-shared key authentication",
                            "native" => "Use system-level authentication",
                            _ => "Unknown authentication method",
                        };
                        
                        // Show a colored icon based on how secure the method is
                        let (security_icon, security_color) = match self.auth_method.as_str() {
                            "password" => ("🔑", egui::Color32::YELLOW),   // Medium security
                            "psk" => ("🔒", egui::Color32::GREEN),         // High security
                            "native" => ("🛡", egui::Color32::LIGHT_GREEN), // Good security
                            _ => ("❓", egui::Color32::RED),                // Unknown
                        };
                        
                        ui.colored_label(security_color, security_icon)
                            .on_hover_text(info_text);
                    });
                    
                    // Additional auth options based on selected method
                    match self.auth_method.as_str() {
                        "password" => {
                            // Password input field with masking
                            ui.horizontal(|ui| {
                                ui.label("Password:");
                                
                                // Store password and visibility state
                                static mut PASSWORD: String = String::new();
                                static mut SHOW_PASSWORD: bool = false;
                                
                                let password = unsafe { &mut PASSWORD };
                                let show_password = unsafe { &mut SHOW_PASSWORD };
                                
                                // Create password display
                                let mut password_display = if !*show_password {
                                    "•".repeat(password.len())
                                } else {
                                    password.clone()
                                };
                                
                                let password_edit = ui.add(
                                    egui::TextEdit::singleline(&mut password_display)
                                        .password(!*show_password)
                                        .hint_text("Enter password")
                                );
                                
                                if password_edit.changed() && *show_password {
                                    *password = password_display.clone();
                                }
                                
                                // Toggle password visibility with button
                                if ui.button(if *show_password { "🙈" } else { "👁" }).clicked() {
                                    *show_password = !*show_password;
                                }
                                
                                password_edit.on_hover_text("Enter your password for authentication");
                            });
                            
                            // Password strength indicator, etc. could go here
                        }
                        "psk" => {
                            // PSK configuration could go here
                            ui.label("NOTE: PSK configuration is done in the client config file");
                        }
                        "native" => {
                            ui.label("Using native OS authentication mechanisms");
                        }
                        _ => {}
                    }
                    
                    // Remember credentials checkbox with better feedback
                    ui.horizontal(|ui| {
                        let remember_label = ui.checkbox(&mut self.remember_credentials, "Remember credentials")
                            .on_hover_text("Save connection credentials for future use");
                        
                        if self.remember_credentials {
                            ui.colored_label(egui::Color32::LIGHT_GREEN, "✓")
                                .on_hover_text("Credentials will be saved when connecting");
                        }
                        
                        if remember_label.changed() {
                            // If the user unchecks this, we should clear saved credentials
                            if !self.remember_credentials {
                                let tx = self.event_tx.clone();
                                self.rt_handle.spawn(async move {
                                    let _ = tx.send(AppEvent::ClearCredentials).await;
                                });
                            } else {
                                let tx = self.event_tx.clone();
                                self.rt_handle.spawn(async move {
                                    let _ = tx.send(AppEvent::SaveCredentials).await;
                                });
                            }
                        }
                    });
                });
            
            // Connection details section (only visible when connected)
            if self.is_connected {
                ui.add_space(5.0);
                
                egui::CollapsingHeader::new("Connection Details")
                    .default_open(true)
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.label("Connected to:");
                            ui.strong(format!("{}:{}", self.server_address, self.server_port));
                        });
                        
                        ui.horizontal(|ui| {
                            ui.label("User:");
                            ui.strong(&self.username);
                        });
                        
                        ui.horizontal(|ui| {
                            ui.label("Auth method:");
                            ui.strong(&self.auth_method);
                        });
                        
                        ui.horizontal(|ui| {
                            ui.label("Encryption:");
                            if self.config.server.use_tls {
                                ui.strong("TLS Encrypted 🔒");
                            } else {
                                ui.colored_label(egui::Color32::YELLOW, "Unencrypted ⚠");
                            }
                        });
                    });
            }
            
            ui.add_space(10.0);
            
            // Buttons with improved styling and keyboard shortcuts
            ui.horizontal(|ui| {
                // Input validation for connect button
                let inputs_valid = !self.server_address.is_empty() 
                    && !self.server_port.is_empty()
                    && self.server_port.parse::<u16>().is_ok()
                    && self.server_address.chars().all(|c| {
                        c.is_alphanumeric() || c == '.' || c == '-' || c == '_' || c == ':'
                    });
                
                // Create a bigger, more visually distinctive connect button
                let connect_text = if self.connecting {
                    "Connecting..."
                } else if self.is_connected {
                    "Reconnect"
                } else {
                    "Connect"
                };
                
                // Calculate button color based on connection state
                let button_color = if self.is_connected {
                    egui::Color32::from_rgb(100, 200, 100) // Green for connected
                } else if self.connecting {
                    egui::Color32::from_rgb(200, 200, 100) // Yellow for connecting
                } else if inputs_valid {
                    egui::Color32::from_rgb(100, 150, 255) // Blue for ready to connect
                } else {
                    egui::Color32::from_rgb(180, 180, 180) // Gray for disabled
                };
                
                // Custom connect button with better visual appearance
                let connect_response = ui.add_enabled(
                    !self.connecting && inputs_valid,
                    egui::Button::new(
                        egui::RichText::new(connect_text)
                            .size(18.0)
                            .color(if inputs_valid || self.is_connected || self.connecting { 
                                egui::Color32::WHITE 
                            } else { 
                                egui::Color32::GRAY 
                            })
                    )
                    .fill(button_color)
                    .min_size(egui::Vec2::new(120.0, 32.0))
                );
                
                // Add a tooltip with connection details
                let tooltip_text = if !inputs_valid {
                    "Please enter valid server address and port".to_string()
                } else if self.is_connected {
                    format!("Reconnect to {}:{}", self.server_address, self.server_port)
                } else {
                    format!("Connect to {}:{} using {} authentication", 
                        self.server_address, 
                        self.server_port,
                        self.auth_method)
                };
                
                // Apply tooltip to the tooltip text
                ui.label("").on_hover_text(tooltip_text);
                
                // Handle click response separately
                if connect_response.clicked() {
                    // First save the config
                    if let Err(e) = self.save_config() {
                        self.update_status(format!("Error saving config: {}", e));
                    } else {
                        self.update_status("Saving config and connecting...".to_string());
                        // Connect is triggered asynchronously after config save
                        self.connect();
                    }
                }
                
                // Add keyboard shortcut for connect
                if ui.input_mut(|i| i.key_pressed(egui::Key::Enter) && i.modifiers.ctrl) && inputs_valid && !self.connecting {
                    if let Err(e) = self.save_config() {
                        self.update_status(format!("Error saving config: {}", e));
                    } else {
                        self.update_status("Saving config and connecting...".to_string());
                        self.connect();
                    }
                }
                
                // Disconnect button
                let disconnect_response = ui.add_enabled(
                    self.is_connected,
                    egui::Button::new(
                        egui::RichText::new("Disconnect")
                            .size(18.0)
                            .color(if self.is_connected { egui::Color32::WHITE } else { egui::Color32::GRAY })
                    )
                    .fill(if self.is_connected { egui::Color32::from_rgb(200, 100, 100) } else { egui::Color32::from_rgb(180, 180, 180) })
                    .min_size(egui::Vec2::new(120.0, 32.0))
                );
                
                if disconnect_response.clicked() {
                    self.update_status("Disconnecting...".to_string());
                    self.disconnect();
                }
                
                disconnect_response.on_hover_text(if self.is_connected {
                    format!("Disconnect from {}:{}", self.server_address, self.server_port)
                } else {
                    "Not currently connected".to_string()
                });
                
                // Add keyboard shortcut for disconnect
                if ui.input_mut(|i| i.key_pressed(egui::Key::D) && i.modifiers.ctrl) {
                    if self.is_connected {
                        self.update_status("Disconnecting...".to_string());
                        self.disconnect();
                    }
                }
                
                // Save config button
                let save_button = ui.add(
                    egui::Button::new(
                        egui::RichText::new("Save")
                            .size(18.0)
                    )
                    .fill(egui::Color32::from_rgb(150, 150, 200))
                    .min_size(egui::Vec2::new(80.0, 32.0))
                );
                
                if save_button.clicked() {
                    if let Err(e) = self.save_config() {
                        self.update_status(format!("Error initiating config save: {}", e));
                    } else {
                        self.update_status("Saving configuration...".to_string());
                    }
                }
                save_button.on_hover_text("Save current configuration (Ctrl+S)");
                
                // Add keyboard shortcut for save
                if ui.input_mut(|i| i.key_pressed(egui::Key::S) && i.modifiers.ctrl) {
                    if let Err(e) = self.save_config() {
                        self.update_status(format!("Error initiating config save: {}", e));
                    } else {
                        self.update_status("Saving configuration...".to_string());
                    }
                }
            });
            
            // Connection progress indicator with more detail
            if self.connecting {
                ui.add_space(5.0);
                egui::Frame::none()
                    .fill(egui::Color32::from_rgba_premultiplied(255, 255, 0, 25))
                    .rounding(egui::Rounding::same(5.0))
                    .show(ui, |ui| {
                        ui.vertical(|ui| {
                            ui.horizontal(|ui| {
                                ui.spinner(); // Show a spinner animation while connecting
                                ui.label(format!("Connecting to {}:{}...", self.server_address, self.server_port));
                            });
                            
                            // Add connection attempt counter or timeout info
                            ui.label("This may take a few seconds. Press ESC to cancel.");
                            
                            // Add a progress bar
                            let time = ui.input(|i| i.time) as f32;
                            let progress = (time % 3.0) / 3.0; // Create a cycling progress between 0-1
                            ui.add(egui::ProgressBar::new(progress).animate(true));
                        });
                    });
            }
            
            // Status message at the bottom
            ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
                // Add keyboard shortcuts help section
                let help_frame = egui::Frame::none()
                    .fill(egui::Color32::from_rgba_premultiplied(100, 100, 100, 25))
                    .rounding(egui::Rounding::same(8.0))
                    .inner_margin(10.0);
                
                help_frame.show(ui, |ui| {
                    ui.collapsing("⌨ Keyboard Shortcuts", |ui| {
                        ui.label("Tab / Shift+Tab: Navigate between fields");
                        ui.label("Enter: Move to next field");
                        ui.label("Ctrl+Enter: Connect to server");
                        ui.label("Ctrl+S: Save configuration");
                        ui.label("Ctrl+D: Disconnect from server");
                        ui.label("Esc: Cancel connection attempt");
                    });
                    
                    // Add a small vertical space
                    ui.add_space(5.0);
                    
                    // Status indicator legend
                    ui.collapsing("🏁 Status Indicators", |ui| {
                        ui.horizontal(|ui| {
                            ui.colored_label(egui::Color32::GREEN, "Green");
                            ui.label("Connected");
                        });
                        ui.horizontal(|ui| {
                            ui.colored_label(egui::Color32::YELLOW, "Yellow");
                            ui.label("Connecting");
                        });
                        ui.horizontal(|ui| {
                            ui.colored_label(egui::Color32::RED, "Red");
                            ui.label("Disconnected or Error");
                        });
                    });
                });
                
                ui.add_space(5.0);
                
                // Version and links
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 5.0;
                    ui.label("RCP Client v1.0.0 •");
                    ui.hyperlink_to("Help", "https://github.com/open-rcp/rust-rcp-client");
                    ui.label("•");
                    ui.hyperlink_to("Report Bug", "https://github.com/open-rcp/rust-rcp-client/issues");
                });
            });
        });
        
        // Request repaint frequently to update status
        ctx.request_repaint_after(std::time::Duration::from_millis(100));
    }
    
    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        // Send shutdown signal when the application is closing
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(());
        }
    }
}

/// Run the GUI application
pub fn run_gui(config: ClientConfig, auto_connect: bool) -> Result<()> {
    // Create the tokio runtime
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?;
    
    // Store the runtime handle for global access
    let rt_handle = rt.handle().clone();
    
    // Initialize native options (window size, title, etc.)
    let mut options = eframe::NativeOptions::default();
    options.default_theme = if config.ui.dark_mode {
        eframe::Theme::Dark
    } else {
        eframe::Theme::Light
    };
    options.centered = true;
    
    // Set up a channel for shutdown notification
    let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel();
    
    // Clone config for moving into the closure
    let config_clone = config.clone();
    let auto_connect_clone = auto_connect;
    
    // Spawn a thread to run the tokio runtime
    std::thread::spawn(move || {
        rt.block_on(async {
            // Wait for the shutdown signal
            let _ = shutdown_rx.await;
            // Perform any cleanup here
            info!("GUI application shutdown complete");
        });
    });
    
    // Create and run the application
    match eframe::run_native(
        "RCP Client",
        options,
        Box::new(move |cc| {
            // Store the tokio runtime handle in the context
            cc.egui_ctx.set_visuals(if config_clone.ui.dark_mode {
                egui::Visuals::dark()
            } else {
                egui::Visuals::light()
            });
            
            Box::new(RcpClientApp::new(config_clone, auto_connect_clone, rt_handle, shutdown_tx))
        }),
    ) {
        Ok(_) => Ok(()),
        Err(e) => {
            error!("Failed to run GUI: {}", e);
            Err(anyhow::anyhow!("GUI initialization failed: {}", e))
        }
    }
}