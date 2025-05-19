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
}

/// RCP Client GUI Application
pub struct RcpClientApp {
    /// Application config
    config: ClientConfig,
    /// Auto-connect on startup
    auto_connect: bool,
    /// Connection status
    connection_status: String,
    /// Server address input
    server_address: String,
    /// Server port input
    server_port: String,
    /// Username input
    username: String,
    /// Authentication method
    auth_method: String,
    /// Message channel
    event_tx: mpsc::Sender<AppEvent>,
    /// Client instance
    client: Arc<Mutex<Option<protocol::Client>>>,
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
        
        // Clone necessary values for the event handler
        let event_tx_clone = event_tx.clone();
        let client_clone = client.clone();
        let config_clone = config.clone();
        
        // Spawn async task to handle events on the runtime
        rt_handle.spawn(async move {
            while let Some(event) = event_rx.recv().await {
                match event {
                    AppEvent::Connect => {
                        let config = config_clone.clone();
                        let tx = event_tx_clone.clone();
                        
                        // Attempt to connect to server
                        tokio::spawn(async move {
                            info!("Connecting to server {}:{}", config.server.address, config.server.port);
                            match protocol::Client::connect(&config.server.address, config.server.port).await {
                                Ok(client) => {
                                    let _ = tx.send(AppEvent::ConnectionSucceeded(client)).await;
                                }
                                Err(e) => {
                                    let _ = tx.send(AppEvent::ConnectionFailed(e.to_string())).await;
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
                            }
                            Ok(false) => {
                                let _ = event_tx_clone.send(AppEvent::AuthenticationFailed("Authentication rejected".to_string())).await;
                            }
                            Err(e) => {
                                let _ = event_tx_clone.send(AppEvent::AuthenticationFailed(e.to_string())).await;
                            }
                        }
                    }
                    AppEvent::ConnectionFailed(error) => {
                        error!("Connection failed: {}", error);
                    }
                    AppEvent::AuthenticationSucceeded => {
                        info!("Authentication succeeded");
                    }
                    AppEvent::AuthenticationFailed(error) => {
                        error!("Authentication failed: {}", error);
                    }
                    AppEvent::Disconnect => {
                        if let Some(_client) = client_clone.lock().await.take() {
                            info!("Disconnecting from server");
                            // Ideal implementation would call client.close() here
                        }
                    }
                }
            }
        });
        
        // Initialize UI state
        let app = Self {
            server_address: config.server.address.clone(),
            server_port: config.server.port.to_string(),
            username: config.auth.username.clone().unwrap_or_default(),
            auth_method: config.auth.method.clone(),
            connection_status: "Disconnected".to_string(),
            config,
            auto_connect,
            event_tx,
            client,
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
        self.rt_handle.spawn(async move {
            let _ = tx.send(AppEvent::Connect).await;
        });
    }
    
    /// Disconnect from server
    fn disconnect(&self) {
        let tx = self.event_tx.clone();
        self.rt_handle.spawn(async move {
            let _ = tx.send(AppEvent::Disconnect).await;
        });
    }
    
    /// Update connection status
    fn update_status(&mut self, status: String) {
        self.connection_status = status;
    }
    
    /// Save current configuration
    fn save_config(&mut self) -> Result<()> {
        // Update config from UI values
        self.config.server.address = self.server_address.clone();
        self.config.server.port = self.server_port.parse().unwrap_or(8717);
        self.config.auth.method = self.auth_method.clone();
        
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
        
        // Save config using the runtime handle
        self.rt_handle.block_on(crate::config::save_config(&config_path, &self.config))?;
        Ok(())
    }
}

impl eframe::App for RcpClientApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("RCP Client");
            
            // Connection status
            ui.horizontal(|ui| {
                ui.label("Status:");
                ui.label(&self.connection_status);
            });
            
            // Server configuration
            ui.group(|ui| {
                ui.heading("Server Configuration");
                ui.horizontal(|ui| {
                    ui.label("Address:");
                    ui.text_edit_singleline(&mut self.server_address);
                });
                ui.horizontal(|ui| {
                    ui.label("Port:");
                    ui.text_edit_singleline(&mut self.server_port);
                });
            });
            
            // Authentication configuration
            ui.group(|ui| {
                ui.heading("Authentication");
                ui.horizontal(|ui| {
                    ui.label("Username:");
                    ui.text_edit_singleline(&mut self.username);
                });
                ui.horizontal(|ui| {
                    ui.label("Method:");
                    egui::ComboBox::from_label("")
                        .selected_text(&self.auth_method)
                        .show_ui(ui, |ui| {
                            ui.selectable_value(&mut self.auth_method, "password".to_string(), "Password");
                            ui.selectable_value(&mut self.auth_method, "psk".to_string(), "Pre-Shared Key");
                            ui.selectable_value(&mut self.auth_method, "native".to_string(), "Native OS");
                        });
                });
            });
            
            // Buttons
            ui.horizontal(|ui| {
                if ui.button("Connect").clicked() {
                    // First save the config
                    if let Err(e) = self.save_config() {
                        self.update_status(format!("Error saving config: {}", e));
                    } else {
                        self.update_status("Connecting...".to_string());
                        self.connect();
                    }
                }
                
                if ui.button("Disconnect").clicked() {
                    self.update_status("Disconnecting...".to_string());
                    self.disconnect();
                }
                
                if ui.button("Save Config").clicked() {
                    if let Err(e) = self.save_config() {
                        self.update_status(format!("Error saving config: {}", e));
                    } else {
                        self.update_status("Configuration saved".to_string());
                    }
                }
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