// filepath: /Volumes/EXT/repos/open-rcp/rust-rcp-client/src/ui/gui.rs
use crate::config::ClientConfig;
use crate::protocol;
use crate::ui::events::AppEvent;
use crate::ui::history::{add_to_connection_history, load_connection_history, save_connection_history}; // Added save_connection_history
use crate::ui::models::{AppState, ConnectionEntry};
use eframe::egui;
use std::sync::Arc;
use tokio::runtime::Handle;
use tokio::sync::{mpsc, oneshot, Mutex};

/// RCP Client GUI Application
pub struct RcpClientApp {
    server_address: String,
    server_port: String,
    auth_method: String,
    username: String,
    password: String, // For UI binding if needed, auth_panel uses AppState.password for its logic
    token: String,    // For UI binding if needed
    psk_identity: String, // For UI binding if needed
    psk_key: String,      // For UI binding if needed
    use_tls: bool,
    remember_credentials: bool,
    auto_connect: bool,
    auto_reconnect: bool,

    status: Arc<Mutex<String>>,
    app_state: Arc<Mutex<AppState>>,
    client: Arc<Mutex<Option<protocol::Client>>>,
    event_tx: mpsc::Sender<AppEvent>,
    event_rx: Option<mpsc::Receiver<AppEvent>>,
    status_message: String,
    rt_handle: Handle,
    shutdown_tx: Option<oneshot::Sender<()>>,
    connection_history: Vec<ConnectionEntry>, // Changed to Vec<ConnectionEntry>
}

impl RcpClientApp {
    pub fn new(
        cc: &eframe::CreationContext<'_>,
        config: ClientConfig, 
        rt_handle: Handle,
        shutdown_tx: oneshot::Sender<()>
    ) -> Self {
        // Configure the egui context with larger text and improved styling
        let ctx = &cc.egui_ctx;
        
        // Increase font size throughout the application
        ctx.set_pixels_per_point(1.3); // Increase UI scale by 30%
        
        let (event_tx_async_to_gui, event_rx_gui) = mpsc::channel(100);
        let (event_tx_gui_to_async, event_rx_async) = mpsc::channel(100);

        let app_state = Arc::new(Mutex::new(AppState {
            is_connected: false,
            connecting: false,
            connection_status: "Disconnected".to_string(),
            password: String::new(),
            show_password: false,
            last_validated_address: None,
            connection_time: None,
        }));

        let status = Arc::new(Mutex::new("Ready".to_string()));
        let client_arc = Arc::new(Mutex::new(None::<protocol::Client>));

        let rt_handle_clone = rt_handle.clone();
        let status_clone = status.clone();
        let app_state_clone = app_state.clone();
        let client_clone = client_arc.clone();
        let event_tx_for_async_logic = event_tx_async_to_gui.clone();
    
        let config_clone_for_async = config.clone(); 
        // Always disable auto-connect on startup
        let auto_connect_for_async = false; // Force disable auto-connect regardless of config

        rt_handle.spawn(async move {
            run_gui_inner(
                config_clone_for_async, 
                auto_connect_for_async,
                event_tx_for_async_logic,
                event_rx_async,
                rt_handle_clone,
                status_clone,
                app_state_clone,
                client_clone,
            )
            .await;
        });
        
        let loaded_history = load_connection_history();

        Self {
            server_address: config.server.address.clone(),
            server_port: config.server.port.to_string(),
            auth_method: config.auth.method.clone(),
            username: config.auth.username.clone().unwrap_or_default(),
            password: String::new(), 
            token: String::new(),
            psk_identity: String::new(),
            psk_key: String::new(),
            use_tls: config.server.use_tls,
            remember_credentials: config.auth.save_credentials,
            auto_connect: config.ui.auto_connect, // Use original config
            auto_reconnect: config.ui.auto_reconnect, // Use original config
            status,
            app_state,
            client: client_arc,
            event_tx: event_tx_gui_to_async, 
            event_rx: Some(event_rx_gui),  
            status_message: "Ready".to_string(),
            rt_handle,
            shutdown_tx: Some(shutdown_tx),
            connection_history: loaded_history, // Assign Vec<ConnectionEntry>
        }
    }

    fn update_ui(&mut self, ctx: &egui::Context) {
        // Use try_lock() to avoid blocking the main thread
        let (is_connected, is_connecting) = if let Ok(app_state_guard) = self.app_state.try_lock() {
            (app_state_guard.is_connected, app_state_guard.connecting)
        } else {
            // If the lock is contended, use the last known values (default to false if unsure)
            (false, false)
        };

        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.heading("Rust RCP Client");
                ui.separator();
                if ui.button("Save Config").clicked() {
                    if let Err(e) = self.event_tx.try_send(AppEvent::SaveConfig) {
                        eprintln!("Failed to send SaveConfig event: {}", e);
                    }
                }
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Client Control Panel");
            ui.add_space(10.0);

            if !is_connected && !is_connecting {
                // Server Panel (8 arguments)
                crate::ui::widgets::server_panel::draw_server_panel(
                    ui,
                    &mut self.server_address,
                    &mut self.server_port,
                    &mut self.use_tls,
                    &self.event_tx, 
                    &self.rt_handle, 
                    &self.connection_history, 
                    &self.app_state, 
                );

                // Auth Panel (7 arguments)
                crate::ui::widgets::auth_panel::draw_auth_panel(
                    ui,
                    &mut self.username,
                    &mut self.auth_method,
                    &mut self.remember_credentials,
                    &self.event_tx,
                    &self.rt_handle,
                    &self.app_state
                );
            }

            // Connection Panel (9 arguments) - This was the one with the argument mismatch error previously at line 159 according to compiler, but it was for server_panel.
            // The definition of draw_connection_panel actually takes 9 arguments.
            // The previous call was: draw_connection_panel(ui, &mut self.auto_connect, &mut self.auto_reconnect, is_connected, is_connecting, &self.status_message, &self.event_tx)
            // This is 7 arguments. It needs server_address, server_port, username, auth_method, use_tls, event_tx, rt_handle, app_state.
            // However, the connection_panel is typically shown *when connected*. The current logic shows it always.
            // For now, let's assume it should be called when connected, similar to action_panel.
            // If it's meant to be shown always, its parameters need to be available always.
            // The existing call had different parameters. Let's adjust the call to match its definition, assuming it's shown when connected.
            if is_connected { // Assuming connection_panel is shown when connected
                crate::ui::widgets::connection_panel::draw_connection_panel(
                    ui,                                 
                    &self.server_address,               
                    &self.server_port,                  
                    &self.username,                     
                    &self.auth_method,                  
                    self.use_tls,                       
                    &self.event_tx,                     
                    &self.rt_handle,                    
                    &self.app_state                     
                );
            } else {
                 crate::ui::widgets::connection_panel::draw_connection_panel_controls(
                    ui,
                    &self.server_address, // Pass current server_address
                    &self.server_port,    // Pass current server_port
                    &mut self.auto_connect,
                    &mut self.auto_reconnect,
                    is_connecting,
                    &self.status_message,
                    &self.event_tx,
                );
            }

            // Action Panel (10 arguments, only if connected)
            if is_connected {
                crate::ui::widgets::action_panel::draw_action_panel(
                    ui,
                    &self.server_address,
                    &self.server_port,
                    &self.auth_method,
                    &mut self.auto_connect, 
                    &mut self.auto_reconnect, 
                    is_connected,
                    is_connecting,
                    &self.status_message,
                    self.event_tx.clone(), 
                );
            }

            ui.add_space(10.0);
            ui.separator();
            ui.label(format!("Status: {}", self.status_message));
            if is_connecting {
                ui.spinner();
            }
        });
    }

    fn handle_event(&mut self, event: AppEvent) {
        match event {
            AppEvent::Connect => {
                println!("GUI: Connect event received, should be handled by async task via channel");
                if let Ok(mut app_state_mg) = self.app_state.try_lock() {
                    app_state_mg.connecting = true;
                    app_state_mg.is_connected = false;
                    app_state_mg.connection_status = "Connecting...".to_string();
                }
                self.status_message = "Connecting...".to_string();
                if let Ok(mut status_mg) = self.status.try_lock() {
                    *status_mg = "Connecting...".to_string();
                }
            }
            AppEvent::Disconnect => {
                println!("GUI: Disconnect event received, should be handled by async task via channel");
                if let Ok(mut app_state_mg) = self.app_state.try_lock() {
                    app_state_mg.connecting = false;
                    app_state_mg.is_connected = false;
                    app_state_mg.connection_status = "Disconnected".to_string();
                }
                self.status_message = "Disconnected".to_string();
                if let Ok(mut status_mg) = self.status.try_lock() {
                    *status_mg = "Disconnected".to_string();
                }
            }
            AppEvent::ConnectionSucceeded => {
                println!("GUI: ConnectionSucceeded event received");
                if let Ok(mut app_state_mg) = self.app_state.try_lock() {
                    app_state_mg.is_connected = true;
                    app_state_mg.connecting = false;
                    app_state_mg.set_connected(true);
                }
                self.status_message = "Connected".to_string();
                if let Ok(mut status_mg) = self.status.try_lock() {
                    *status_mg = "Connected".to_string();
                }

                add_to_connection_history(
                    &mut self.connection_history, 
                    &self.server_address, 
                    &self.server_port, 
                    Some(&self.username), 
                    &self.auth_method,
                    true // successful
                );
                save_connection_history(&self.connection_history); 
            }
            AppEvent::ConnectionFailed(reason) => {
                println!("GUI: ConnectionFailed event received: {}", reason);
                if let Ok(mut app_state_mg) = self.app_state.try_lock() {
                    app_state_mg.is_connected = false;
                    app_state_mg.connecting = false;
                    app_state_mg.connection_status = format!("Failed: {}", reason);
                }
                self.status_message = format!("Failed: {}", reason);
                if let Ok(mut status_mg) = self.status.try_lock() {
                    *status_mg = format!("Failed: {}", reason);
                }

                add_to_connection_history(
                    &mut self.connection_history,
                    &self.server_address,
                    &self.server_port,
                    Some(&self.username),
                    &self.auth_method,
                    false // successful
                );
                save_connection_history(&self.connection_history); 
            }
            AppEvent::DisconnectedConfirmed => { 
                println!("GUI: Confirmed Disconnected event received");
                if let Ok(mut app_state_mg) = self.app_state.try_lock() {
                    app_state_mg.is_connected = false;
                    app_state_mg.connecting = false;
                    app_state_mg.set_connected(false);
                }
                self.status_message = "Disconnected".to_string();
                if let Ok(mut status_mg) = self.status.try_lock() {
                    *status_mg = "Disconnected".to_string();
                }
            }
            AppEvent::SaveConfig => {
                println!("GUI: SaveConfig event received by GUI event handler.");
                self.status_message = "Configuration save requested.".to_string();
                if let Ok(mut status_mg) = self.status.try_lock() {
                    *status_mg = "Configuration save requested.".to_string();
                }
            }
            AppEvent::StatusUpdate(message) => {
                println!("GUI: StatusUpdate event received: {}", message);
                self.status_message = message.clone();
                if let Ok(mut status_mg) = self.status.try_lock() {
                    *status_mg = message;
                }
            }
            AppEvent::ValidateInput(field) => {
                println!("GUI: ValidateInput event received for field: {}. This is unexpected here.", field);
            }
            // Placeholder arms for other AppEvent variants
            AppEvent::AuthenticationSucceeded => {println!("GUI: AuthenticationSucceeded event - not fully handled yet.");}
            AppEvent::AuthenticationFailed(reason) => {println!("GUI: AuthenticationFailed event: {} - not fully handled yet.", reason);}
            AppEvent::ConfigSaved => {println!("GUI: ConfigSaved event - not fully handled yet.");}
            AppEvent::ConfigSaveFailed(reason) => {println!("GUI: ConfigSaveFailed event: {} - not fully handled yet.", reason);}
            AppEvent::UpdateConnectionState(is_connected) => {println!("GUI: UpdateConnectionState event: {} - not fully handled yet.", is_connected);}
            AppEvent::SetConnecting(is_connecting) => {println!("GUI: SetConnecting event: {} - not fully handled yet.", is_connecting);}
            AppEvent::UpdateConnectionHistory(..) => {println!("GUI: UpdateConnectionHistory event - not fully handled yet.");}
            AppEvent::SaveCredentials => {println!("GUI: SaveCredentials event - not fully handled yet.");}
            AppEvent::ClearCredentials => {println!("GUI: ClearCredentials event - not fully handled yet.");}
            _ => { 
                // log::debug!("Unhandled AppEvent in GUI: {:?}", event);
                // Or, if certain events are not expected by the GUI handler directly:
                // println!("GUI: Received an AppEvent that is not directly handled by the GUI's main event loop: {:?}", event);
            }
        }
    }
}

impl eframe::App for RcpClientApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let mut events_to_process = Vec::new();
        if let Some(rx) = &mut self.event_rx {
            // Drain the channel into a temporary Vec
            while let Ok(event) = rx.try_recv() {
                events_to_process.push(event);
            }
        }

        // Process events outside of the borrow of self.event_rx
        for event in events_to_process {
            self.handle_event(event); // This takes &mut self
        }

        // Sync status from shared Arc<Mutex<String>>
        // This part should be fine as it's sequential to handle_event
        if let Ok(status_guard) = self.status.try_lock() {
            if self.status_message != *status_guard {
                self.status_message = status_guard.clone(); // Modifies self.status_message
            }
        }

        self.update_ui(ctx); // This also takes &mut self, sequential, so fine.
    }

    fn save(&mut self, _storage: &mut dyn eframe::Storage) {
        // eframe::set_value(storage, eframe::APP_KEY, self);
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(()).map_err(|e| eprintln!("Failed to send shutdown signal: {:?}", e));
        }
    }
}

async fn run_gui_inner(
    _config: ClientConfig, 
    auto_connect_initial: bool, 
    event_tx_to_gui: mpsc::Sender<AppEvent>, 
    mut event_rx_from_gui: mpsc::Receiver<AppEvent>, 
    _rt_handle: Handle, 
    status_arc: Arc<Mutex<String>>, 
    app_state_arc: Arc<Mutex<AppState>>, 
    _client_arc: Arc<Mutex<Option<protocol::Client>>> 
) {
    let (_shutdown_tx, mut shutdown_rx) = oneshot::channel::<()>();

    // Auto-connect is explicitly disabled, the if-condition will never be true
    // but we keep the code structure for future reference
    if auto_connect_initial {
        {
            // Use async lock here as we are in an async function
            let mut app_state = app_state_arc.lock().await;
            app_state.connecting = true;
        } 
        if let Err(e) = event_tx_to_gui.send(AppEvent::Connect).await {
             eprintln!("run_gui_inner: Failed to send initial Connect event: {}", e);
        }
    }

    loop {
        tokio::select! {
            Some(event) = event_rx_from_gui.recv() => {
                println!("Async task received event: {:?}", event);

                match event {
                    AppEvent::Connect => {
                        println!("Async task: Handling Connect event");
                        app_state_arc.lock().await.connecting = true;
                        status_arc.lock().await.clear();
                        status_arc.lock().await.push_str("Connecting...");
                        
                        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await; // Shorter delay for testing

                        let connected = true; 
                        if connected {
                            let mut app_state_locked = app_state_arc.lock().await;
                            app_state_locked.is_connected = true;
                            app_state_locked.connecting = false;
                            app_state_locked.set_connected(true); // Use method to set time and status string
                            drop(app_state_locked); // Release lock before sending event

                            status_arc.lock().await.clear();
                            status_arc.lock().await.push_str("Connected successfully!");
                            if let Err(e) = event_tx_to_gui.send(AppEvent::ConnectionSucceeded).await {
                                eprintln!("Failed to send ConnectionSucceeded: {}", e);
                            }
                        } else {
                            let mut app_state_locked = app_state_arc.lock().await;
                            app_state_locked.is_connected = false;
                            app_state_locked.connecting = false;
                            app_state_locked.connection_status = "Failed to connect".to_string();
                            drop(app_state_locked); // Release lock

                            status_arc.lock().await.clear();
                            status_arc.lock().await.push_str("Connection failed.");
                            if let Err(e) = event_tx_to_gui.send(AppEvent::ConnectionFailed("Simulated failure".to_string())).await {
                                eprintln!("Failed to send ConnectionFailed: {}", e);
                            }
                        }
                    }
                    AppEvent::Disconnect => {
                        println!("Async task: Handling Disconnect event");
                        let mut app_state_locked = app_state_arc.lock().await;
                        app_state_locked.is_connected = false;
                        app_state_locked.connecting = false;
                        app_state_locked.set_connected(false); // Use method
                        drop(app_state_locked); // Release lock

                        status_arc.lock().await.clear();
                        status_arc.lock().await.push_str("Disconnected.");
                        if let Err(e) = event_tx_to_gui.send(AppEvent::DisconnectedConfirmed).await { // Changed to DisconnectedConfirmed
                             eprintln!("Failed to send DisconnectedConfirmed event: {}", e);
                        }
                    }
                    AppEvent::SaveConfig => {
                        println!("Async task: SaveConfig event received.");
                        status_arc.lock().await.clear();
                        status_arc.lock().await.push_str("Configuration saved (simulated).");
                         if let Err(e) = event_tx_to_gui.send(AppEvent::StatusUpdate("Config saved.".to_string())).await {
                             eprintln!("Failed to send StatusUpdate event: {}", e);
                        }
                    }
                    _ => {}
                }
            }
            _ = &mut shutdown_rx => {
                println!("Async task shutting down");
                break;
            }
        }
    }
}