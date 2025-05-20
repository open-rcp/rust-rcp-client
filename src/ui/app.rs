use crate::config::ClientConfig;
use crate::protocol;
use anyhow::Result;
use eframe::{self, egui};
use log::{error, info};
use std::sync::Arc;
use tokio::runtime::Handle;
use tokio::sync::{mpsc, oneshot, Mutex};

use super::events::AppEvent;
use super::history::load_connection_history;
use super::models::{AppState, ConnectionEntry};

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
        // The original run_gui took (ClientConfig, bool for auto_connect)
        // The new RcpClientApp::new expects (egui::Context, ClientConfig, Handle, shutdown_tx)
        // This needs to be refactored to align with how RcpClientApp is now initialized and run by eframe
        // For now, let's assume the main.rs or lib.rs will set up eframe and RcpClientApp directly.
        // This App struct might be deprecated or need significant changes.
        // Temporarily, we'll make it a no-op or return an error to indicate it needs updating.
        // run_gui(self.config.clone(), self.config.ui.auto_connect)
        anyhow::bail!("App::run() needs to be refactored to use eframe and RcpClientApp directly.");
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
        shutdown_tx: oneshot::Sender<()>,
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
        let app_state_locked = self.app_state.blocking_lock(); // Lock once
        let is_connected = app_state_locked.is_connected;
        let is_connecting = app_state_locked.connecting;
        let status_message_from_state = app_state_locked.connection_status.clone(); // Assuming this exists
        drop(app_state_locked); // Drop the lock

        let _current_status = self.status.blocking_lock().clone(); // This is the main status string

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("RCP Client");
            ui.separator();

            // Get the state for UI rendering - using a non-blocking approach
            // This was the old way, let's use the locked values from above
            // let app_state_tuple = {
            //     if let Ok(state) = self.app_state.try_lock() {
            //         (state.is_connected, state.connecting, state.connection_status.clone())
            //     } else {
            //         (false, false, "Locked".to_string())
            //     }
            // };

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
            // if app_state_tuple.0 { // Old way
            if is_connected {
                // New way
                super::widgets::connection_panel::draw_connection_panel(
                    ui,
                    &self.server_address,
                    &self.server_port,
                    &self.username,
                    &self.auth_method,
                    self.use_tls,
                    &self.event_tx,
                    &self.rt_handle,
                    &self.app_state, // Pass Arc<Mutex<AppState>> directly
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
            // These handlers (_update_status, _save_config, _connect, _disconnect) are unused as per warnings.
            // They were intended for the old action_panel design. The new design uses event_tx.
            // We can remove them if they are truly not used by any other part of app.rs.
            // For now, prefixing them with underscore if they are defined here.
            let _status_mutex = self.status.clone();
            let _update_status = move |_status: String| {
                // if let Ok(mut status_guard) = status_mutex.try_lock() {
                //     *status_guard = status;
                // }
            };

            let _save_config = || -> Result<()> { Ok(()) };

            let _event_tx_clone = self.event_tx.clone(); // Renamed to avoid conflict if original event_tx is used
            let _connect = || {
                // let tx = event_tx_clone.clone();
                // self.rt_handle.spawn(async move {
                //     let _ = tx.send(AppEvent::Connect).await;
                // });
            };

            let _disconnect = || {
                // let tx = event_tx_clone.clone();
                // self.rt_handle.spawn(async move {
                //     let _ = tx.send(AppEvent::Disconnect).await;
                // });
            };

            // Action Panel
            super::widgets::action_panel::draw_action_panel(
                ui,
                &self.server_address,
                &self.server_port,
                &self.auth_method,
                &mut self.config.ui.auto_connect,
                &mut self.config.ui.auto_reconnect,
                is_connected,               // Use locked value
                is_connecting,              // Use locked value
                &status_message_from_state, // Use status from AppState for action panel
                self.event_tx.clone(),
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

            Box::new(RcpClientApp::new(
                config_clone,
                auto_connect_clone,
                rt_handle,
                shutdown_tx,
            ))
        }),
    ) {
        Ok(_) => Ok(()),
        Err(e) => {
            error!("Failed to run GUI: {}", e);
            Err(anyhow::anyhow!("GUI initialization failed: {}", e))
        }
    }
}
