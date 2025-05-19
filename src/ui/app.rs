use crate::config::ClientConfig;
use crate::protocol;
use anyhow::Result;
use eframe::{self, egui};
use log::{info, error};
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex, oneshot};
use tokio::runtime::Handle;

use super::models::{AppState, ConnectionEntry};
use super::events::AppEvent;
use super::history::load_connection_history;

/// Simple wrapper for the legacy App interface
pub struct App {
    config: ClientConfig,
}

impl App {
    /// Create a new application with the given configuration
    pub fn new(config: ClientConfig) -> Result<Self> {
        Ok(Self { config })
    }

    /// Run the application
    pub async fn run(self) -> Result<()> {
        // Just delegate to the GUI implementation
        run_gui(self.config.clone(), self.config.ui.auto_connect)
    }
}

/// The main RCP Client application
pub struct RcpClientApp {
    /// Application config
    config: ClientConfig,
    /// Auto-connect on startup
    auto_connect: bool,
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
    /// Use TLS for connection
    use_tls: bool,
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
        let (event_tx, _event_rx) = mpsc::channel(32);
        let client = Arc::new(Mutex::new(None));
        let status = Arc::new(Mutex::new("Disconnected".to_string()));
        let app_state = Arc::new(Mutex::new(AppState::new()));
        
        // Load connection history
        let connection_history = load_connection_history();
        
        // Initialize app instance
        Self {
            config: config.clone(),
            auto_connect,
            server_address: config.server.address.clone(),
            server_port: config.server.port.to_string(),
            username: config.auth.username.clone().unwrap_or_default(),
            auth_method: config.auth.method.clone(),
            remember_credentials: config.auth.save_credentials,
            use_tls: config.server.use_tls,
            connection_history,
            event_tx,
            client,
            status,
            app_state,
            rt_handle,
            shutdown_tx: Some(shutdown_tx),
        }
    }

    fn update_ui(&mut self, ctx: &egui::Context) {
        // Draw the main application UI
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("RCP Client");
            ui.separator();
            
            // Get the state for UI rendering - using a non-blocking approach
            let app_state = {
                // Try to get a quick lock, if it fails just provide default values
                if let Ok(state) = self.app_state.try_lock() {
                    (state.is_connected, state.connecting, state.connection_status.clone())
                } else {
                    (false, false, "Locked".to_string())
                }
            };
            
            // Draw server configuration panel
            super::widgets::server_panel::draw_server_panel(
                ui,
                &mut self.server_address,
                &mut self.server_port,
                &mut self.use_tls,
                &self.event_tx,
                &self.rt_handle,
                &self.connection_history,
                &self.app_state,
            );
            
            // Only show the connection panel when connected
            if app_state.0 {
                super::widgets::connection_panel::draw_connection_panel(
                    ui,
                    &self.server_address,
                    &self.server_port,
                    &self.username,
                    &self.auth_method,
                    self.use_tls,
                    &self.event_tx,
                    &self.rt_handle,
                );
            }
            
            // Draw authentication panel
            super::widgets::auth_panel::draw_auth_panel(
                ui,
                &mut self.username,
                &mut self.auth_method,
                &mut self.remember_credentials,
                &self.event_tx,
                &self.rt_handle,
                &self.app_state,
            );
            
            // For action panel, we need to create the handlers
            let status_mutex = self.status.clone();
            let mut update_status = move |status: String| {
                if let Ok(mut status_guard) = status_mutex.try_lock() {
                    *status_guard = status;
                }
            };
            
            let mut save_config = || -> Result<()> {
                // Config saving logic would go here
                Ok(())
            };
            
            let event_tx = self.event_tx.clone();
            let connect = || {
                let tx = event_tx.clone();
                self.rt_handle.spawn(async move {
                    let _ = tx.send(AppEvent::Connect).await;
                });
            };
            
            let disconnect = || {
                let tx = event_tx.clone();
                self.rt_handle.spawn(async move {
                    let _ = tx.send(AppEvent::Disconnect).await;
                });
            };
            
            // Draw action panel
            super::widgets::action_panel::draw_action_panel(
                ui,
                app_state.0, // is_connected
                app_state.1, // connecting
                &self.server_address,
                &self.server_port,
                &self.auth_method,
                &self.event_tx,
                &self.rt_handle,
                &mut save_config,
                &mut update_status,
                &connect,
                &disconnect,
            );
        });
    }
}

impl eframe::App for RcpClientApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.update_ui(ctx);
        
        // Request a repaint at a reasonable framerate
        ctx.request_repaint_after(std::time::Duration::from_millis(100));
    }
    
    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        info!("Application exiting");
        
        // Send shutdown signal
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
    
    // Initialize native options
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
            info!("GUI application shutdown complete");
        });
    });
    
    // Create and run the application
    match eframe::run_native(
        "RCP Client",
        options,
        Box::new(move |cc| {
            // Set the visuals based on theme
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